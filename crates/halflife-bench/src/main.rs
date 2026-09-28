//! Measured compute cost of a passport check on Solana.
//!
//! This binary exists to answer one question with a number rather than an
//! argument: **can a program afford to check a passport inside its hot path?**
//!
//! Everything here is measured, not modelled. The programs are the real SBF
//! artifacts from `anchor build`, the transactions are real, and the compute
//! figures come from the runtime. The passport bytes are produced by
//! `halflife-core` — the same encoder the CLI uses — so a pass also proves the
//! on-chain decoder and the off-chain encoder agree on all 125 bytes.
//!
//! Run: `cargo run --release -p halflife-bench`

use anyhow::{anyhow, Context, Result};
use ed25519_dalek::{Signer, SigningKey};
use halflife_core::{Capability, PassportCore, Status, PASSPORT_VERSION, SIGNING_DOMAIN};
use litesvm::LiteSVM;
use sha2::{Digest, Sha256};
use solana_sdk::{
    ed25519_program,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer as SolSigner},
    system_program,
    sysvar::instructions as instructions_sysvar,
    transaction::Transaction,
};
use std::path::Path;
use std::str::FromStr;

const PASSPORT_ID: &str = "CkDhRfJRiGEa3kgnEUEvCBgyht62MTkDD6e754DLtB2";
const CONSUMER_ID: &str = "BAcrrJYj5Y5DfcqnHgDwm5rhJUJdowZh25NvFLJAUzSW";

/// Anchor numbers custom errors from 6000 in declaration order. Resolving them
/// here turns `Custom(6013)` into evidence a reader can check.
fn error_name(raw: &str) -> String {
    const PASSPORT: [&str; 14] = [
        "UnsupportedVersion", "UnknownStatus", "UnknownCapability", "IssuerMismatch",
        "IssuerRevoked", "AlreadyRevoked", "MissingSignatureInstruction",
        "MalformedSignatureInstruction", "SignerMismatch", "SignedMessageMismatch",
        "IssuedInFuture", "ExpiryBeforeIssuance", "IssuedBeforeRegistration",
        "SequenceNotIncreasing",
    ];
    const CONSUMER: [&str; 4] = [
        "WrongCircuit", "InsufficientCapability", "PassportStale", "PassportInvalid",
    ];
    let Some(rest) = raw.split("Custom(").nth(1) else { return raw.into() };
    let Some(code) = rest.split(')').next().and_then(|c| c.parse::<usize>().ok()) else {
        return raw.into();
    };
    let idx = code.saturating_sub(6000);
    // Instruction index 1 is the passport program; 0 is the consumer.
    let table: &[&str] = if raw.contains("InstructionError(1") { &PASSPORT } else { &CONSUMER };
    table.get(idx).map(|n| (*n).to_string()).unwrap_or_else(|| raw.into())
}

/// Anchor dispatches on the first eight bytes of `sha256("global:<name>")`.
fn discriminator(name: &str) -> [u8; 8] {
    let mut h = Sha256::new();
    h.update(format!("global:{name}").as_bytes());
    h.finalize()[..8].try_into().unwrap()
}

/// Build a single-signature Ed25519 precompile instruction by hand.
///
/// Constructed explicitly rather than via a helper so the byte offsets match
/// the ones the on-chain verifier checks, and so a change on either side shows
/// up as a test failure instead of a silent mismatch.
fn ed25519_instruction(key: &SigningKey, message: &[u8]) -> Instruction {
    const PUBKEY_OFFSET: u16 = 16;
    const SIGNATURE_OFFSET: u16 = 48;
    const MESSAGE_OFFSET: u16 = 112;
    const CURRENT_IX: u16 = u16::MAX;

    let signature = key.sign(message).to_bytes();
    let pubkey = key.verifying_key().to_bytes();

    let mut data = Vec::with_capacity(MESSAGE_OFFSET as usize + message.len());
    data.push(1); // exactly one signature
    data.push(0); // padding
    for v in [
        SIGNATURE_OFFSET,
        CURRENT_IX,
        PUBKEY_OFFSET,
        CURRENT_IX,
        MESSAGE_OFFSET,
        message.len() as u16,
        CURRENT_IX,
    ] {
        data.extend_from_slice(&v.to_le_bytes());
    }
    data.extend_from_slice(&pubkey);
    data.extend_from_slice(&signature);
    data.extend_from_slice(message);

    Instruction {
        program_id: ed25519_program::ID,
        accounts: vec![],
        data,
    }
}

struct Bench {
    svm: LiteSVM,
    payer: Keypair,
    passport_program: Pubkey,
    consumer_program: Pubkey,
}

