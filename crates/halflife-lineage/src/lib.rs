//! Dependency intelligence — the P4A layer.
//!
//! Resolve a circuit's dependency closure, match every registry package against
//! published advisories, and produce the [`Evidence`] a passport is issued over.
//!
//! No model is involved anywhere here, and none ever should be. This path must
//! stay reproducible offline by anyone.

pub mod advisory;
pub mod lockfile;

use anyhow::{Context, Result};
use halflife_core::{Capability, Dependency, DisclosureState, Evidence, Method, Subject};
use serde::Deserialize;
use std::path::Path;

/// A circuit's declared identity, read from `halflife.toml`.
#[derive(Debug, Deserialize)]
pub struct TargetConfig {
    pub subject: SubjectConfig,
}

#[derive(Debug, Deserialize)]
pub struct SubjectConfig {
    pub name: String,
    pub repository: String,
    pub commit: String,
    pub proof_system: String,
}

/// Where advisory data came from. Recorded in the evidence, because provenance
/// is part of what a passport attests.
pub enum AdvisorySource<'a> {
    /// Live query against OSV.
    Osv,
    /// A pinned snapshot, so results are reproducible without a network.
    Snapshot(&'a Path),
}

impl AdvisorySource<'_> {
    fn label(&self) -> String {
        match self {
            AdvisorySource::Osv => "osv.dev/v1/querybatch".into(),
            AdvisorySource::Snapshot(p) => format!("snapshot:{}", p.display()),
        }
    }
}

pub struct Closure {
    pub evidence: Evidence,
    /// Packages resolved from a registry, and therefore matchable against
    /// advisories at all. Path and workspace members are excluded.
    pub registry_count: usize,
}

/// Resolve `target` into evidence: closure, advisory hits, methodology.
///
/// `target` must contain `halflife.toml` and `Cargo.lock`. We read the lockfile
/// rather than invoking cargo: the lockfile is the closure that actually
/// shipped, and reading it needs neither a toolchain nor a network fetch.
pub fn resolve(
    target: &Path,
    source: AdvisorySource<'_>,
    disclosure: DisclosureState,
) -> Result<Closure> {
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

    let advisories = match source {
        AdvisorySource::Osv => advisory::lookup_online(&registry)?,
        AdvisorySource::Snapshot(p) => advisory::lookup_offline(p)?,
    };

    let dependencies: Vec<Dependency> = packages
        .iter()
        .map(|p| Dependency {
            name: p.name.clone(),
            version: p.version.clone(),
            checksum: p.checksum.clone(),
            // A path or workspace member has no registry identity, so no
            // advisory can be matched against it. Recording an empty list is
            // honest; omitting the package would hide it from the closure.
            advisories: if p.from_registry {
                advisories
                    .get(&advisory::key(&p.name, &p.version))
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
        advisory_source: source.label(),
        dependencies,
    };
    evidence.normalize();

    Ok(Closure {
        evidence,
        registry_count: registry.len(),
    })
}
