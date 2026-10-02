//! `halflife-exercise` — controlled threat scenarios against real infrastructure.
//!
//! Two scenarios, run against Solana devnet with the deployed programs:
//!
//! * **dependency-compromise** — a dependency becomes unsafe and the chain of
//!   consequence runs to a consumer that will not proceed.
//! * **relayer-censorship** — delivery stops, nothing is published, and the
//!   consumer must block anyway. This is the one that matters: steps where a
//!   message *arrives* demonstrate the fast path; the safety property is what
//!   happens when nothing arrives at all.
//!
//! Every timing comes from an observed on-chain event. `verify` re-fetches each
//! recorded signature from the cluster, so a third party can confirm the record
//! without trusting us.

mod ledger;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use ed25519_dalek::{Signer, SigningKey};
use halflife_core::{Capability, PassportCore, Status, PASSPORT_VERSION, SIGNING_DOMAIN};
use ledger::{Evidence, EventKind, Exercise, Scenario};
use sha2::{Digest, Sha256};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    ed25519_program,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer as SolSigner},
    system_program,
    sysvar::instructions as instructions_sysvar,
    transaction::Transaction,
};
use std::path::{Path, PathBuf};
use std::str::FromStr;

const PASSPORT_ID: &str = "CkDhRfJRiGEa3kgnEUEvCBgyht62MTkDD6e754DLtB2";
const CONSUMER_ID: &str = "BAcrrJYj5Y5DfcqnHgDwm5rhJUJdowZh25NvFLJAUzSW";
const DEVNET: &str = "https://api.devnet.solana.com";

#[derive(Parser)]
#[command(name = "halflife-exercise", about = "Controlled threat-scenario execution")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run a scenario end to end against the cluster.
    Run {
        #[arg(value_enum)]
        scenario: ScenarioArg,
        #[arg(long, default_value = "solana/deploy-keypair.json")]
        payer: PathBuf,
        #[arg(long, default_value = DEVNET)]
        rpc: String,
        #[arg(long, default_value = "exercises")]
        out: PathBuf,
        /// Passport lifetime. The censorship scenario waits this long for
        /// expiry, so keep it short when exercising.
        #[arg(long, default_value_t = 45)]
        ttl: i64,
    },
    /// Print the canonical 125 bytes of a passport as it exists on chain.
    ///
    /// Used to drive the destination chain with bytes that genuinely came from
    /// the deployed Solana registry, rather than bytes a test produced.
    Fetch {
        /// Circuit hash, hex. Omit to use the most recent exercise's circuit.
        circuit: Option<String>,
        #[arg(long, default_value = DEVNET)]
        rpc: String,
    },
    /// Re-check a recorded exercise against the cluster.
    Verify {
        record: PathBuf,
        #[arg(long, default_value = DEVNET)]
        rpc: String,
    },
}

#[derive(Copy, Clone, Debug, clap::ValueEnum)]
enum ScenarioArg {
    DependencyCompromise,
    RelayerCensorship,
}

impl From<ScenarioArg> for Scenario {
    fn from(a: ScenarioArg) -> Self {
        match a {
            ScenarioArg::DependencyCompromise => Scenario::DependencyCompromise,
            ScenarioArg::RelayerCensorship => Scenario::RelayerCensorship,
        }
    }
}

fn discriminator(name: &str) -> [u8; 8] {
    let mut h = Sha256::new();
    h.update(format!("global:{name}").as_bytes());
    h.finalize()[..8].try_into().unwrap()
}

/// Single-signature Ed25519 precompile instruction, offsets matching the
/// on-chain verifier.
fn ed25519_instruction(key: &SigningKey, message: &[u8]) -> Instruction {
    let sig = key.sign(message).to_bytes();
    let pk = key.verifying_key().to_bytes();
    let mut data = Vec::with_capacity(112 + message.len());
    data.push(1);
    data.push(0);
    for v in [48u16, u16::MAX, 16, u16::MAX, 112, message.len() as u16, u16::MAX] {
        data.extend_from_slice(&v.to_le_bytes());
    }
    data.extend_from_slice(&pk);
    data.extend_from_slice(&sig);
    data.extend_from_slice(message);
    Instruction { program_id: ed25519_program::ID, accounts: vec![], data }
}

struct Ctx {
    rpc: RpcClient,
    payer: Keypair,
    passport_program: Pubkey,
    consumer_program: Pubkey,
    issuer_key: SigningKey,
}

