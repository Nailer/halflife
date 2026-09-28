//! Reference consumer — a program that actually refuses to proceed.
//!
//! This is the point of the whole system: not a dashboard that displays a
//! status, but a third-party program that will not do its work when the
//! cryptography it depends on has no current security evidence.
//!
//! # The access pattern being demonstrated
//!
//! The passport arrives as a **read-only** account. Sealevel serialises only on
//! writable accounts, so any number of programs can perform this check in the
//! same slot without contending on shared state. That is what makes an
//! enforcement check affordable inside a hot path, and it is the concrete
//! reason this layer is on Solana.
//!
//! # Absence is not permission
//!
//! `submit_proof` requires the passport account. There is no branch where a
//! missing passport means *proceed* — Anchor's account resolution fails first,
//! and the transaction aborts. The unsafe integration (*check the passport if
//! one exists*) cannot be expressed here.

use anchor_lang::prelude::*;
use halflife_passport::{Effective, Passport};

declare_id!("BAcrrJYj5Y5DfcqnHgDwm5rhJUJdowZh25NvFLJAUzSW");

#[program]
pub mod halflife_consumer {
    use super::*;

    /// Do the work, but only behind a current passport.
    ///
    /// `expected_circuit` and `min_capability` are the consumer's own policy.
    /// Halflife publishes state; it never decides what this program requires.
    pub fn submit_proof(
        ctx: Context<SubmitProof>,
        expected_circuit: [u8; 32],
        min_capability: u8,
    ) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;

        match check(&ctx.accounts.passport, &expected_circuit, min_capability, now) {
            Ok(()) => {
                msg!("passport current — proceeding");
                Ok(())
            }
            Err(e) => {
                msg!("blocked: passport is not acceptable");
                Err(e)
            }
        }
    }

    /// The check with nothing around it, for measurement.
    ///
    /// Cost is taken from the runtime's own `consumed N of M compute units` log
    /// rather than bracketed in-program. That deliberately includes account
    /// deserialization and instruction dispatch, because those are what a
    /// consumer actually pays. Bracketing only the comparisons would produce a
    /// smaller, flattering number that nobody could reproduce in practice.
    pub fn bench_check(
        ctx: Context<SubmitProof>,
        expected_circuit: [u8; 32],
        min_capability: u8,
    ) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let _ = check(&ctx.accounts.passport, &expected_circuit, min_capability, now);
        Ok(())
    }
}

/// The whole enforcement decision. Four comparisons and a clock read.
#[inline(never)]
fn check(
    passport: &Passport,
    expected_circuit: &[u8; 32],
    min_capability: u8,
    now: i64,
) -> Result<()> {
    require!(
        passport.circuit_hash == *expected_circuit,
        ConsumerError::WrongCircuit
    );
    require!(
        passport.capability >= min_capability,
        ConsumerError::InsufficientCapability
    );
    match passport.effective(now) {
        Effective::Valid => Ok(()),
        // Distinct errors on purpose. Stale is not a finding: it means the
        // evidence is no longer current, not that anything was discovered.
        // Collapsing the two would make every delivery hiccup look like a
        // vulnerability, and every vulnerability look routine.
        Effective::Stale => err!(ConsumerError::PassportStale),
        Effective::Invalid => err!(ConsumerError::PassportInvalid),
    }
}

#[derive(Accounts)]
pub struct SubmitProof<'info> {
    /// Read-only: no write lock, so concurrent consumers do not contend.
    pub passport: Account<'info, Passport>,
    pub caller: Signer<'info>,
}

#[error_code]
pub enum ConsumerError {
    #[msg("passport describes a different circuit than this program expects")]
    WrongCircuit,
    #[msg("passport asserts a lower capability tier than this program requires")]
    InsufficientCapability,
    #[msg("security evidence is no longer current; this is not a finding")]
    PassportStale,
    #[msg("an accepted issuer reported this circuit should no longer be trusted")]
    PassportInvalid,
}
