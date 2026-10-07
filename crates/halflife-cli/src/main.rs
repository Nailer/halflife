//! `halflife` — produce, register and inspect Security Passports.
//!
//! The deterministic path only. No model is involved anywhere, and the tool
//! claims capability C1: dependency closure resolution and advisory matching.

mod keys;
mod vectors;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use halflife_core::{
    Audience, DisclosureState, EffectiveStatus, Evidence, PassportCore, SignedPassport,
    Status, PASSPORT_VERSION,
};
use halflife_lineage::{resolve, AdvisorySource};
use halflife_projection::{project, Impact};
use halflife_registry::Store;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_REGISTRY: &str = "halflife.db";

#[derive(Parser)]
#[command(
    name = "halflife",
    version,
    about = "Security passports for zero-knowledge circuits"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Generate an issuer keypair in Solana keypair format.
    Keygen {
        #[arg(short, long, default_value = "issuer.json")]
        out: PathBuf,
    },
    /// Scan a circuit's closure and emit a signed passport to disk.
    Scan {
        target: PathBuf,
        #[command(flatten)]
        common: ScanArgs,
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Exit non-zero if the closure carries an advisory. This is the CI
        /// gate: a pull request that moves a circuit onto an affected
        /// dependency fails the build instead of reaching an incident.
        #[arg(long)]
        deny: bool,
    },
    /// Scan a circuit and record it in the registry.
    Register {
        target: PathBuf,
        #[command(flatten)]
        common: ScanArgs,
        #[arg(long, default_value = DEFAULT_REGISTRY)]
        registry: PathBuf,
    },
    /// What would break if this dependency became unsafe?
    Impact {
        /// `name@version`, e.g. halo2_gadgets@0.4.0
        target: String,
        #[arg(long, default_value = DEFAULT_REGISTRY)]
        registry: PathBuf,
        /// Answer as an authorized operator, seeing embargoed findings too.
        #[arg(long)]
        operator: bool,
        /// Emit JSON instead of a table.
        #[arg(long)]
        json: bool,
    },
    /// Inspect the registry.
    Registry {
        #[command(subcommand)]
        op: RegistryOp,
        #[arg(long, default_value = DEFAULT_REGISTRY, global = true)]
        registry: PathBuf,
    },
    /// Export the whole system state as JSON for the control room.
    Export {
        #[arg(long, default_value = DEFAULT_REGISTRY)]
        registry: PathBuf,
        #[arg(long, default_value = "exercises")]
        exercises: PathBuf,
        #[arg(short, long, default_value = "apps/control-room/public/state.json")]
        out: PathBuf,
        /// Export the operator view. Without this the export is the public view
        /// and embargoed findings are absent, not redacted.
        #[arg(long)]
        operator: bool,
    },
    /// Verify a passport's signature and bindings, and resolve its status now.
    Verify {
        passport: PathBuf,
        #[arg(short, long)]
        evidence: Option<PathBuf>,
        #[arg(long)]
        at: Option<i64>,
    },
    /// Generate or verify the conformance vectors that pin the wire format.
    Vectors {
        #[command(subcommand)]
        op: VectorOp,
    },
}

#[derive(clap::Args)]
struct ScanArgs {
    #[arg(short, long, default_value = "issuer.json")]
    issuer: PathBuf,
    /// Use a pinned advisory snapshot instead of querying OSV.
    #[arg(long)]
    offline: bool,
    #[arg(long, default_value = "fixtures/osv-snapshot.json")]
    snapshot: PathBuf,
    /// Passport lifetime in seconds.
    #[arg(long, default_value_t = 86_400)]
    ttl: i64,
    /// Monotonic sequence for this (circuit, issuer). Must increase.
    #[arg(long, default_value_t = 1)]
    sequence: u64,
    /// Whether the underlying finding may be discussed publicly.
    #[arg(long, value_enum, default_value_t = Disclosure::Embargoed)]
    disclosure: Disclosure,
}