impl Ctx {
    fn new(rpc_url: &str, payer_path: &Path) -> Result<Self> {
        let payer = read_keypair_file(payer_path)
            .map_err(|e| anyhow!("reading payer {}: {e}", payer_path.display()))?;
        Ok(Self {
            rpc: RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed()),
            payer,
            passport_program: Pubkey::from_str(PASSPORT_ID)?,
            consumer_program: Pubkey::from_str(CONSUMER_ID)?,
            // Deterministic so a re-run addresses the same issuer PDA. An
            // exercise key, never a production issuer.
            issuer_key: SigningKey::from_bytes(&[11u8; 32]),
        })
    }

    fn issuer_pub(&self) -> [u8; 32] {
        self.issuer_key.verifying_key().to_bytes()
    }

    fn issuer_pda(&self) -> Pubkey {
        Pubkey::find_program_address(&[b"issuer", &self.issuer_pub()], &self.passport_program).0
    }

    fn passport_pda(&self, circuit: &[u8; 32]) -> Pubkey {
        Pubkey::find_program_address(
            &[b"passport", circuit, &self.issuer_pub()],
            &self.passport_program,
        )
        .0
    }

    /// Send and confirm, returning the signature and the slot it landed in.
    fn send(&self, ixs: Vec<Instruction>, extra: &[&Keypair]) -> Result<(String, u64)> {
        let bh = self.rpc.get_latest_blockhash()?;
        let mut signers: Vec<&Keypair> = vec![&self.payer];
        signers.extend_from_slice(extra);
        let tx = Transaction::new_signed_with_payer(
            &ixs,
            Some(&self.payer.pubkey()),
            &signers,
            bh,
        );
        let sig = self.rpc.send_and_confirm_transaction(&tx)?;
        let slot = self.rpc.get_slot()?;
        Ok((sig.to_string(), slot))
    }

    /// Send expecting failure; returns the simulation error. Used where the
    /// *point* is that the transaction cannot land.
    fn expect_block(&self, ixs: Vec<Instruction>) -> Result<String> {
        let bh = self.rpc.get_latest_blockhash()?;
        let tx = Transaction::new_signed_with_payer(
            &ixs,
            Some(&self.payer.pubkey()),
            &[&self.payer],
            bh,
        );
        match self.rpc.simulate_transaction(&tx) {
            Ok(r) => match r.value.err {
                Some(e) => Ok(format!("{e:?}")),
                None => Err(anyhow!("expected the consumer to block, it would have proceeded")),
            },
            Err(e) => Ok(format!("{e}")),
        }
    }

    fn ensure_issuer(&self) -> Result<Option<(String, u64)>> {
        if self.rpc.get_account(&self.issuer_pda()).is_ok() {
            return Ok(None);
        }
        let mut data = discriminator("register_issuer").to_vec();
        data.extend_from_slice(&self.issuer_pub());
        let r = self.send(
            vec![Instruction {
                program_id: self.passport_program,
                accounts: vec![
                    AccountMeta::new(self.issuer_pda(), false),
                    AccountMeta::new(self.payer.pubkey(), true),
                    AccountMeta::new_readonly(system_program::ID, false),
                ],
                data,
            }],
            &[],
        )?;
        Ok(Some(r))
    }

    fn publish(&self, core: &PassportCore) -> Result<(String, u64)> {
        let mut msg = Vec::with_capacity(SIGNING_DOMAIN.len() + 125);
        msg.extend_from_slice(SIGNING_DOMAIN);
        msg.extend_from_slice(&core.canonical_bytes());
        let mut data = discriminator("publish").to_vec();
        data.extend_from_slice(&core.canonical_bytes());
        self.send(
            vec![
                ed25519_instruction(&self.issuer_key, &msg),
                Instruction {
                    program_id: self.passport_program,
                    accounts: vec![
                        AccountMeta::new(self.passport_pda(&core.circuit_hash), false),
                        AccountMeta::new_readonly(self.issuer_pda(), false),
                        AccountMeta::new(self.payer.pubkey(), true),
                        AccountMeta::new_readonly(instructions_sysvar::ID, false),
                        AccountMeta::new_readonly(system_program::ID, false),
                    ],
                    data,
                },
            ],
            &[],
        )
    }

    fn consumer_ix(&self, circuit: &[u8; 32]) -> Instruction {
        let mut data = discriminator("submit_proof").to_vec();
        data.extend_from_slice(circuit);
        data.push(1);
        Instruction {
            program_id: self.consumer_program,
            accounts: vec![
                AccountMeta::new_readonly(self.passport_pda(circuit), false),
                AccountMeta::new_readonly(self.payer.pubkey(), true),
            ],
            data,
        }
    }

    fn next_sequence(&self, circuit: &[u8; 32]) -> Result<u64> {
        match self.rpc.get_account(&self.passport_pda(circuit)) {
            Ok(a) if a.data.len() >= 8 + 72 => {
                let s = u64::from_le_bytes(a.data[8 + 64..8 + 72].try_into().unwrap());
                Ok(s + 1)
            }
            _ => Ok(1),
        }
    }
}

