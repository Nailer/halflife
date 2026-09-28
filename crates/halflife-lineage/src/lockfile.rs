//! Cargo.lock parsing.
//!
//! We read the lockfile rather than invoking `cargo metadata` on purpose: the
//! lockfile is the resolved closure, it is what actually shipped, and reading
//! it needs neither a toolchain nor a network fetch for the target project.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Lockfile {
    #[serde(default)]
    package: Vec<LockPackage>,
}

#[derive(Debug, Deserialize)]
struct LockPackage {
    name: String,
    version: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    checksum: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: String,
    pub checksum: Option<String>,
    /// False for path/workspace members, which have no registry source and so
    /// cannot be matched against a registry advisory.
    pub from_registry: bool,
}

pub fn parse(path: &Path) -> Result<Vec<ResolvedPackage>> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading lockfile {}", path.display()))?;
    let lock: Lockfile =
        toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;

    let mut out: Vec<ResolvedPackage> = lock
        .package
        .into_iter()
        .map(|p| ResolvedPackage {
            from_registry: p
                .source
                .as_deref()
                .map(|s| s.starts_with("registry+"))
                .unwrap_or(false),
            name: p.name,
            version: p.version,
            checksum: p.checksum,
        })
        .collect();

    out.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    out.dedup_by(|a, b| a.name == b.name && a.version == b.version);
    Ok(out)
}