#[derive(Subcommand)]
enum RegistryOp {
    /// Every registered circuit and its current status.
    List,
    /// Every distinct dependency the registry knows about.
    Deps,
}

#[derive(Subcommand)]
enum VectorOp {
    Generate {
        #[arg(short, long, default_value = "fixtures/vectors.json")]
        out: PathBuf,
    },
    Verify {
        #[arg(default_value = "fixtures/vectors.json")]
        path: PathBuf,
    },
}

#[derive(Copy, Clone, Debug, clap::ValueEnum)]
enum Disclosure {
    Public,
    Embargoed,
}

impl From<Disclosure> for DisclosureState {
    fn from(d: Disclosure) -> Self {
        match d {
            Disclosure::Public => DisclosureState::Public,
            Disclosure::Embargoed => DisclosureState::Embargoed,
        }
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Keygen { out } => keygen(&out),
        Cmd::Scan {
            target,
            common,
            out,
            deny,
        } => {
            let out = out.unwrap_or_else(|| target.join("out"));
            let (evidence, signed) = build(&target, &common)?;
            report(&evidence, &signed);
            write_out(&out, &evidence, &signed)?;
            println!("written   {}", out.display());
            if deny && signed.core.status == Status::Invalid {
                eprintln!(
                    "denied    {} advisory hit(s) in the closure",
                    evidence.advisory_count()
                );
                std::process::exit(2);
            }
            Ok(())
        }
        Cmd::Register {
            target,
            common,
            registry,
        } => {
            let (evidence, signed) = build(&target, &common)?;
            report(&evidence, &signed);
            let mut store = Store::open(&registry)?;
            let hash = store.register(&evidence, &signed, now())?;
            println!("registered {} in {}", &hash[..16], registry.display());
            if !evidence.is_public() {
                println!("           evidence withheld — finding is embargoed");
            }
            Ok(())
        }
        Cmd::Impact {
            target,
            registry,
            operator,
            json,
        } => impact(&target, &registry, operator, json),
        Cmd::Registry { op, registry } => match op {
            RegistryOp::List => registry_list(&registry),
            RegistryOp::Deps => registry_deps(&registry),
        },
        Cmd::Export {
            registry,
            exercises,
            out,
            operator,
        } => export(&registry, &exercises, &out, operator),
        Cmd::Verify {
            passport,
            evidence,
            at,
        } => verify(&passport, evidence.as_deref(), at.unwrap_or_else(now)),
        Cmd::Vectors { op } => match op {
            VectorOp::Generate { out } => vectors::generate(&out),
            VectorOp::Verify { path } => vectors::verify(&path),
        },
    }
}

fn keygen(out: &Path) -> Result<()> {
    if out.exists() {
        return Err(anyhow!(
            "{} already exists — refusing to overwrite an issuer key",
            out.display()
        ));
    }
    let issuer = keys::Issuer::generate()?;
    issuer.save(out)?;
    println!("issuer  {}", issuer.pubkey_b58());
    println!("written {}", out.display());
    Ok(())
}

/// Resolve a target and issue a passport over it.
fn build(target: &Path, args: &ScanArgs) -> Result<(Evidence, SignedPassport)> {
    let source = if args.offline {
        AdvisorySource::Snapshot(&args.snapshot)
    } else {
        AdvisorySource::Osv
    };
    let closure = resolve(target, source, args.disclosure.into())?;
    let evidence = closure.evidence;

    println!(
        "closure   {} packages ({} from registry)",
        evidence.dependencies.len(),
        closure.registry_count
    );
    println!("advisory  {}", evidence.advisory_source);

    let issuer = keys::Issuer::load(&args.issuer)?;
    let issued_at = now();
    let core = PassportCore {
        version: PASSPORT_VERSION,
        circuit_hash: evidence.circuit_hash(),
        issuer: issuer.pubkey_bytes(),
        sequence: args.sequence,
        capability: evidence.capability,
        status: evidence.derive_status(),
        issued_at,
        expires_at: issued_at + args.ttl,
        evidence_hash: evidence.hash()?,
        advisory_count: evidence.advisory_count().min(u16::MAX as usize) as u16,
    };

    let signed = SignedPassport {
        signature: issuer.sign(&core.signing_preimage()),
        issuer_id: issuer.pubkey_b58(),
        core,
    };
    Ok((evidence, signed))
}

