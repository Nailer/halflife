//! `halflife` — produce and inspect Security Passports.
//!
//! v0.1 does exactly one deterministic thing: resolve a circuit's dependency
//! closure, match it against published advisories, and emit a signed passport.
//! No model is involved, and the tool claims capability C1 only.

mod keys;
mod lockfile;
mod osv;
mod vectors;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use halflife_core::{
    Capability, Dependency, DisclosureState, EffectiveStatus, Evidence, Method, PassportCore,
    SignedPassport, Subject, PASSPORT_VERSION,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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
    /// Scan a circuit's dependency closure and emit a signed passport.
    Scan {
        /// Directory holding halflife.toml and Cargo.lock.
        target: PathBuf,
        #[arg(short, long, default_value = "issuer.json")]
        issuer: PathBuf,
        /// Output directory for passport.json and evidence.json.
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Use a pinned advisory snapshot instead of querying OSV.
        #[arg(long)]
        offline: bool,
        /// Snapshot path used with --offline.
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
    },
    /// Generate or verify the conformance vectors that pin the wire format.
    Vectors {
        #[command(subcommand)]
        op: VectorOp,
    },
    /// Verify a passport's signature and bindings, and resolve its status now.
    Verify {
        passport: PathBuf,
        /// Evidence file, to confirm evidence_hash actually binds.
        #[arg(short, long)]
        evidence: Option<PathBuf>,
        /// Evaluate the clock at this unix time instead of now.
        #[arg(long)]
        at: Option<i64>,
    },
}