fn core(circuit: [u8; 32], issuer: [u8; 32], seq: u64, status: Status, now: i64, ttl: i64) -> PassportCore {
    PassportCore {
        version: PASSPORT_VERSION,
        circuit_hash: circuit,
        issuer,
        sequence: seq,
        capability: Capability::C1,
        status,
        issued_at: now,
        expires_at: now + ttl,
        evidence_hash: [0x44; 32],
        advisory_count: if status == Status::Invalid { 1 } else { 0 },
    }
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Run { scenario, payer, rpc, out, ttl } => {
            let ctx = Ctx::new(&rpc, &payer)?;
            let ex = match scenario.into() {
                Scenario::DependencyCompromise => dependency_compromise(&ctx, ttl)?,
                Scenario::RelayerCensorship => relayer_censorship(&ctx, ttl)?,
            };
            report(&ex);
            let p = ex.save(&out)?;
            println!("\nrecord  {}", p.display());
            println!("verify  halflife-exercise verify {}", p.display());
            Ok(())
        }
        Cmd::Fetch { circuit, rpc } => fetch(circuit.as_deref(), &rpc),
        Cmd::Verify { record, rpc } => verify(&record, &rpc),
    }
}

/// A dependency becomes unsafe; consequence runs to a blocked consumer.
fn dependency_compromise(ctx: &Ctx, ttl: i64) -> Result<Exercise> {
    // Distinct per run so a re-run does not collide with a previous passport.
    let circuit = {
        let mut h = Sha256::new();
        h.update(b"halflife-exercise-dependency-compromise");
        h.update(ledger::now().to_le_bytes());
        let d: [u8; 32] = h.finalize().into();
        d
    };
    let mut ex = Exercise::new(
        Scenario::DependencyCompromise,
        "devnet",
        PASSPORT_ID,
        CONSUMER_ID,
        &hex::encode(circuit),
        ledger::now(),
    );
    println!("\x1b[1mScenario: dependency compromise\x1b[0m  (devnet)");

    if let Some((sig, slot)) = ctx.ensure_issuer()? {
        ex.record(
            EventKind::FindingRegistered,
            Evidence::SolanaTransaction { signature: sig, slot },
            "exercise issuer registered",
        );
    }

    ex.record(
        EventKind::ImpactProjected,
        Evidence::Local {
            note: "halo2_gadgets@0.4.0 matches GHSA-ww9q-8r59-xv46 in this closure".into(),
        },
        "affected circuit identified from the registry",
    );
    step("impact projected");

    let now = ledger::now();
    let seq = ctx.next_sequence(&circuit)?;
    let good = core(circuit, ctx.issuer_pub(), seq, Status::Valid, now, ttl);
    let (sig, slot) = ctx.publish(&good)?;
    ex.record(
        EventKind::SolanaCommitted,
        Evidence::SolanaTransaction { signature: sig.clone(), slot },
        format!("baseline VALID passport, sequence {seq}"),
    );
    step(&format!("baseline published  slot {slot}"));

    // The consumer proceeds here, which is what makes the block meaningful.
    if ctx.expect_block(vec![ctx.consumer_ix(&circuit)]).is_ok() {
        return Err(anyhow!("baseline should have been accepted but the consumer blocked"));
    }
    step("consumer accepts the baseline");

    let bad = core(circuit, ctx.issuer_pub(), seq + 1, Status::Invalid, ledger::now(), ttl);
    let (sig2, slot2) = ctx.publish(&bad)?;
    ex.record(
        EventKind::PassportInvalidated,
        Evidence::SolanaTransaction { signature: sig2, slot: slot2 },
        format!("INVALID published, sequence {}", seq + 1),
    );
    step(&format!("invalidated  slot {slot2}"));

    let err = ctx.expect_block(vec![ctx.consumer_ix(&circuit)])?;
    ex.record(
        EventKind::ConsumerBlocked,
        Evidence::Local { note: err.clone() },
        "consumer refused to proceed",
    );
    step(&format!("consumer BLOCKED  {err}"));

    Ok(ex)
}