impl Bench {
    fn new(deploy_dir: &Path) -> Result<Self> {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), 100_000_000_000)
            .map_err(|e| anyhow!("airdrop failed: {e:?}"))?;

        let passport_program = Pubkey::from_str(PASSPORT_ID)?;
        let consumer_program = Pubkey::from_str(CONSUMER_ID)?;

        for (id, file) in [
            (passport_program, "halflife_passport.so"),
            (consumer_program, "halflife_consumer.so"),
        ] {
            let path = deploy_dir.join(file);
            let bytes = std::fs::read(&path).with_context(|| {
                format!("{} not found — run `cd solana && anchor build`", path.display())
            })?;
            svm.add_program(id, &bytes);
        }

        Ok(Self {
            svm,
            payer,
            passport_program,
            consumer_program,
        })
    }

    fn send(&mut self, ixs: Vec<Instruction>, label: &str) -> Result<u64> {
        let blockhash = self.svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &ixs,
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        );
        match self.svm.send_transaction(tx) {
            Ok(r) => Ok(r.compute_units_consumed),
            Err(e) => Err(anyhow!("{label} failed: {:?}\n{:#?}", e.err, e.meta.logs)),
        }
    }

    /// Same failure path, but expected — returns the cost anyway.
    fn send_expecting_failure(&mut self, ixs: Vec<Instruction>) -> Result<(u64, String)> {
        let blockhash = self.svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &ixs,
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        );
        match self.svm.send_transaction(tx) {
            Ok(_) => Err(anyhow!("expected a failure, transaction succeeded")),
            Err(e) => Ok((e.meta.compute_units_consumed, format!("{:?}", e.err))),
        }
    }

    fn issuer_pda(&self, key: &[u8; 32]) -> Pubkey {
        Pubkey::find_program_address(&[b"issuer", key], &self.passport_program).0
    }

    fn passport_pda(&self, circuit: &[u8; 32], issuer: &[u8; 32]) -> Pubkey {
        Pubkey::find_program_address(
            &[b"passport", circuit, issuer],
            &self.passport_program,
        )
        .0
    }
}

fn core(circuit: [u8; 32], issuer: [u8; 32], sequence: u64, status: Status, now: i64) -> PassportCore {
    PassportCore {
        version: PASSPORT_VERSION,
        circuit_hash: circuit,
        issuer,
        sequence,
        capability: Capability::C1,
        status,
        issued_at: now,
        expires_at: now + 86_400,
        evidence_hash: [0x22; 32],
        advisory_count: if status == Status::Invalid { 3 } else { 0 },
    }
}

