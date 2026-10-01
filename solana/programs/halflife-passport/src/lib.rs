//! Halflife passport registry and enforcement on Solana.
//!
//! # Why Solana, concretely
//!
//! Passport accounts are **read-only** in the consumer path. Sealevel serialises
//! only on writable accounts, so any number of programs can read the same
//! passport in the same slot without contending — only Halflife writes, and only
//! when a status changes. That is the access pattern this chain's account model
//! exists to serve, and it is why an enforcement check can sit inside a hot path
//! rather than beside it. The measured cost is in `benches/`; if that number
//! does not hold up, the argument does not either.
//!
//! # What is verified here
//!
//! Publication is permissionless. Anyone may submit a passport; the program
//! verifies the issuer's ed25519 signature over the canonical 125-byte core via
//! the Ed25519 sigverify precompile. Issuers therefore never need SOL, never
//! sign a Solana transaction, and can be offline or threshold-controlled.
//!
//! The same bytes signed off-chain are verified here unchanged — one
//! serialization, no re-encoding, no second format to keep in step.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::AccountMeta;
use anchor_lang::solana_program::sysvar::instructions::{
    load_instruction_at_checked, ID as INSTRUCTIONS_ID,
};

/// The Ed25519 sigverify precompile. Declared here rather than imported so the
/// check does not move if the solana-program module layout changes.
pub const ED25519_PROGRAM_ID: Pubkey = pubkey!("Ed25519SigVerify111111111111111111111111111");

/// SPL Noop, required by Hyperlane's mailbox for message logging.
pub const SPL_NOOP_PROGRAM_ID: Pubkey = pubkey!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

/// Borsh discriminant of `Instruction::OutboxDispatch` in Hyperlane's mailbox
/// instruction enum: Init(0), InboxProcess(1), InboxSetDefaultIsm(2),
/// InboxGetRecipientIsm(3), OutboxDispatch(4).
pub const HYPERLANE_OUTBOX_DISPATCH: u8 = 4;

/// Seeds for the PDA that signs a dispatch on this program's behalf.
///
/// Hyperlane requires this and does not infer it: the mailbox checks that the
/// signer is exactly `find_program_address(dispatch_authority_seeds, sender)`,
/// which is what stops any program from dispatching as any sender it likes. It
/// is also what makes the destination's `originSender` check meaningful — only
/// this program can speak as this program.
pub const DISPATCH_AUTHORITY_SEEDS: &[&[u8]] =
    &[b"hyperlane_dispatcher", b"-", b"dispatch_authority"];

/// Seeds for Hyperlane's outbox PDA, owned by the mailbox program.
pub const MAILBOX_OUTBOX_SEEDS: &[&[u8]] = &[b"hyperlane", b"-", b"outbox"];

declare_id!("CkDhRfJRiGEa3kgnEUEvCBgyht62MTkDD6e754DLtB2");

/// Must match `halflife_core::SIGNING_DOMAIN`.
pub const SIGNING_DOMAIN: &[u8] = b"halflife-passport-v1";
/// Must match `halflife_core::PASSPORT_CORE_LEN`.
pub const CORE_LEN: usize = 125;
/// Must match `halflife_core::MAX_CLOCK_SKEW_SECS`.
pub const MAX_CLOCK_SKEW_SECS: i64 = 300;

pub const STATUS_VALID: u8 = 1;
pub const STATUS_INVALID: u8 = 2;

#[program]
pub mod halflife_passport {
    use super::*;

