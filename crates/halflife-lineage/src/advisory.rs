//! Advisory matching against OSV.
//!
//! OSV is used rather than RustSec because RustSec carries no entry for
//! `halo2_gadgets`, `orchard` or `zcash_primitives` — so `cargo audit` returns
//! clean on a tree pinned to the vulnerable versions. OSV federates the GitHub
//! advisory (GHSA-ww9q-8r59-xv46) and does resolve them.
//!
//! `--offline` swaps in a pinned snapshot so a demo never depends on a network
//! round trip, and so results are reproducible.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

const OSV_BATCH_URL: &str = "https://api.osv.dev/v1/querybatch";
const ECOSYSTEM: &str = "crates.io";

#[derive(Serialize)]
struct BatchQuery<'a> {
    queries: Vec<Query<'a>>,
}

#[derive(Serialize)]
struct Query<'a> {
    package: Package<'a>,
    version: &'a str,
}

#[derive(Serialize)]
struct Package<'a> {
    name: &'a str,
    ecosystem: &'a str,
}

#[derive(Deserialize)]
struct BatchResponse {
    #[serde(default)]
    results: Vec<QueryResult>,
}

#[derive(Deserialize, Default)]
struct QueryResult {
    #[serde(default)]
    vulns: Vec<Vuln>,
}

#[derive(Deserialize)]
struct Vuln {
    id: String,
}

/// Keyed `"name@version"` -> advisory ids.
pub type AdvisoryMap = BTreeMap<String, Vec<String>>;

pub fn key(name: &str, version: &str) -> String {
    format!("{name}@{version}")
}

/// Query OSV in batches. Returns only the entries that matched, so an empty map
/// means a clean closure.
pub fn lookup_online(pkgs: &[(String, String)]) -> Result<AdvisoryMap> {
    let mut map = AdvisoryMap::new();

    for chunk in pkgs.chunks(400) {
        let body = BatchQuery {
            queries: chunk
                .iter()
                .map(|(n, v)| Query {
                    package: Package {
                        name: n,
                        ecosystem: ECOSYSTEM,
                    },
                    version: v,
                })
                .collect(),
        };

        let resp: BatchResponse = ureq::post(OSV_BATCH_URL)
            .set("Content-Type", "application/json")
            .send_json(serde_json::to_value(&body)?)
            .context("OSV batch query failed — retry, or use --offline")?
            .into_json()
            .context("decoding OSV response")?;

        for (i, res) in resp.results.iter().enumerate() {
            if res.vulns.is_empty() {
                continue;
            }
            let (n, v) = &chunk[i];
            let mut ids: Vec<String> = res.vulns.iter().map(|x| x.id.clone()).collect();
            ids.sort();
            ids.dedup();
            map.insert(key(n, v), ids);
        }
    }
    Ok(map)
}

pub fn lookup_offline(path: &Path) -> Result<AdvisoryMap> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading advisory snapshot {}", path.display()))?;
    let map: AdvisoryMap =
        serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
    Ok(map)
}