#[derive(Subcommand)]
enum VectorOp {
    /// Regenerate vectors from the fixed test key.
    Generate {
        #[arg(short, long, default_value = "fixtures/vectors.json")]
        out: PathBuf,
    },
    /// Check that this implementation reproduces every vector byte-for-byte.
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

#[derive(Debug, Deserialize)]
struct TargetConfig {
    subject: SubjectConfig,
}

#[derive(Debug, Deserialize)]
struct SubjectConfig {
    name: String,
    repository: String,
    commit: String,
    proof_system: String,
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
            issuer,
            out,
            offline,
            snapshot,
            ttl,
            sequence,
            disclosure,
        } => {
            let out = out.unwrap_or_else(|| target.join("out"));
            scan(
                &target,
                &issuer,
                &out,
                offline,
                &snapshot,
                ttl,
                sequence,
                disclosure.into(),
            )
        }
        Cmd::Vectors { op } => match op {
            VectorOp::Generate { out } => vectors::generate(&out),
            VectorOp::Verify { path } => vectors::verify(&path),
        },
        Cmd::Verify {
            passport,
            evidence,
            at,
        } => verify(&passport, evidence.as_deref(), at.unwrap_or_else(now)),
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

#[allow(clippy::too_many_arguments)]
fn scan(
    target: &Path,
    issuer_path: &Path,
    out: &Path,
    offline: bool,
    snapshot: &Path,
    ttl: i64,
    sequence: u64,
    disclosure: DisclosureState,
) -> Result<()> {
    let cfg_path = target.join("halflife.toml");
    let cfg: TargetConfig = toml::from_str(
        &std::fs::read_to_string(&cfg_path)
            .with_context(|| format!("reading {}", cfg_path.display()))?,
    )
    .with_context(|| format!("parsing {}", cfg_path.display()))?;

    let packages = lockfile::parse(&target.join("Cargo.lock"))?;
    let registry: Vec<(String, String)> = packages
        .iter()
        .filter(|p| p.from_registry)
        .map(|p| (p.name.clone(), p.version.clone()))
        .collect();

    println!(
        "closure   {} packages ({} from registry)",
        packages.len(),
        registry.len()
    );

    let advisories = if offline {
        println!("advisory  snapshot {}", snapshot.display());
        osv::lookup_offline(snapshot)?
    } else {
        println!("advisory  osv.dev (crates.io)");
        osv::lookup_online(&registry)?
    };

    let dependencies: Vec<Dependency> = packages
        .iter()
        .map(|p| Dependency {
            name: p.name.clone(),
            version: p.version.clone(),
            checksum: p.checksum.clone(),
            advisories: if p.from_registry {
                advisories
                    .get(&osv::key(&p.name, &p.version))
                    .cloned()
                    .unwrap_or_default()
            } else {
                Vec::new()
            },
        })
        .collect();

    let mut evidence = Evidence {
        evidence_version: 1,
        subject: Subject {
            name: cfg.subject.name,
            repository: cfg.subject.repository,
            commit: cfg.subject.commit,
            proof_system: cfg.subject.proof_system,
        },
        capability: Capability::C1,
        disclosure,
        methods: vec![
            Method {
                id: "closure.resolve".into(),
                description: "Resolve the dependency closure from Cargo.lock".into(),
            },
            Method {
                id: "advisory.match".into(),
                description: "Match each registry package name@version against OSV".into(),
            },
        ],
        advisory_source: if offline {
            format!("snapshot:{}", snapshot.display())
        } else {
            "osv.dev/v1/querybatch".into()
        },
        dependencies,
    };
    evidence.normalize();

    let issued_at = now();
    let core = PassportCore {
        version: PASSPORT_VERSION,
        circuit_hash: evidence.circuit_hash(),
        issuer: keys::Issuer::load(issuer_path)?.pubkey_bytes(),
        sequence,
        capability: evidence.capability,
        status: evidence.derive_status(),
        issued_at,
        expires_at: issued_at + ttl,
        evidence_hash: evidence.hash()?,
        advisory_count: evidence.advisory_count().min(u16::MAX as usize) as u16,
    };

    let issuer = keys::Issuer::load(issuer_path)?;
    let signed = SignedPassport {
        signature: issuer.sign(&core.signing_preimage()),
        issuer_id: issuer.pubkey_b58(),
        core,
    };

    std::fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;
    std::fs::write(
        out.join("evidence.json"),
        serde_json::to_vec_pretty(&evidence)?,
    )?;
    std::fs::write(
        out.join("passport.json"),
        serde_json::to_vec_pretty(&signed)?,
    )?;

    let hits: Vec<&Dependency> = evidence
        .dependencies
        .iter()
        .filter(|d| !d.advisories.is_empty())
        .collect();

    println!("circuit   {}", hex::encode(signed.core.circuit_hash));
    println!("evidence  {}", hex::encode(signed.core.evidence_hash));
    println!("issuer    {}", signed.issuer_id);
    println!("capability C{}", signed.core.capability.tier());
    println!("sequence  {}", signed.core.sequence);
    println!("disclosure {:?}", disclosure);
    if hits.is_empty() {
        println!("findings  none");
    } else {
        for d in &hits {
            println!(
                "findings  {}@{}  {}",
                d.name,
                d.version,
                d.advisories.join(", ")
            );
        }
    }
    println!("status    {:?}", signed.core.status);
    println!("written   {}", out.display());
    Ok(())
}

fn verify(passport: &Path, evidence: Option<&Path>, at: i64) -> Result<()> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let signed: SignedPassport = serde_json::from_slice(
        &std::fs::read(passport).with_context(|| format!("reading {}", passport.display()))?,
    )?;

    // The advertised issuer id and the key inside the signed core must agree,
    // or a passport could name one issuer while being signed by another.
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

    let effective = signed.core.status_at(at);
    println!("issued    {}", signed.core.issued_at);
    println!("expires   {}", signed.core.expires_at);
    println!("evaluated {}", at);
    println!("effective {:?}", effective);

    // Exit non-zero on anything a consumer would block on, so this is usable in
    // CI and in the demo harness without parsing stdout.
    if effective != EffectiveStatus::Valid {
        std::process::exit(2);
    }
    Ok(())
}