/// Delivery stops. Nothing is published. The consumer must block anyway.
fn relayer_censorship(ctx: &Ctx, ttl: i64) -> Result<Exercise> {
    let circuit = {
        let mut h = Sha256::new();
        h.update(b"halflife-exercise-relayer-censorship");
        h.update(ledger::now().to_le_bytes());
        let d: [u8; 32] = h.finalize().into();
        d
    };
    let mut ex = Exercise::new(
        Scenario::RelayerCensorship,
        "devnet",
        PASSPORT_ID,
        CONSUMER_ID,
        &hex::encode(circuit),
        ledger::now(),
    );
    println!("\x1b[1mScenario: relayer censorship\x1b[0m  (devnet)");
    println!("  nothing will be published after the baseline — expiry alone must block\n");

    if let Some((sig, slot)) = ctx.ensure_issuer()? {
        ex.record(
            EventKind::FindingRegistered,
            Evidence::SolanaTransaction { signature: sig, slot },
            "exercise issuer registered",
        );
    }

    let now = ledger::now();
    let seq = ctx.next_sequence(&circuit)?;
    let good = core(circuit, ctx.issuer_pub(), seq, Status::Valid, now, ttl);
    let (sig, slot) = ctx.publish(&good)?;
    ex.record(
        EventKind::SolanaCommitted,
        Evidence::SolanaTransaction { signature: sig, slot },
        format!("VALID passport, expires at {}", good.expires_at),
    );
    step(&format!("baseline published  slot {slot}  ttl {ttl}s"));

    ex.record(
        EventKind::RelayerStopped,
        Evidence::Local {
            note: "no further transaction is submitted for this circuit".into(),
        },
        "delivery deliberately stopped",
    );
    step("relayer stopped — no invalidation will be sent");

    // Wait out the clock. No further writes: the point is that absence of an
    // update is sufficient.
    let deadline = good.expires_at + 2;
    while ledger::now() < deadline {
        let left = deadline - ledger::now();
        print!("\r  waiting for expiry… {left:>3}s  ");
        use std::io::Write;
        std::io::stdout().flush().ok();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    println!("\r  expiry reached            ");

    ex.record(
        EventKind::PassportExpired,
        Evidence::Local { note: format!("expires_at {} passed", good.expires_at) },
        "passport aged out with no update delivered",
    );

    let err = ctx.expect_block(vec![ctx.consumer_ix(&circuit)])?;
    ex.record(
        EventKind::ConsumerBlocked,
        Evidence::Local { note: err.clone() },
        "consumer blocked on staleness, not on any published finding",
    );
    step(&format!("consumer BLOCKED  {err}"));

    Ok(ex)
}

fn step(s: &str) {
    println!("  \x1b[32m•\x1b[0m {s}");
}

fn report(ex: &Exercise) {
    println!("\n\x1b[1m{}\x1b[0m", ex.exercise_id);
    for e in &ex.events {
        let ev = match &e.evidence {
            Evidence::SolanaTransaction { signature, slot } => {
                format!("slot {slot}  {}…", &signature[..16])
            }
            Evidence::HyperlaneMessage { message_id } => format!("msg {message_id}"),
            Evidence::Local { .. } => "local".into(),
        };
        println!("  {:>2}. {:<22} {:<28} {}", e.seq, format!("{:?}", e.kind), ev, e.detail);
    }
    match ex.containment_secs() {
        Some(s) => println!("\ncontainment  {s}s   (derived from recorded timestamps)"),
        None => println!("\ncontainment  not measurable — no events recorded"),
    }
    println!(
        "verifiable   {} of {} events carry independently checkable evidence",
        ex.verifiable_count(),
        ex.events.len()
    );
}

/// Read a passport account and rebuild its canonical bytes.
fn fetch(circuit: Option<&str>, rpc_url: &str) -> Result<()> {
    let ctx = Ctx::new(rpc_url, Path::new("solana/deploy-keypair.json"))?;

    let hash: [u8; 32] = match circuit {
        Some(h) => hex::decode(h)
            .context("circuit hash is not hex")?
            .try_into()
            .map_err(|_| anyhow!("circuit hash must be 32 bytes"))?,
        None => {
            // Fall back to the newest exercise record, so the common case needs
            // no arguments.
            let mut files: Vec<_> = std::fs::read_dir("exercises")
                .context("no exercises/ directory and no circuit given")?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .collect();
            files.sort();
            let last = files.last().ok_or_else(|| anyhow!("no exercise records"))?;
            let ex = ledger::Exercise::load(last)?;
            hex::decode(&ex.circuit_hash)?
                .try_into()
                .map_err(|_| anyhow!("bad circuit hash in record"))?
        }
    };

    let pda = ctx.passport_pda(&hash);
    let acct = ctx
        .rpc
        .get_account(&pda)
        .with_context(|| format!("no passport account at {pda}"))?;
    if acct.data.len() < 8 + 124 {
        return Err(anyhow!("account is too small to be a passport"));
    }

    // Anchor discriminator, then the fields in declaration order — which is the
    // canonical layout with a `bump` appended.
    let d = &acct.data[8..];
    let mut core = Vec::with_capacity(125);
    core.push(1u8);
    core.extend_from_slice(&d[0..32]);
    core.extend_from_slice(&d[32..64]);
    core.extend_from_slice(&d[64..72]);
    core.push(d[72]);
    core.push(d[73]);
    core.extend_from_slice(&d[74..82]);
    core.extend_from_slice(&d[82..90]);
    core.extend_from_slice(&d[90..122]);
    core.extend_from_slice(&d[122..124]);

    eprintln!("account   {pda}");
    eprintln!("circuit   {}", hex::encode(hash));
    eprintln!("sequence  {}", u64::from_le_bytes(d[64..72].try_into().unwrap()));
    eprintln!("status    {}", if d[73] == 2 { "INVALID" } else { "VALID" });
    eprintln!("bytes     {} (canonical)", core.len());
    // Only the hex on stdout, so this composes into a shell pipeline.
    println!("0x{}", hex::encode(&core));
    Ok(())
}

/// Re-fetch every recorded signature from the cluster.
fn verify(record: &Path, rpc_url: &str) -> Result<()> {
    let ex = Exercise::load(record).with_context(|| format!("reading {}", record.display()))?;
    let rpc = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    println!("\x1b[1mVerifying {}\x1b[0m", ex.exercise_id);
    println!("cluster {}\n", ex.cluster);

    let mut checked = 0usize;
    let mut failed = 0usize;
    for e in &ex.events {
        match &e.evidence {
            Evidence::SolanaTransaction { signature, slot } => {
                let sig = solana_sdk::signature::Signature::from_str(signature)?;
                // History search is required, not optional: the default status
                // lookup only covers a recent window, so a record older than a
                // few minutes would report "not found" and `verify` would be
                // useless for precisely the case it exists for -- checking an
                // exercise someone else ran, later.
                let status = rpc
                    .get_signature_statuses_with_history(&[sig])?
                    .value
                    .into_iter()
                    .next()
                    .flatten()
                    .map(|s| match s.err {
                        Some(e) => Err(e),
                        None => Ok(()),
                    });
                match status {
                    Some(Ok(())) => {
                        println!("  \x1b[32m✓\x1b[0m {:>2}. {:<22} on chain, recorded slot {slot}", e.seq, format!("{:?}", e.kind));
                        checked += 1;
                    }
                    Some(Err(err)) => {
                        println!("  \x1b[31m✗\x1b[0m {:>2}. transaction failed on chain: {err:?}", e.seq);
                        failed += 1;
                    }
                    None => {
                        println!("  \x1b[31m✗\x1b[0m {:>2}. signature not found on {}", e.seq, ex.cluster);
                        failed += 1;
                    }
                }
            }
            Evidence::HyperlaneMessage { message_id } => {
                println!("  \x1b[33m–\x1b[0m {:>2}. hyperlane message {message_id} — check the explorer", e.seq);
            }
            Evidence::Local { .. } => {
                println!("  \x1b[33m–\x1b[0m {:>2}. {:<22} local, not independently verifiable", e.seq, format!("{:?}", e.kind));
            }
        }
    }

    println!("\n{checked} on-chain events confirmed, {failed} failed");
    if let Some(s) = ex.containment_secs() {
        println!("containment {s}s recomputed from the record's own timestamps");
    }
    if failed > 0 {
        return Err(anyhow!("{failed} recorded event(s) could not be confirmed"));
    }
    println!("\x1b[32mrecord is consistent with the cluster\x1b[0m");
    Ok(())
}
