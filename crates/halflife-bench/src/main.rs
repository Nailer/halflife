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
    const PASSPORT: [&str; 18] = [
        "UnsupportedVersion", "UnknownStatus", "UnknownCapability", "IssuerMismatch",
        "IssuerRevoked", "AlreadyRevoked", "MissingSignatureInstruction",
        "MalformedSignatureInstruction", "SignerMismatch", "SignedMessageMismatch",
        "IssuedInFuture", "ExpiryBeforeIssuance", "IssuedBeforeRegistration",
        "SequenceNotIncreasing", "CircuitMismatch", "MailboxMismatch", "NotAdmin",
        "UnknownMailbox",
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

/// Resolve a passport-program custom error code to its name, whichever
/// instruction index it surfaced at.
fn passport_error_name(raw: &str) -> String {
    const NAMES: [&str; 18] = [
        "UnsupportedVersion", "UnknownStatus", "UnknownCapability", "IssuerMismatch",
        "IssuerRevoked", "AlreadyRevoked", "MissingSignatureInstruction",
        "MalformedSignatureInstruction", "SignerMismatch", "SignedMessageMismatch",
        "IssuedInFuture", "ExpiryBeforeIssuance", "IssuedBeforeRegistration",
        "SequenceNotIncreasing", "CircuitMismatch", "MailboxMismatch", "NotAdmin",
        "UnknownMailbox",
    ];
    raw.split("Custom(")
        .nth(1)
        .and_then(|r| r.split(')').next())
        .and_then(|c| c.parse::<usize>().ok())
        .and_then(|c| c.checked_sub(6000))
        .and_then(|i| NAMES.get(i))
        .map(|n| (*n).to_string())
        .unwrap_or_else(|| raw.to_string())
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

    fn send_with(&mut self, ixs: Vec<Instruction>, extra: &[&Keypair], label: &str) -> Result<u64> {
        let blockhash = self.svm.latest_blockhash();
        let mut signers: Vec<&Keypair> = vec![&self.payer];
        signers.extend_from_slice(extra);
        let tx = Transaction::new_signed_with_payer(&ixs, Some(&self.payer.pubkey()), &signers, blockhash);
        match self.svm.send_transaction(tx) {
            Ok(r) => Ok(r.compute_units_consumed),
            Err(e) => Err(anyhow!("{label} failed: {:?}\n{:#?}", e.err, e.meta.logs)),
        }
    }

    /// An attempt that is *supposed* to be refused. Returns the passport
    /// program's error name, and errors if the transaction unexpectedly landed.
    fn refused(&mut self, ixs: Vec<Instruction>, extra: &[&Keypair]) -> Result<String> {
        self.svm.expire_blockhash();
        let blockhash = self.svm.latest_blockhash();
        let mut signers: Vec<&Keypair> = vec![&self.payer];
        signers.extend_from_slice(extra);
        let tx = Transaction::new_signed_with_payer(&ixs, Some(&self.payer.pubkey()), &signers, blockhash);
        match self.svm.send_transaction(tx) {
            Ok(_) => Err(anyhow!("an attack landed: the transaction was accepted")),
            Err(e) => Ok(passport_error_name(&format!("{:?}", e.err))),
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

    // ---- the registry needs an admin before anything else ----------------
    let config_pda = Pubkey::find_program_address(&[b"config"], &b.passport_program).0;
    let cu = b.send(
        vec![Instruction {
            program_id: b.passport_program,
            accounts: vec![
                AccountMeta::new(config_pda, false),
                AccountMeta::new(b.payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: discriminator("init_config").to_vec(),
        }],
        "init_config",
    )?;
    println!("  init_config                {cu:>7} CU");

    // ---- register the issuer, proving control of its key -------------------
    let cu = b.send(
        register_issuer_ixs(&b, &issuer_key, issuer_pub, &b.payer.pubkey(), issuer_pda),
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

    // ---- access control: both holes that existed are closed ----------------
    //
    // These are not hypothetical. An earlier version of this program let anyone
    // set a dispatch route (choosing the program that receives the registry's
    // dispatch signature) and let anyone register somebody else's public key as
    // an issuer. Each is attempted here by a keypair with no authority.
    println!();
    println!("  access control");
    let attacker = Keypair::new();
    b.svm.airdrop(&attacker.pubkey(), 5_000_000_000)
        .map_err(|e| anyhow!("airdrop failed: {e:?}"))?;

    let devnet_mailbox = Pubkey::from_str("5yM5YrrzHCrp4ZPLKN9Y2eUAqEWsTbBqaorgbngQcR54")?;
    let recipient = [0xEE; 32];

    // 1. A non-admin cannot set a route.
    let e1 = b.refused(
        vec![set_route_ix(&b, &attacker.pubkey(), 84532, recipient, devnet_mailbox)],
        &[&attacker],
    )?;
    println!("    set_route by a non-admin          refused   {e1}");
    if e1 != "NotAdmin" { return Err(anyhow!("expected NotAdmin, got {e1}")); }

    // 2. Even the admin cannot aim the dispatch signature at an unknown program.
    let hostile = Pubkey::new_unique();
    let e2 = b.refused(
        vec![set_route_ix(&b, &b.payer.pubkey(), 84532, recipient, hostile)],
        &[],
    )?;
    println!("    set_route to an unknown mailbox   refused   {e2}");
    if e2 != "UnknownMailbox" { return Err(anyhow!("expected UnknownMailbox, got {e2}")); }

    // 3. The admin can set a route to a real Hyperlane mailbox.
    b.svm.expire_blockhash();
    let cu = b.send(
        vec![set_route_ix(&b, &b.payer.pubkey(), 84532, recipient, devnet_mailbox)],
        "set_route",
    )?;
    println!("    set_route by the admin            accepted  {cu} CU");

    // 4. The config cannot be initialised a second time to seize the admin role.
    b.svm.expire_blockhash();
    let again = b.refused(
        vec![Instruction {
            program_id: b.passport_program,
            accounts: vec![
                AccountMeta::new(config_pda, false),
                AccountMeta::new(attacker.pubkey(), true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: discriminator("init_config").to_vec(),
        }],
        &[&attacker],
    )?;
    println!("    init_config a second time         refused   {again}");

    // 5. Nobody can register a victim's public key without holding its secret.
    let victim = SigningKey::from_bytes(&[42u8; 32]);
    let victim_pub = victim.verifying_key().to_bytes();
    let victim_pda = b.issuer_pda(&victim_pub);
    let attacker_sk = SigningKey::from_bytes(&[99u8; 32]);
    let e5 = b.refused(
        // The attacker signs with their own key but names the victim's.
        register_issuer_ixs(&b, &attacker_sk, victim_pub, &attacker.pubkey(), victim_pda),
        &[&attacker],
    )?;
    println!("    registering a key you don't hold  refused   {e5}");
    if e5 != "SignerMismatch" { return Err(anyhow!("expected SignerMismatch, got {e5}")); }

    // 6. Skipping the proof entirely.
    let mut bare = register_issuer_ixs(&b, &victim, victim_pub, &attacker.pubkey(), victim_pda);
    bare.remove(0);
    let e6 = b.refused(bare, &[&attacker])?;
    println!("    registering with no proof         refused   {e6}");
    if e6 != "MissingSignatureInstruction" { return Err(anyhow!("expected MissingSignatureInstruction, got {e6}")); }

    // 7. A proof made for one authority is useless to another.
    let stolen = register_issuer_ixs(&b, &victim, victim_pub, &b.payer.pubkey(), victim_pda);
    let e7 = b.refused(
        // Replay the victim's proof (bound to the payer) under the attacker.
        vec![stolen[0].clone(), {
            let mut ix = stolen[1].clone();
            ix.accounts[1] = AccountMeta::new(attacker.pubkey(), true);
            ix
        }],
        &[&attacker],
    )?;
    println!("    replaying a proof as someone else refused   {e7}");
    if e7 != "SignedMessageMismatch" { return Err(anyhow!("expected SignedMessageMismatch, got {e7}")); }
    println!();

    // ---- the dispatch body must be the bytes that were signed -------------
    //
    // The program re-encodes the stored passport to 125 bytes before handing it
    // to Hyperlane. If that re-encoding drifts from the encoder that produced
    // the signature, the destination decodes something nobody ever signed --
    // silently, because the destination cannot verify ed25519. So the round
    // trip through the on-chain account is checked explicitly.
    let stored = b
        .svm
        .get_account(&passport_pda)
        .ok_or_else(|| anyhow!("passport account missing"))?;
    // Anchor discriminator (8) then the fields in declaration order, which is
    // the canonical layout with a `bump` appended.
    let d = &stored.data[8..];
    let mut rebuilt = Vec::with_capacity(125);
    rebuilt.push(1u8);
    rebuilt.extend_from_slice(&d[0..32]);    // circuit_hash
    rebuilt.extend_from_slice(&d[32..64]);   // issuer
    rebuilt.extend_from_slice(&d[64..72]);   // sequence
    rebuilt.push(d[72]);                     // capability
    rebuilt.push(d[73]);                     // status
    rebuilt.extend_from_slice(&d[74..82]);   // issued_at
    rebuilt.extend_from_slice(&d[82..90]);   // expires_at
    rebuilt.extend_from_slice(&d[90..122]);  // evidence_hash
    rebuilt.extend_from_slice(&d[122..124]); // advisory_count

    if rebuilt[..] != fresh.canonical_bytes()[..] {
        return Err(anyhow!(
            "on-chain round trip altered the signed bytes\n  signed:  {}\n  rebuilt: {}",
            hex::encode(fresh.canonical_bytes()),
            hex::encode(&rebuilt)
        ));
    }
    println!("  round trip preserves 125 bytes      stored == signed");

    println!();
    println!("Measured with litesvm against the SBF artifacts from `anchor build`.");
    println!("Reproduce: cargo run --release -p halflife-bench");
    Ok(())
}

const REGISTER_DOMAIN: &[u8] = b"halflife-register-issuer-v1";

/// An issuer registration: the issuer key signs `REGISTER_DOMAIN || authority`,
/// then the program instruction. `claimed` is the key the instruction names, and
/// is separate from `signer` so an attack can claim a key it does not hold.
fn register_issuer_ixs(
    b: &Bench,
    signer: &SigningKey,
    claimed: [u8; 32],
    authority: &Pubkey,
    issuer_pda: Pubkey,
) -> Vec<Instruction> {
    let mut msg = Vec::new();
    msg.extend_from_slice(REGISTER_DOMAIN);
    msg.extend_from_slice(authority.as_ref());
    let mut data = discriminator("register_issuer").to_vec();
    data.extend_from_slice(&claimed);
    vec![
        ed25519_instruction(signer, &msg),
        Instruction {
            program_id: b.passport_program,
            accounts: vec![
                AccountMeta::new(issuer_pda, false),
                AccountMeta::new(*authority, true),
                AccountMeta::new_readonly(instructions_sysvar::ID, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data,
        },
    ]
}

fn set_route_ix(b: &Bench, admin: &Pubkey, domain: u32, recipient: [u8; 32], mailbox: Pubkey) -> Instruction {
    let route = Pubkey::find_program_address(&[b"route", &domain.to_le_bytes()], &b.passport_program).0;
    let config = Pubkey::find_program_address(&[b"config"], &b.passport_program).0;
    let mut data = discriminator("set_route").to_vec();
    data.extend_from_slice(&domain.to_le_bytes());
    data.extend_from_slice(&recipient);
    data.extend_from_slice(mailbox.as_ref());
    Instruction {
        program_id: b.passport_program,
        accounts: vec![
            AccountMeta::new(route, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(*admin, true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
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