    /// Register an issuer. The authority may later revoke it.
    pub fn register_issuer(ctx: Context<RegisterIssuer>, key: [u8; 32]) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let rec = &mut ctx.accounts.issuer;
        rec.key = key;
        rec.authority = ctx.accounts.authority.key();
        rec.registered_at = now;
        rec.revoked_at = 0;
        rec.bump = ctx.bumps.issuer;
        emit!(IssuerRegistered { key, at: now });
        Ok(())
    }

    /// Revoke an issuer. Forward-only: passports it signed stay verifiable, but
    /// consumers stop accepting them from now on.
    pub fn revoke_issuer(ctx: Context<RevokeIssuer>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let rec = &mut ctx.accounts.issuer;
        require!(rec.revoked_at == 0, HalflifeError::AlreadyRevoked);
        rec.revoked_at = now;
        emit!(IssuerRevoked { key: rec.key, at: now });
        Ok(())
    }

    /// Publish a passport, verifying the issuer's off-chain signature.
    ///
    /// The transaction must contain an Ed25519 sigverify instruction covering
    /// `SIGNING_DOMAIN || core`, signed by the issuer named in the core.
    pub fn publish(ctx: Context<Publish>, core: [u8; CORE_LEN]) -> Result<()> {
        let parsed = PassportCore::decode(&core)?;

        // The core names its own issuer; the account must be that issuer's
        // record, or a valid passport could be filed under someone else's PDA.
        require!(
            ctx.accounts.issuer.key == parsed.issuer,
            HalflifeError::IssuerMismatch
        );
        require!(
            ctx.accounts.issuer.revoked_at == 0,
            HalflifeError::IssuerRevoked
        );

        let mut message = Vec::with_capacity(SIGNING_DOMAIN.len() + CORE_LEN);
        message.extend_from_slice(SIGNING_DOMAIN);
        message.extend_from_slice(&core);
        verify_ed25519(&ctx.accounts.instructions, &parsed.issuer, &message)?;

        let now = Clock::get()?.unix_timestamp;
        require!(
            parsed.issued_at <= now.saturating_add(MAX_CLOCK_SKEW_SECS),
            HalflifeError::IssuedInFuture
        );
        require!(
            parsed.expires_at > parsed.issued_at,
            HalflifeError::ExpiryBeforeIssuance
        );
        require!(
            parsed.issued_at >= ctx.accounts.issuer.registered_at,
            HalflifeError::IssuedBeforeRegistration
        );

        let passport = &mut ctx.accounts.passport;
        // Monotonic per (circuit, issuer). A superseded passport replayed here
        // -- including one relayed from a chain that has not seen the newer
        // one -- is refused on sequence, not on timestamp.
        if passport.sequence != 0 || passport.issued_at != 0 {
            require!(
                parsed.sequence > passport.sequence,
                HalflifeError::SequenceNotIncreasing
            );
            require!(
                passport.circuit_hash == parsed.circuit_hash,
                HalflifeError::CircuitMismatch
            );
        }

        passport.circuit_hash = parsed.circuit_hash;
        passport.issuer = parsed.issuer;
        passport.sequence = parsed.sequence;
        passport.capability = parsed.capability;
        passport.status = parsed.status;
        passport.issued_at = parsed.issued_at;
        passport.expires_at = parsed.expires_at;
        passport.evidence_hash = parsed.evidence_hash;
        passport.advisory_count = parsed.advisory_count;
        passport.bump = ctx.bumps.passport;

        emit!(PassportPublished {
            circuit_hash: parsed.circuit_hash,
            issuer: parsed.issuer,
            sequence: parsed.sequence,
            status: parsed.status,
            expires_at: parsed.expires_at,
        });
        Ok(())
    }

    /// Configure where passport state is propagated for one destination domain.
    pub fn set_route(
        ctx: Context<SetRoute>,
        destination_domain: u32,
        recipient: [u8; 32],
        mailbox: Pubkey,
    ) -> Result<()> {
        let route = &mut ctx.accounts.route;
        route.destination_domain = destination_domain;
        route.recipient = recipient;
        route.mailbox = mailbox;
        route.authority = ctx.accounts.authority.key();
        route.bump = ctx.bumps.route;
        emit!(RouteSet {
            destination_domain,
            recipient,
            mailbox
        });
        Ok(())
    }

    /// Propagate a passport to a destination chain over Hyperlane.
    ///
    /// The message body is the canonical 125 bytes, unchanged — the same
    /// serialization that was signed off-chain, stored here, and decoded by the
    /// destination. One format across three runtimes.
    ///
    /// Note what is *not* re-derived: the passport is read from this program's
    /// own account, so a caller cannot propagate a passport the registry never
    /// accepted. Anyone may pay to relay; nobody may invent what is relayed.
    pub fn dispatch(ctx: Context<Dispatch>) -> Result<()> {
        let p = &ctx.accounts.passport;
        let route = &ctx.accounts.route;

        require_keys_eq!(
            ctx.accounts.mailbox_program.key(),
            route.mailbox,
            HalflifeError::MailboxMismatch
        );

        let body = p.canonical_bytes();

        // Borsh: variant tag, then sender, destination_domain, recipient, and a
        // length-prefixed body. Encoded by hand so this program carries no
        // dependency on the Hyperlane crates, which pin an incompatible
        // solana-program major version.
        let mut data = Vec::with_capacity(1 + 32 + 4 + 32 + 4 + body.len());
        data.push(HYPERLANE_OUTBOX_DISPATCH);
        data.extend_from_slice(crate::ID.as_ref());
        data.extend_from_slice(&route.destination_domain.to_le_bytes());
        data.extend_from_slice(&route.recipient);
        data.extend_from_slice(&(body.len() as u32).to_le_bytes());
        data.extend_from_slice(&body);

        let accounts = vec![
            AccountMeta::new(ctx.accounts.outbox.key(), false),
            AccountMeta::new_readonly(ctx.accounts.dispatch_authority.key(), true),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
            AccountMeta::new_readonly(SPL_NOOP_PROGRAM_ID, false),
            AccountMeta::new(ctx.accounts.payer.key(), true),
            AccountMeta::new_readonly(ctx.accounts.unique_message.key(), true),
            AccountMeta::new(ctx.accounts.dispatched_message.key(), false),
        ];

        let bump = ctx.bumps.dispatch_authority;
        let seeds: &[&[u8]] = &[
            DISPATCH_AUTHORITY_SEEDS[0],
            DISPATCH_AUTHORITY_SEEDS[1],
            DISPATCH_AUTHORITY_SEEDS[2],
            &[bump],
        ];

        anchor_lang::solana_program::program::invoke_signed(
            &anchor_lang::solana_program::instruction::Instruction {
                program_id: route.mailbox,
                accounts,
                data,
            },
            &[
                ctx.accounts.outbox.to_account_info(),
                ctx.accounts.dispatch_authority.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.spl_noop.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.unique_message.to_account_info(),
                ctx.accounts.dispatched_message.to_account_info(),
            ],
            &[seeds],
        )?;

        emit!(PassportDispatched {
            circuit_hash: p.circuit_hash,
            issuer: p.issuer,
            sequence: p.sequence,
            destination_domain: route.destination_domain,
        });
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Canonical core
// ---------------------------------------------------------------------------

/// The 125-byte layout, decoded. Field order and widths match
/// `docs/passport-spec.md` exactly; this is the same preimage that was signed.
pub struct PassportCore {
    pub version: u8,
    pub circuit_hash: [u8; 32],
    pub issuer: [u8; 32],
    pub sequence: u64,
    pub capability: u8,
    pub status: u8,
    pub issued_at: i64,
    pub expires_at: i64,
    pub evidence_hash: [u8; 32],
    pub advisory_count: u16,
}

impl PassportCore {
    pub fn decode(b: &[u8; CORE_LEN]) -> Result<Self> {
        let version = b[0];
        require!(version == 1, HalflifeError::UnsupportedVersion);
        let status = b[74];
        require!(
            status == STATUS_VALID || status == STATUS_INVALID,
            HalflifeError::UnknownStatus
        );
        require!(b[73] <= 5, HalflifeError::UnknownCapability);

        Ok(Self {
            version,
            circuit_hash: b[1..33].try_into().unwrap(),
            issuer: b[33..65].try_into().unwrap(),
            sequence: u64::from_le_bytes(b[65..73].try_into().unwrap()),
            capability: b[73],
            status,
            issued_at: i64::from_le_bytes(b[75..83].try_into().unwrap()),
            expires_at: i64::from_le_bytes(b[83..91].try_into().unwrap()),
            evidence_hash: b[91..123].try_into().unwrap(),
            advisory_count: u16::from_le_bytes(b[123..125].try_into().unwrap()),
        })
    }
}

// ---------------------------------------------------------------------------
// Ed25519 verification via the sigverify precompile
// ---------------------------------------------------------------------------

/// Offsets within a single-signature Ed25519 precompile instruction.
const ED25519_PUBKEY_OFFSET: usize = 16;
const ED25519_SIGNATURE_OFFSET: usize = 48;
const ED25519_MESSAGE_OFFSET: usize = 112;

/// Confirm the transaction carries an Ed25519 sigverify instruction over
/// exactly this message, by exactly this key.
///
/// The precompile has already checked the signature by the time the runtime
/// reaches us — a bad signature aborts the whole transaction. What remains is
/// to confirm it covered what we think it covered, which is the step that is
/// easy to get wrong and fatal to skip.
fn verify_ed25519(instructions: &AccountInfo, pubkey: &[u8; 32], message: &[u8]) -> Result<()> {
    let ix = load_instruction_at_checked(0, instructions)
        .map_err(|_| error!(HalflifeError::MissingSignatureInstruction))?;

    require!(
        ix.program_id == ED25519_PROGRAM_ID,
        HalflifeError::MissingSignatureInstruction
    );

    let data = &ix.data;
    require!(
        data.len() >= ED25519_MESSAGE_OFFSET + message.len(),
        HalflifeError::MalformedSignatureInstruction
    );
    // Exactly one signature. Multiple would let an attacker append a valid
    // signature over unrelated data and have us inspect the wrong one.
    require!(data[0] == 1, HalflifeError::MalformedSignatureInstruction);

    require!(
        &data[ED25519_PUBKEY_OFFSET..ED25519_PUBKEY_OFFSET + 32] == pubkey.as_slice(),
        HalflifeError::SignerMismatch
    );
    require!(
        &data[ED25519_MESSAGE_OFFSET..ED25519_MESSAGE_OFFSET + message.len()] == message,
        HalflifeError::SignedMessageMismatch
    );
    // Trailing bytes would mean the signature covered more than we checked.
    require!(
        data.len() == ED25519_MESSAGE_OFFSET + message.len(),
        HalflifeError::SignedMessageMismatch
    );
    let _ = ED25519_SIGNATURE_OFFSET;
    Ok(())
}

// ---------------------------------------------------------------------------
// Accounts
// ---------------------------------------------------------------------------

#[account]
pub struct IssuerRecord {
    pub key: [u8; 32],
    pub authority: Pubkey,
    pub registered_at: i64,
    /// Zero while active.
    pub revoked_at: i64,
    pub bump: u8,
}

impl IssuerRecord {
    pub const LEN: usize = 8 + 32 + 32 + 8 + 8 + 1;
}

#[account]
pub struct Passport {
    pub circuit_hash: [u8; 32],
    pub issuer: [u8; 32],
    pub sequence: u64,
    pub capability: u8,
    pub status: u8,
    pub issued_at: i64,
    pub expires_at: i64,
    pub evidence_hash: [u8; 32],
    pub advisory_count: u16,
    pub bump: u8,
}

impl Passport {
    pub const LEN: usize = 8 + CORE_LEN + 1;

    /// Re-encode to the canonical 125 bytes.
    ///
    /// The destination receives exactly what was signed, so the bytes must be
    /// rebuilt in the same layout rather than re-serialized in some convenient
    /// local form. Field order and widths follow `docs/passport-spec.md`.
    pub fn canonical_bytes(&self) -> [u8; CORE_LEN] {
        let mut out = [0u8; CORE_LEN];
        out[0] = 1;
        out[1..33].copy_from_slice(&self.circuit_hash);
        out[33..65].copy_from_slice(&self.issuer);
        out[65..73].copy_from_slice(&self.sequence.to_le_bytes());
        out[73] = self.capability;
        out[74] = self.status;
        out[75..83].copy_from_slice(&self.issued_at.to_le_bytes());
        out[83..91].copy_from_slice(&self.expires_at.to_le_bytes());
        out[91..123].copy_from_slice(&self.evidence_hash);
        out[123..125].copy_from_slice(&self.advisory_count.to_le_bytes());
        out
    }

    /// Resolve the status a consumer should act on.
    ///
    /// `INVALID` outranks the clock; an expired `VALID` degrades to stale and
    /// blocks. A consumer that stops receiving updates therefore blocks, so
    /// suppressing delivery cannot hold a circuit open.
    pub fn effective(&self, now: i64) -> Effective {
        if self.status == STATUS_INVALID {
            Effective::Invalid
        } else if now >= self.expires_at {
            Effective::Stale
        } else {
            Effective::Valid
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Effective {
    Valid,
    Stale,
    Invalid,
}

#[derive(Accounts)]
#[instruction(key: [u8; 32])]
pub struct RegisterIssuer<'info> {
    #[account(
        init,
        payer = authority,
        space = IssuerRecord::LEN,
        seeds = [b"issuer", key.as_ref()],
        bump
    )]
    pub issuer: Account<'info, IssuerRecord>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RevokeIssuer<'info> {
    #[account(
        mut,
        seeds = [b"issuer", issuer.key.as_ref()],
        bump = issuer.bump,
        has_one = authority
    )]
    pub issuer: Account<'info, IssuerRecord>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(core: [u8; CORE_LEN])]
pub struct Publish<'info> {
    #[account(
        init_if_needed,
        payer = payer,
        space = Passport::LEN,
        seeds = [b"passport", &core[1..33], &core[33..65]],
        bump
    )]
    pub passport: Account<'info, Passport>,
    #[account(
        seeds = [b"issuer", &core[33..65]],
        bump = issuer.bump
    )]
    pub issuer: Account<'info, IssuerRecord>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: address-constrained to the instructions sysvar, read only.
    #[account(address = INSTRUCTIONS_ID)]
    pub instructions: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct DispatchRoute {
    pub destination_domain: u32,
    /// The destination registry, left-padded to 32 bytes as Hyperlane expects.
    pub recipient: [u8; 32],
    /// The Hyperlane mailbox on this chain.
    pub mailbox: Pubkey,
    pub authority: Pubkey,
    pub bump: u8,
}