fn main() -> Result<()> {
    let deploy = Path::new("solana/target/deploy");
    let mut b = Bench::new(deploy)?;

    // A real ed25519 issuer key, as an offline issuer would hold.
    let issuer_key = SigningKey::from_bytes(&[7u8; 32]);
    let issuer_pub = issuer_key.verifying_key().to_bytes();
    let circuit = [0x11u8; 32];

    let issuer_pda = b.issuer_pda(&issuer_pub);
    let passport_pda = b.passport_pda(&circuit, &issuer_pub);

    println!("Halflife — measured compute cost on Solana");
    println!("passport program  {}", b.passport_program);
    println!("consumer program  {}", b.consumer_program);
    println!();

    // ---- register the issuer -------------------------------------------
    let mut data = discriminator("register_issuer").to_vec();
    data.extend_from_slice(&issuer_pub);
    let cu = b.send(
        vec![Instruction {
            program_id: b.passport_program,
            accounts: vec![
                AccountMeta::new(issuer_pda, false),
                AccountMeta::new(b.payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data,
        }],
        "register_issuer",
    )?;
    println!("  register_issuer            {cu:>7} CU");

    // ---- publish a VALID passport ---------------------------------------
    let now = b.svm.get_sysvar::<solana_sdk::clock::Clock>().unix_timestamp;
    let valid = core(circuit, issuer_pub, 1, Status::Valid, now);
    let publish_cu = publish(&mut b, &issuer_key, &valid, issuer_pda, passport_pda)?;
    println!("  publish (valid)            {publish_cu:>7} CU");

    // ---- THE NUMBER: a third-party program checking the passport --------
    let mut check_data = discriminator("bench_check").to_vec();
    check_data.extend_from_slice(&circuit);
    check_data.push(1); // require capability >= C1
    let check_ix = Instruction {
        program_id: b.consumer_program,
        accounts: vec![
            // Read-only. This is the property the whole argument rests on.
            AccountMeta::new_readonly(passport_pda, false),
            AccountMeta::new_readonly(b.payer.pubkey(), true),
        ],
        data: check_data.clone(),
    };
    let check_cu = b.send(vec![check_ix.clone()], "bench_check")?;
    println!("  consumer check             {check_cu:>7} CU   <-- the number");
    println!();

    // ---- the kill switch actually fires ---------------------------------
    let invalid = core(circuit, issuer_pub, 2, Status::Invalid, now);
    publish(&mut b, &issuer_key, &invalid, issuer_pda, passport_pda)?;

    let mut proof_data = discriminator("submit_proof").to_vec();
    proof_data.extend_from_slice(&circuit);
    proof_data.push(1);
    let proof_ix = Instruction {
        program_id: b.consumer_program,
        accounts: vec![
            AccountMeta::new_readonly(passport_pda, false),
            AccountMeta::new_readonly(b.payer.pubkey(), true),
        ],
        data: proof_data,
    };
    let (blocked_cu, err) = b.send_expecting_failure(vec![proof_ix])?;
    println!("  consumer BLOCKED           {blocked_cu:>7} CU   {}", error_name(&err));

    // ---- a superseded passport cannot be replayed ------------------------
    //
    // The blockhash must be expired first. Without it the replay is a
    // byte-identical transaction, which the runtime rejects as a duplicate
    // signature before the program runs at all -- so the test would pass while
    // proving nothing about the sequence check.
    b.svm.expire_blockhash();
    let replay = core(circuit, issuer_pub, 1, Status::Valid, now);
    let message = signing_message(&replay);
    let mut pdata = discriminator("publish").to_vec();
    pdata.extend_from_slice(&replay.canonical_bytes());
    let (_, replay_err) = b.send_expecting_failure(vec![
        ed25519_instruction(&issuer_key, &message),
        Instruction {
            program_id: b.passport_program,
            accounts: publish_accounts(&b, issuer_pda, passport_pda),
            data: pdata,
        },
    ])?;
    println!("  replay of seq 1 refused            {}", error_name(&replay_err));

    // ---- the fail-safe: no update arrives, so the consumer blocks ---------
    //
    // Nothing is published here. The clock simply moves past expiry, and the
    // passport degrades to stale on its own. This is the property that makes
    // suppressing delivery useless to an attacker.
    let fresh = core(circuit, issuer_pub, 3, Status::Valid, now);
    b.svm.expire_blockhash();
    publish(&mut b, &issuer_key, &fresh, issuer_pda, passport_pda)?;

    let mut clock = b.svm.get_sysvar::<solana_sdk::clock::Clock>();
    clock.unix_timestamp = fresh.expires_at + 1;
    b.svm.set_sysvar(&clock);

    let mut stale_data = discriminator("submit_proof").to_vec();
    stale_data.extend_from_slice(&circuit);
    stale_data.push(1);
    b.svm.expire_blockhash();
    let (stale_cu, stale_err) = b.send_expecting_failure(vec![Instruction {
        program_id: b.consumer_program,
        accounts: vec![
            AccountMeta::new_readonly(passport_pda, false),
            AccountMeta::new_readonly(b.payer.pubkey(), true),
        ],
        data: stale_data,
    }])?;
    println!("  consumer BLOCKED (stale)   {stale_cu:>7} CU   {}", error_name(&stale_err));
    println!("                                     no invalidation was published");

    println!();
    println!("Measured with litesvm against the SBF artifacts from `anchor build`.");
    println!("Reproduce: cargo run --release -p halflife-bench");
    Ok(())
}

fn signing_message(c: &PassportCore) -> Vec<u8> {
    let mut m = Vec::with_capacity(SIGNING_DOMAIN.len() + 125);
    m.extend_from_slice(SIGNING_DOMAIN);
    m.extend_from_slice(&c.canonical_bytes());
    m
}

fn publish_accounts(b: &Bench, issuer_pda: Pubkey, passport_pda: Pubkey) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(passport_pda, false),
        AccountMeta::new_readonly(issuer_pda, false),
        AccountMeta::new(b.payer.pubkey(), true),
        AccountMeta::new_readonly(instructions_sysvar::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ]
}

fn publish(
    b: &mut Bench,
    key: &SigningKey,
    c: &PassportCore,
    issuer_pda: Pubkey,
    passport_pda: Pubkey,
) -> Result<u64> {
    let message = signing_message(c);
    let mut data = discriminator("publish").to_vec();
    data.extend_from_slice(&c.canonical_bytes());
    let accounts = publish_accounts(b, issuer_pda, passport_pda);
    b.send(
        vec![
            // Must be instruction 0: the program inspects that index.
            ed25519_instruction(key, &message),
            Instruction {
                program_id: b.passport_program,
                accounts,
                data,
            },
        ],
        "publish",
    )
}
