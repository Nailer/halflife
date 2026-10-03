//! The registry: registered circuits, their resolved closures, their passports,
//! and the reverse index from a dependency to everything that depends on it.
//!
//! This is the layer that answers what the advisory could not. GHSA-ww9q-8r59-xv46
//! lists its affected crates and then says *"and any dependents thereof"* —
//! and nothing enumerates them. [`Store::dependents`] is that enumeration.
//!
//! Deliberately local and boring: SQLite, one file, no service. Projection reads
//! it through a port so it stays pure and testable.

use anyhow::{Context, Result};
use halflife_core::{Capability, DisclosureState, Evidence, SignedPassport, Status};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

const SCHEMA: &str = r#"
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS circuits (
    circuit_hash  TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    repository    TEXT NOT NULL,
    commit_hash   TEXT NOT NULL,
    proof_system  TEXT NOT NULL,
    registered_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS dependencies (
    circuit_hash  TEXT NOT NULL REFERENCES circuits(circuit_hash) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    version       TEXT NOT NULL,
    checksum      TEXT,
    PRIMARY KEY (circuit_hash, name, version)
);

-- The reverse index. Without it, "what depends on halo2_gadgets 0.4.0" is a
-- full scan; with it, it is a lookup.
CREATE INDEX IF NOT EXISTS idx_dependencies_reverse
    ON dependencies(name, version);

CREATE TABLE IF NOT EXISTS advisory_hits (
    circuit_hash TEXT NOT NULL REFERENCES circuits(circuit_hash) ON DELETE CASCADE,
    dep_name     TEXT NOT NULL,
    dep_version  TEXT NOT NULL,
    advisory_id  TEXT NOT NULL,
    PRIMARY KEY (circuit_hash, dep_name, dep_version, advisory_id)
);

CREATE INDEX IF NOT EXISTS idx_advisory_hits_id
    ON advisory_hits(advisory_id);

CREATE TABLE IF NOT EXISTS passports (
    circuit_hash  TEXT NOT NULL,
    issuer        TEXT NOT NULL,
    sequence      INTEGER NOT NULL,
    status        TEXT NOT NULL,
    capability    INTEGER NOT NULL,
    issued_at     INTEGER NOT NULL,
    expires_at    INTEGER NOT NULL,
    disclosure    TEXT NOT NULL,
    passport_json TEXT NOT NULL,
    -- NULL while a finding is embargoed and the evidence is withheld. The
    -- passport still verifies standalone; only the causal detail is absent.
    evidence_json TEXT,
    PRIMARY KEY (circuit_hash, issuer, sequence)
);
"#;

pub struct Store {
    conn: Connection,
}

#[derive(Debug, Clone)]
pub struct CircuitRow {
    pub circuit_hash: String,
    pub name: String,
    pub repository: String,
    pub commit: String,
    pub proof_system: String,
    pub registered_at: i64,
}

#[derive(Debug, Clone)]
pub struct PassportRow {
    pub circuit_hash: String,
    pub issuer: String,
    pub sequence: u64,
    pub status: Status,
    pub capability: Capability,
    pub issued_at: i64,
    pub expires_at: i64,
    pub disclosure: DisclosureState,
}

/// One circuit reached by a dependency, with the advisories that reached it.
#[derive(Debug, Clone)]
pub struct Dependent {
    pub circuit: CircuitRow,
    pub advisories: Vec<String>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        let conn = Connection::open(path)
            .with_context(|| format!("opening registry {}", path.display()))?;
        conn.execute_batch(SCHEMA).context("applying schema")?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    /// Register a circuit with its closure and the passport issued over it.
    ///
    /// Re-registering the same circuit replaces its closure, because a closure
    /// is a fact about one build and re-resolving it means it changed. The
    /// passport history is append-only and is never replaced.
    pub fn register(
        &mut self,
        evidence: &Evidence,
        signed: &SignedPassport,
        now: i64,
    ) -> Result<String> {
        let circuit_hash = hex::encode(signed.core.circuit_hash);
        let tx = self.conn.transaction()?;

        tx.execute(
            "INSERT INTO circuits
               (circuit_hash, name, repository, commit_hash, proof_system, registered_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(circuit_hash) DO UPDATE SET
               name = excluded.name,
               repository = excluded.repository,
               commit_hash = excluded.commit_hash,
               proof_system = excluded.proof_system",
            params![
                circuit_hash,
                evidence.subject.name,
                evidence.subject.repository,
                evidence.subject.commit,
                evidence.subject.proof_system,
                now
            ],
        )?;

        tx.execute(
            "DELETE FROM dependencies WHERE circuit_hash = ?1",
            params![circuit_hash],
        )?;
        tx.execute(
            "DELETE FROM advisory_hits WHERE circuit_hash = ?1",
            params![circuit_hash],
        )?;

        for d in &evidence.dependencies {
            tx.execute(
                "INSERT INTO dependencies (circuit_hash, name, version, checksum)
                 VALUES (?1, ?2, ?3, ?4)",
                params![circuit_hash, d.name, d.version, d.checksum],
            )?;
            for a in &d.advisories {
                tx.execute(
                    "INSERT INTO advisory_hits
                       (circuit_hash, dep_name, dep_version, advisory_id)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![circuit_hash, d.name, d.version, a],
                )?;
            }
        }

        // Evidence is withheld while embargoed. The passport row still carries
        // everything a consumer needs; only the causal detail is absent.
        let evidence_json = if evidence.is_public() {
            Some(serde_json::to_string(evidence)?)
        } else {
            None
        };

        tx.execute(
            "INSERT OR REPLACE INTO passports
               (circuit_hash, issuer, sequence, status, capability,
                issued_at, expires_at, disclosure, passport_json, evidence_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                circuit_hash,
                signed.issuer_id,
                signed.core.sequence as i64,
                format!("{:?}", signed.core.status).to_uppercase(),
                signed.core.capability.tier(),
                signed.core.issued_at,
                signed.core.expires_at,
                format!("{:?}", evidence.disclosure).to_uppercase(),
                serde_json::to_string(signed)?,
                evidence_json
            ],
        )?;

        tx.commit()?;
        Ok(circuit_hash)
    }

    pub fn circuits(&self) -> Result<Vec<CircuitRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT circuit_hash, name, repository, commit_hash, proof_system, registered_at
             FROM circuits ORDER BY name, circuit_hash",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(CircuitRow {
                    circuit_hash: r.get(0)?,
                    name: r.get(1)?,
                    repository: r.get(2)?,
                    commit: r.get(3)?,
                    proof_system: r.get(4)?,
                    registered_at: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// The latest passport for a circuit, by sequence.
    pub fn latest_passport(&self, circuit_hash: &str) -> Result<Option<PassportRow>> {
        let row = self
            .conn
            .query_row(
                "SELECT circuit_hash, issuer, sequence, status, capability,
                        issued_at, expires_at, disclosure
                 FROM passports WHERE circuit_hash = ?1
                 ORDER BY sequence DESC LIMIT 1",
                params![circuit_hash],
                |r| {
                    let status: String = r.get(3)?;
                    let disclosure: String = r.get(7)?;
                    let cap: u8 = r.get(4)?;
                    Ok(PassportRow {
                        circuit_hash: r.get(0)?,
                        issuer: r.get(1)?,
                        sequence: r.get::<_, i64>(2)? as u64,
                        status: if status == "INVALID" {
                            Status::Invalid
                        } else {
                            Status::Valid
                        },
                        capability: Capability::from_tier(cap).unwrap_or(Capability::C0),
                        issued_at: r.get(5)?,
                        expires_at: r.get(6)?,
                        disclosure: if disclosure == "PUBLIC" {
                            DisclosureState::Public
                        } else {
                            DisclosureState::Embargoed
                        },
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    /// **The reverse query.** Everything registered whose closure contains this
    /// exact dependency — the enumeration the advisory could not provide.
    pub fn dependents(&self, name: &str, version: &str) -> Result<Vec<Dependent>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.circuit_hash, c.name, c.repository, c.commit_hash,
                    c.proof_system, c.registered_at
             FROM dependencies d
             JOIN circuits c ON c.circuit_hash = d.circuit_hash
             WHERE d.name = ?1 AND d.version = ?2
             ORDER BY c.name",
        )?;
        let circuits = stmt
            .query_map(params![name, version], |r| {
                Ok(CircuitRow {
                    circuit_hash: r.get(0)?,
                    name: r.get(1)?,
                    repository: r.get(2)?,
                    commit: r.get(3)?,
                    proof_system: r.get(4)?,
                    registered_at: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut out = Vec::with_capacity(circuits.len());
        for circuit in circuits {
            let mut st = self.conn.prepare(
                "SELECT advisory_id FROM advisory_hits
                 WHERE circuit_hash = ?1 AND dep_name = ?2 AND dep_version = ?3
                 ORDER BY advisory_id",
            )?;
            let advisories = st
                .query_map(params![circuit.circuit_hash, name, version], |r| r.get(0))?
                .collect::<Result<Vec<String>, _>>()?;
            out.push(Dependent {
                circuit,
                advisories,
            });
        }
        Ok(out)
    }

    /// Every distinct `name@version` an audience may see.
    ///
    /// Audience-filtered in the query, like every other projection path. An
    /// earlier version was not, and leaked: a fixture's own root package name
    /// appears in its closure, so listing dependencies unfiltered published the
    /// names of embargoed circuits even though the circuit list itself hid
    /// them. Filtering must cover every surface, not the obvious one.
    pub fn known_dependencies(&self, audience: Audience) -> Result<Vec<(String, String, usize)>> {
        let public_only = i64::from(audience.is_public());
        let mut stmt = self.conn.prepare(
            "WITH latest AS (
                 SELECT circuit_hash, disclosure,
                        ROW_NUMBER() OVER (
                            PARTITION BY circuit_hash ORDER BY sequence DESC
                        ) AS rn
                 FROM passports
             )
             SELECT d.name, d.version, COUNT(*)
             FROM dependencies d
             JOIN circuits c ON c.circuit_hash = d.circuit_hash
             LEFT JOIN latest l ON l.circuit_hash = c.circuit_hash AND l.rn = 1
             WHERE ?1 = 0 OR COALESCE(l.disclosure, 'EMBARGOED') = 'PUBLIC'
             GROUP BY d.name, d.version ORDER BY d.name, d.version",
        )?;
        let rows = stmt
            .query_map(params![public_only], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? as usize))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn dependency_count(&self, circuit_hash: &str) -> Result<usize> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM dependencies WHERE circuit_hash = ?1",
            params![circuit_hash],
            |r| r.get::<_, i64>(0),
        )? as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halflife_core::{Dependency, Method, PassportCore, Subject, PASSPORT_VERSION};

    fn evidence(name: &str, commit: &str, dep_ver: &str, advisories: Vec<&str>) -> Evidence {
        let mut e = Evidence {
            evidence_version: 1,
            subject: Subject {
                name: name.into(),
                repository: format!("https://example.test/{name}"),
                commit: commit.into(),
                proof_system: "halo2".into(),
            },
            capability: Capability::C1,
            disclosure: DisclosureState::Public,
            methods: vec![Method {
                id: "closure.resolve".into(),
                description: "d".into(),
            }],
            advisory_source: "test".into(),
            dependencies: vec![Dependency {
                name: "halo2_gadgets".into(),
                version: dep_ver.into(),
                checksum: None,
                advisories: advisories.into_iter().map(String::from).collect(),
            }],
        };
        e.normalize();
        e
    }

    fn signed(e: &Evidence, seq: u64) -> SignedPassport {
        SignedPassport {
            core: PassportCore {
                version: PASSPORT_VERSION,
                circuit_hash: e.circuit_hash(),
                issuer: [1u8; 32],
                sequence: seq,
                capability: e.capability,
                status: e.derive_status(),
                issued_at: 1_000,
                expires_at: 2_000,
                evidence_hash: e.hash().unwrap(),
                advisory_count: e.advisory_count() as u16,
            },
            issuer_id: "TestIssuer".into(),
            signature: [0u8; 64],
        }
    }

    #[test]
    fn reverse_index_enumerates_dependents() {
        let mut s = Store::open_in_memory().unwrap();
        let a = evidence("circuit-a", "aaa", "0.4.0", vec!["GHSA-ww9q-8r59-xv46"]);
        let b = evidence("circuit-b", "bbb", "0.4.0", vec!["GHSA-ww9q-8r59-xv46"]);
        let c = evidence("circuit-c", "ccc", "0.5.0", vec![]);
        for e in [&a, &b, &c] {
            s.register(e, &signed(e, 1), 0).unwrap();
        }

        let hit = s.dependents("halo2_gadgets", "0.4.0").unwrap();
        assert_eq!(hit.len(), 2);
        assert_eq!(hit[0].advisories, vec!["GHSA-ww9q-8r59-xv46"]);

        // The healthy version reaches exactly one circuit, with no advisory.
        let clean = s.dependents("halo2_gadgets", "0.5.0").unwrap();
        assert_eq!(clean.len(), 1);
        assert!(clean[0].advisories.is_empty());

        // A version nothing uses reaches nothing — no false impact.
        assert!(s.dependents("halo2_gadgets", "0.9.9").unwrap().is_empty());
    }

    #[test]
    fn embargoed_evidence_is_withheld_but_the_passport_is_kept() {
        let mut s = Store::open_in_memory().unwrap();
        let mut e = evidence("secret", "sss", "0.4.0", vec!["GHSA-ww9q-8r59-xv46"]);
        e.disclosure = DisclosureState::Embargoed;
        let hash = s.register(&e, &signed(&e, 1), 0).unwrap();

        let stored: Option<String> = s
            .conn
            .query_row(
                "SELECT evidence_json FROM passports WHERE circuit_hash = ?1",
                params![hash],
                |r| r.get(0),
            )
            .unwrap();
        assert!(stored.is_none(), "embargoed evidence must not be stored");

        // The passport itself survives, so a consumer can still decide.
        assert!(s.latest_passport(&hash).unwrap().is_some());
    }

    #[test]
    fn re_registering_replaces_the_closure() {
        let mut s = Store::open_in_memory().unwrap();
        let old = evidence("c", "same-commit", "0.4.0", vec!["GHSA-ww9q-8r59-xv46"]);
        let h = s.register(&old, &signed(&old, 1), 0).unwrap();
        assert_eq!(s.dependents("halo2_gadgets", "0.4.0").unwrap().len(), 1);

        // Same subject, upgraded dependency: a different circuit identity, so
        // the old one is untouched and the new one stands alone.
        let new = evidence("c", "same-commit", "0.5.0", vec![]);
        let h2 = s.register(&new, &signed(&new, 2), 1).unwrap();
        assert_ne!(h, h2);
        assert_eq!(s.dependency_count(&h2).unwrap(), 1);
    }

    #[test]
    fn latest_passport_is_by_sequence() {
        let mut s = Store::open_in_memory().unwrap();
        let e = evidence("c", "x", "0.5.0", vec![]);
        s.register(&e, &signed(&e, 1), 0).unwrap();
        s.register(&e, &signed(&e, 7), 0).unwrap();
        s.register(&e, &signed(&e, 3), 0).unwrap();
        let h = hex::encode(e.circuit_hash());
        assert_eq!(s.latest_passport(&h).unwrap().unwrap().sequence, 7);
    }
}

// ---------------------------------------------------------------------------
// Projection port
// ---------------------------------------------------------------------------

use halflife_core::Audience;
use halflife_projection::{Reached, RegistrySource};

/// Disclosure is enforced **in the query**, not after it.
///
/// Both statements below carry the authorization predicate, so rows a public
/// caller may not see are never assembled in the first place. Filtering a
/// complete result set afterwards would leak through counts and response shape
/// even with the rows removed.
///
/// A circuit with no passport has no disclosure state, and `COALESCE` treats
/// that as `EMBARGOED` — fail closed, never open.
impl RegistrySource for Store {
    fn reached_by(
        &self,
        name: &str,
        version: &str,
        audience: Audience,
    ) -> anyhow::Result<Vec<Reached>> {
        let public_only = i64::from(audience.is_public());
        let mut stmt = self.conn.prepare(
            "WITH latest AS (
                 SELECT circuit_hash, disclosure,
                        ROW_NUMBER() OVER (
                            PARTITION BY circuit_hash ORDER BY sequence DESC
                        ) AS rn
                 FROM passports
             )
             SELECT c.circuit_hash, c.name, c.repository
             FROM dependencies d
             JOIN circuits c ON c.circuit_hash = d.circuit_hash
             LEFT JOIN latest l
                    ON l.circuit_hash = c.circuit_hash AND l.rn = 1
             WHERE d.name = ?1 AND d.version = ?2
               AND (?3 = 0 OR COALESCE(l.disclosure, 'EMBARGOED') = 'PUBLIC')
             ORDER BY c.name",
        )?;
        let base = stmt
            .query_map(params![name, version, public_only], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut out = Vec::with_capacity(base.len());
        for (circuit_hash, cname, repository) in base {
            let mut st = self.conn.prepare(
                "SELECT advisory_id FROM advisory_hits
                 WHERE circuit_hash = ?1 AND dep_name = ?2 AND dep_version = ?3
                 ORDER BY advisory_id",
            )?;
            let advisories = st
                .query_map(params![circuit_hash, name, version], |r| r.get(0))?
                .collect::<Result<Vec<String>, _>>()?;
            out.push(Reached {
                circuit_hash,
                name: cname,
                repository,
                advisories,
            });
        }
        Ok(out)
    }

    fn visible_circuits(&self, audience: Audience) -> anyhow::Result<usize> {
        let public_only = i64::from(audience.is_public());
        Ok(self.conn.query_row(
            "WITH latest AS (
                 SELECT circuit_hash, disclosure,
                        ROW_NUMBER() OVER (
                            PARTITION BY circuit_hash ORDER BY sequence DESC
                        ) AS rn
                 FROM passports
             )
             SELECT COUNT(*)
             FROM circuits c
             LEFT JOIN latest l
                    ON l.circuit_hash = c.circuit_hash AND l.rn = 1
             WHERE ?1 = 0 OR COALESCE(l.disclosure, 'EMBARGOED') = 'PUBLIC'",
            params![public_only],
            |r| r.get::<_, i64>(0),
        )? as usize)
    }
}

#[cfg(test)]
mod port_tests {
    use super::*;
    use halflife_core::{Dependency, Method, PassportCore, Subject, PASSPORT_VERSION};
    use halflife_projection::project;

    fn ev(name: &str, dep_ver: &str, advs: Vec<&str>, disclosure: DisclosureState) -> Evidence {
        let mut e = Evidence {
            evidence_version: 1,
            subject: Subject {
                name: name.into(),
                repository: format!("https://example.test/{name}"),
                commit: name.into(),
                proof_system: "halo2".into(),
            },
            capability: Capability::C1,
            disclosure,
            methods: vec![Method { id: "m".into(), description: "d".into() }],
            advisory_source: "test".into(),
            dependencies: vec![Dependency {
                name: "halo2_gadgets".into(),
                version: dep_ver.into(),
                checksum: None,
                advisories: advs.into_iter().map(String::from).collect(),
            }],
        };
        e.normalize();
        e
    }

    fn sp(e: &Evidence) -> SignedPassport {
        SignedPassport {
            core: PassportCore {
                version: PASSPORT_VERSION,
                circuit_hash: e.circuit_hash(),
                issuer: [1u8; 32],
                sequence: 1,
                capability: e.capability,
                status: e.derive_status(),
                issued_at: 1_000,
                expires_at: 2_000,
                evidence_hash: e.hash().unwrap(),
                advisory_count: e.advisory_count() as u16,
            },
            issuer_id: "T".into(),
            signature: [0u8; 64],
        }
    }

    fn seeded() -> Store {
        let mut s = Store::open_in_memory().unwrap();
        for e in [
            ev("pub-affected", "0.4.0", vec!["GHSA-ww9q-8r59-xv46"], DisclosureState::Public),
            ev("pub-clean", "0.5.0", vec![], DisclosureState::Public),
            ev("secret-affected", "0.4.0", vec!["GHSA-ww9q-8r59-xv46"], DisclosureState::Embargoed),
            // An embargoed circuit whose closure contains its own name, which is
            // exactly the shape that leaked.
            {
                let mut e = ev("secret-root", "0.4.0", vec![], DisclosureState::Embargoed);
                e.dependencies.push(halflife_core::Dependency {
                    name: "secret-root".into(),
                    version: "9.9.9".into(),
                    checksum: None,
                    advisories: vec![],
                });
                e.normalize();
                e
            },
        ] {
            s.register(&e, &sp(&e), 0).unwrap();
        }
        s
    }

    #[test]
    fn public_projection_excludes_embargoed_rows_at_the_query() {
        let s = seeded();
        let p = project(&s, "halo2_gadgets", "0.4.0", Audience::Public).unwrap();
        assert_eq!(p.affected.len(), 1);
        assert_eq!(p.affected[0].name, "pub-affected");

        let o = project(&s, "halo2_gadgets", "0.4.0", Audience::Operator).unwrap();
        assert_eq!(o.affected.len(), 2);
    }

    #[test]
    fn public_counts_never_include_withheld_circuits() {
        let s = seeded();
        // Two public, two embargoed. The public count must not hint at the
        // other two in any way, including by arithmetic.
        assert_eq!(s.visible_circuits(Audience::Public).unwrap(), 2);
        assert_eq!(s.visible_circuits(Audience::Operator).unwrap(), 4);
    }

    #[test]
    fn healthy_version_reaches_circuits_without_affecting_them() {
        let s = seeded();
        let p = project(&s, "halo2_gadgets", "0.5.0", Audience::Public).unwrap();
        assert_eq!(p.reached(), 1);
        assert!(p.is_clear());
    }

    #[test]
    fn the_dependency_index_does_not_leak_embargoed_names() {
        // The leak this caught in production: a fixture's own root package name
        // appears in its closure, so an unfiltered dependency listing published
        // the names of embargoed circuits while the circuit list correctly hid
        // them. Every surface has to filter, not just the obvious one.
        let s = seeded();
        let pub_names: Vec<String> = s
            .known_dependencies(Audience::Public)
            .unwrap()
            .into_iter()
            .map(|(n, _, _)| n)
            .collect();
        assert!(
            !pub_names.iter().any(|n| n.contains("secret")),
            "embargoed circuit name leaked through the dependency index: {pub_names:?}"
        );

        let op_names: Vec<String> = s
            .known_dependencies(Audience::Operator)
            .unwrap()
            .into_iter()
            .map(|(n, _, _)| n)
            .collect();
        assert!(op_names.len() >= pub_names.len());
    }

    #[test]
    fn a_circuit_with_no_passport_fails_closed() {
        let mut s = Store::open_in_memory().unwrap();
        let e = ev("orphan", "0.4.0", vec!["GHSA-ww9q-8r59-xv46"], DisclosureState::Public);
        s.register(&e, &sp(&e), 0).unwrap();
        s.conn.execute("DELETE FROM passports", []).unwrap();
        // No disclosure state means no authorization to show it.
        assert_eq!(s.visible_circuits(Audience::Public).unwrap(), 0);
        let p = project(&s, "halo2_gadgets", "0.4.0", Audience::Public).unwrap();
        assert!(p.affected.is_empty());
    }
}