fn report(evidence: &Evidence, signed: &SignedPassport) {
    println!("circuit   {}", hex::encode(signed.core.circuit_hash));
    println!("evidence  {}", hex::encode(signed.core.evidence_hash));
    println!("issuer    {}", signed.issuer_id);
    println!(
        "capability C{}  seq {}  disclosure {:?}",
        signed.core.capability.tier(),
        signed.core.sequence,
        evidence.disclosure
    );
    let hits: Vec<_> = evidence
        .dependencies
        .iter()
        .filter(|d| !d.advisories.is_empty())
        .collect();
    if hits.is_empty() {
        println!("findings  none");
    } else {
        for d in hits {
            println!(
                "findings  {}@{}  {}",
                d.name,
                d.version,
                d.advisories.join(", ")
            );
        }
    }
    println!("status    {:?}", signed.core.status);
}

fn write_out(out: &Path, evidence: &Evidence, signed: &SignedPassport) -> Result<()> {
    std::fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;
    std::fs::write(
        out.join("evidence.json"),
        serde_json::to_vec_pretty(evidence)?,
    )?;
    std::fs::write(
        out.join("passport.json"),
        serde_json::to_vec_pretty(signed)?,
    )?;
    Ok(())
}

fn impact(target: &str, registry: &Path, operator: bool, json: bool) -> Result<()> {
    let (name, version) = target.rsplit_once('@').ok_or_else(|| {
        anyhow!("expected name@version, e.g. halo2_gadgets@0.4.0 — got {target}")
    })?;
    let audience = if operator {
        Audience::Operator
    } else {
        Audience::Public
    };

    let store = Store::open(registry)?;
    let i: Impact = project(&store, name, version, audience)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&i)?);
        return Ok(());
    }

    println!("dependency  {}@{}", i.dependency, i.version);
    println!("audience    {:?}", i.audience);
    println!();

    if i.affected.is_empty() {
        println!("AFFECTED    none");
    } else {
        println!("AFFECTED    {}", i.affected.len());
        for r in &i.affected {
            println!(
                "  {:<24} {}  {}",
                truncate(&r.name, 24),
                &r.circuit_hash[..12],
                r.advisories.join(", ")
            );
        }
    }
    if !i.clean.is_empty() {
        println!("\nREACHED, CLEAN  {}", i.clean.len());
        for r in &i.clean {
            println!("  {:<24} {}", truncate(&r.name, 24), &r.circuit_hash[..12]);
        }
    }

    println!(
        "\nblast radius  {} affected · {} reached clean · {} not reached",
        i.affected.len(),
        i.clean.len(),
        i.not_reached
    );
    if !i.advisories.is_empty() {
        println!("advisories    {}", i.advisories.join(", "));
    }

    // Non-zero exit on impact, so this is usable as a CI gate without parsing
    // stdout -- the same reason `verify` exits 2 on a block.
    if !i.is_clear() {
        std::process::exit(2);
    }
    Ok(())
}