impl DispatchRoute {
    pub const LEN: usize = 8 + 4 + 32 + 32 + 32 + 1;
}

#[derive(Accounts)]
#[instruction(destination_domain: u32)]
pub struct SetRoute<'info> {
    #[account(
        init_if_needed,
        payer = authority,
        space = DispatchRoute::LEN,
        seeds = [b"route", destination_domain.to_le_bytes().as_ref()],
        bump
    )]
    pub route: Account<'info, DispatchRoute>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Dispatch<'info> {
    /// Read-only: the passport being propagated must already exist in this
    /// registry. A caller cannot relay a passport that was never accepted.
    pub passport: Account<'info, Passport>,
    #[account(
        seeds = [b"route", route.destination_domain.to_le_bytes().as_ref()],
        bump = route.bump
    )]
    pub route: Account<'info, DispatchRoute>,
    /// CHECK: Hyperlane's outbox PDA, validated by the mailbox itself.
    #[account(mut)]
    pub outbox: UncheckedAccount<'info>,
    /// The PDA Hyperlane requires this program to sign with. Deriving it here
    /// and signing via `invoke_signed` is what proves the sender is us.
    /// CHECK: seeds-constrained, never read or written.
    #[account(seeds = [b"hyperlane_dispatcher", b"-", b"dispatch_authority"], bump)]
    pub dispatch_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// A fresh keypair per message; Hyperlane derives the dispatched-message
    /// PDA from it to enforce uniqueness.
    pub unique_message: Signer<'info>,
    /// CHECK: PDA derived by the mailbox from `unique_message`.
    #[account(mut)]
    pub dispatched_message: UncheckedAccount<'info>,
    /// CHECK: checked against `route.mailbox` before the CPI.
    pub mailbox_program: UncheckedAccount<'info>,
    /// CHECK: address-constrained to SPL Noop.
    #[account(address = SPL_NOOP_PROGRAM_ID)]
    pub spl_noop: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

