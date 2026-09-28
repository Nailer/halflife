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
use anchor_lang::solana_program::sysvar::instructions::{
    load_instruction_at_checked, ID as INSTRUCTIONS_ID,
};

/// The Ed25519 sigverify precompile. Declared here rather than imported so the
/// check does not move if the solana-program module layout changes.
pub const ED25519_PROGRAM_ID: Pubkey = pubkey!("Ed25519SigVerify111111111111111111111111111");

declare_id!("HLFpass1111111111111111111111111111111111111");

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

// ---------------------------------------------------------------------------

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
}