fn registry_list(path: &Path) -> Result<()> {
    let store = Store::open(path)?;
    let circuits = store.circuits()?;
    if circuits.is_empty() {
        println!("registry is empty — run `halflife register <target>`");
        return Ok(());
    }
    let n = now();
    println!(
        "{:<18} {:<12} {:>5}  {:<9} {}",
        "CIRCUIT", "ID", "DEPS", "STATUS", "SUBJECT"
    );
    let mut affected = 0usize;
    for c in &circuits {
        let deps = store.dependency_count(&c.circuit_hash)?;
        let status = match store.latest_passport(&c.circuit_hash)? {
            Some(p) => {
                let core_like = (p.status, p.expires_at);
                match core_like {
                    (Status::Invalid, _) => "INVALID",
                    (Status::Valid, exp) if n >= exp => "STALE",
                    _ => "VALID",
                }
            }
            // Absence is not permission: a registered circuit with no passport
            // is not a healthy one.
            None => "NONE",
        };
        if status == "INVALID" {
            affected += 1;
        }
        println!(
            "{:<18} {:<12} {:>5}  {:<9} {}",
            truncate(&c.name, 18),
            &c.circuit_hash[..12],
            deps,
            status,
            c.repository
        );
    }
    println!(
        "\n{} registered · {} invalid · {} clean",
        circuits.len(),
        affected,
        circuits.len() - affected
    );
    Ok(())
}

fn registry_deps(path: &Path) -> Result<()> {
    let store = Store::open(path)?;
    // The CLI runs for an operator; the public surface is the export.
    let deps = store.known_dependencies(Audience::Operator)?;
    if deps.is_empty() {
        println!("registry is empty");
        return Ok(());
    }
    println!("{:<28} {:<12} {:>8}", "DEPENDENCY", "VERSION", "CIRCUITS");
    for (name, version, count) in &deps {
        println!("{:<28} {:<12} {:>8}", truncate(name, 28), version, count);
    }
    println!("\n{} distinct dependencies", deps.len());
    Ok(())
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n - 1])
    }
}

/// Dump everything the control room renders.
///
/// The UI is a client of this, not a second source of truth: it displays what
/// the CLI computed and nothing it computed itself. That is deliberate — a
/// dashboard that derives its own numbers can show something the system does
/// not actually believe.
fn export(registry: &Path, exercises: &Path, out: &Path, operator: bool) -> Result<()> {
    let store = Store::open(registry)?;
    let audience = if operator { Audience::Operator } else { Audience::Public };
    let n = now();

    let mut circuits = Vec::new();
    for c in store.circuits()? {
        let p = store.latest_passport(&c.circuit_hash)?;
        // Absence is not permission: a registered circuit with no passport is
        // reported as NONE, never as healthy.
        let (status, capability, sequence, expires_at, issuer) = match &p {
            Some(p) => (
                match (p.status, p.expires_at) {
                    (Status::Invalid, _) => "INVALID",
                    (Status::Valid, e) if n >= e => "STALE",
                    _ => "VALID",
                },
                p.capability.tier(),
                p.sequence,
                p.expires_at,
                p.issuer.clone(),
            ),
            None => ("NONE", 0, 0, 0, String::new()),
        };
        // An embargoed circuit must not appear in a public export at all.
        if !operator && p.as_ref().map(|p| p.disclosure) != Some(DisclosureState::Public) {
            continue;
        }
        circuits.push(serde_json::json!({
            "circuitHash": c.circuit_hash,
            "name": c.name,
            "repository": c.repository,
            "commit": c.commit,
            "proofSystem": c.proof_system,
            "dependencies": store.dependency_count(&c.circuit_hash)?,
            "status": status,
            "capability": capability,
            "sequence": sequence.to_string(),
            "expiresAt": expires_at.to_string(),
            "issuer": issuer,
        }));
    }

    let mut deps = Vec::new();
    for (name, version, count) in store.known_dependencies(audience)? {
        let imp = project(&store, &name, &version, audience)?;
        deps.push(serde_json::json!({
            "name": name,
            "version": version,
            "circuits": count,
            "affected": imp.affected.len(),
            "clean": imp.clean.len(),
            "advisories": imp.advisories,
        }));
    }

    let mut records = Vec::new();
    let mut dispatches = Vec::new();
    if exercises.is_dir() {
        let mut files: Vec<_> = std::fs::read_dir(exercises)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        files.sort();
        for f in files {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&std::fs::read(&f)?) {
                // The directory holds more than one kind of record. An exercise
                // has an id and events; a Hyperlane dispatch does not. Mixing
                // them would hand the interface a shape it cannot render.
                if v.get("kind").and_then(|k| k.as_str()) == Some("HYPERLANE_DISPATCH") {
                    dispatches.push(v);
                } else if v.get("exercise_id").is_some() && v.get("events").is_some() {
                    records.push(v);
                }
            }
        }
    }

    let state = serde_json::json!({
        "generatedAt": n.to_string(),
        "audience": if operator { "OPERATOR" } else { "PUBLIC" },
        "circuits": circuits,
        "dependencies": deps,
        "exercises": records,
        "hyperlaneDispatches": dispatches,
        "deployments": {
            "solanaDevnet": {
                "passportProgram": "CkDhRfJRiGEa3kgnEUEvCBgyht62MTkDD6e754DLtB2",
                "consumerProgram": "BAcrrJYj5Y5DfcqnHgDwm5rhJUJdowZh25NvFLJAUzSW",
                "hyperlaneMailbox": "5yM5YrrzHCrp4ZPLKN9Y2eUAqEWsTbBqaorgbngQcR54"
            }
        },
        "measured": { "consumerCheckCu": 1520, "publishCu": 11550, "registerIssuerCu": 7406 }
    });

    if let Some(d) = out.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(out, serde_json::to_vec_pretty(&state)?)?;
    println!(
        "exported {} circuits · {} dependencies · {} exercises  ({})",
        state["circuits"].as_array().map_or(0, |a| a.len()),
        state["dependencies"].as_array().map_or(0, |a| a.len()),
        state["exercises"].as_array().map_or(0, |a| a.len()),
        if operator { "OPERATOR view" } else { "PUBLIC view" }
    );
    println!("written  {}", out.display());
    Ok(())
}