// ---------------------------------------------------------------------------

#[event]
pub struct RouteSet {
    pub destination_domain: u32,
    pub recipient: [u8; 32],
    pub mailbox: Pubkey,
}

#[event]
pub struct PassportDispatched {
    pub circuit_hash: [u8; 32],
    pub issuer: [u8; 32],
    pub sequence: u64,
    pub destination_domain: u32,
}

#[event]
pub struct IssuerRegistered {
    pub key: [u8; 32],
    pub at: i64,
}

#[event]
pub struct IssuerRevoked {
    pub key: [u8; 32],
    pub at: i64,
}

#[event]
pub struct PassportPublished {
    pub circuit_hash: [u8; 32],
    pub issuer: [u8; 32],
    pub sequence: u64,
    pub status: u8,
    pub expires_at: i64,
}

#[error_code]
pub enum HalflifeError {
    #[msg("passport version is not supported")]
    UnsupportedVersion,
    #[msg("status byte is not VALID or INVALID")]
    UnknownStatus,
    #[msg("capability tier is out of range")]
    UnknownCapability,
    #[msg("the core names a different issuer than the account provided")]
    IssuerMismatch,
    #[msg("issuer has been revoked")]
    IssuerRevoked,
    #[msg("issuer is already revoked")]
    AlreadyRevoked,
    #[msg("transaction carries no ed25519 sigverify instruction at index 0")]
    MissingSignatureInstruction,
    #[msg("ed25519 instruction is malformed or carries more than one signature")]
    MalformedSignatureInstruction,
    #[msg("ed25519 instruction was signed by a different key")]
    SignerMismatch,
    #[msg("ed25519 instruction covered a different message")]
    SignedMessageMismatch,
    #[msg("passport was issued beyond the accepted clock skew")]
    IssuedInFuture,
    #[msg("expiry is not after issuance")]
    ExpiryBeforeIssuance,
    #[msg("passport predates the issuer's registration")]
    IssuedBeforeRegistration,
    #[msg("sequence must increase; this passport is superseded")]
    SequenceNotIncreasing,
    #[msg("circuit hash does not match the existing passport account")]
    CircuitMismatch,
    #[msg("supplied mailbox program does not match the configured route")]
    MailboxMismatch,
}