fn verify(passport: &Path, evidence: Option<&Path>, at: i64) -> Result<()> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let signed: SignedPassport = serde_json::from_slice(
        &std::fs::read(passport).with_context(|| format!("reading {}", passport.display()))?,
    )?;

    let declared = bs58::decode(&signed.issuer_id)
        .into_vec()
        .context("issuer_id is not valid base58")?;
    if declared != signed.core.issuer {
        return Err(anyhow!(
            "issuer mismatch: issuer_id does not match the key bound in the signed core"
        ));
    }

    let vk = VerifyingKey::from_bytes(&signed.core.issuer).context("issuer key is not valid")?;
    vk.verify(
        &signed.core.signing_preimage(),
        &Signature::from_bytes(&signed.signature),
    )
    .map_err(|_| anyhow!("signature does not verify"))?;
    println!("signature ok ({})", signed.issuer_id);

    if let Some(path) = evidence {
        let mut ev: Evidence = serde_json::from_slice(
            &std::fs::read(path).with_context(|| format!("reading {}", path.display()))?,
        )?;
        ev.normalize();
        if ev.hash()? != signed.core.evidence_hash {
            return Err(anyhow!("evidence_hash does not match the supplied evidence"));
        }
        if ev.circuit_hash() != signed.core.circuit_hash {
            return Err(anyhow!("circuit_hash does not match the supplied evidence"));
        }
        println!("evidence  binds ({} deps)", ev.dependencies.len());
    }

    if !signed.core.within_clock_bounds(at) {
        return Err(anyhow!("passport is outside acceptable clock bounds"));
    }

    let effective = signed.core.status_at(at);
    println!("sequence  {}", signed.core.sequence);
    println!("issued    {}", signed.core.issued_at);
    println!("expires   {}", signed.core.expires_at);
    println!("evaluated {}", at);
    println!("effective {:?}", effective);

    if effective != EffectiveStatus::Valid {
        std::process::exit(2);
    }
    Ok(())
}
