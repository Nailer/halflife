//! Halflife core: the Security Passport primitive.
//!
//! Two artifacts, deliberately separated:
//!
//! * [`Evidence`] — the full off-chain record (dependency closure, advisory hits,
//!   methodology). Large, human-auditable, hashed into a single 32-byte digest.
//! * [`PassportCore`] — the 117-byte on-chain record. Fixed width, no heap, no
//!   strings. This is what the Solana program stores and what consumers read in
//!   their hot path.
//!
//! The signature covers `PassportCore` only, over a domain-separated canonical
//! encoding, using ed25519 — so the exact bytes signed here are verifiable on
//! Solana via the ed25519 sigverify precompile, and an issuer id *is* a Solana
//! pubkey.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Prepended before signing so a Halflife signature can never be replayed as a
/// signature over some other protocol's message.
pub const SIGNING_DOMAIN: &[u8] = b"halflife-passport-v1";

pub const PASSPORT_VERSION: u8 = 1;

/// Wire size of a canonically encoded [`PassportCore`].
pub const PASSPORT_CORE_LEN: usize = 1 + 32 + 32 + 1 + 1 + 8 + 8 + 32 + 2;

// ---------------------------------------------------------------------------
// Capability ladder
// ---------------------------------------------------------------------------

/// What was actually *done* to the circuit — not how confident anyone feels.
///
/// A passport asserts a tier was executed. It never asserts the circuit is safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Capability {
    /// No automated verification performed.
    C0,
    /// Structural and dependency analysis: manifest closure, advisory matching.
    C1,
    /// Constraint consistency testing over the circuit's own constraint system.
    C2,
    /// Differential testing against a known-good reference implementation.
    C3,
    /// Adversarial soundness testing.
    C4,
    /// Independent re-verification under a newer methodology than the original.
    C5,
}

impl Capability {
    pub fn tier(self) -> u8 {
        match self {
            Capability::C0 => 0,
            Capability::C1 => 1,
            Capability::C2 => 2,
            Capability::C3 => 3,
            Capability::C4 => 4,
            Capability::C5 => 5,
        }
    }
}

/// Status as *asserted at issuance*.
///
/// Note what is absent: `STALE` is not issuable. Staleness is a function of the
/// reader's clock against `expires_at`, so a passport cannot claim to be fresh.
/// This is what makes suppression fail-safe — see [`PassportCore::status_at`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Status {
    Valid,
    Invalid,
}

impl Status {
    pub fn code(self) -> u8 {
        match self {
            Status::Valid => 1,
            Status::Invalid => 2,
        }
    }
}

/// What a consumer actually resolves to after applying its own clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum EffectiveStatus {
    Valid,
    Stale,
    Invalid,
}

// ---------------------------------------------------------------------------
// On-chain core
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassportCore {
    pub version: u8,
    /// Build-identity fingerprint of the circuit. See `docs/passport-spec.md`
    /// for the v1 derivation and its stated limits.
    #[serde(with = "hex32")]
    pub circuit_hash: [u8; 32],
    /// Issuer ed25519 public key — equivalently, a Solana pubkey.
    #[serde(with = "hex32")]
    pub issuer: [u8; 32],
    pub capability: Capability,
    pub status: Status,
    pub issued_at: i64,
    pub expires_at: i64,
    /// SHA-256 over the canonical encoding of the off-chain [`Evidence`].
    #[serde(with = "hex32")]
    pub evidence_hash: [u8; 32],
    /// Count of advisories matched. A cheap on-chain signal that avoids
    /// putting the advisory list itself on-chain.
    pub advisory_count: u16,
}

impl PassportCore {
    /// Fixed-width little-endian encoding. This is the byte layout the Solana
    /// program deserializes and the exact preimage that gets signed.
    pub fn canonical_bytes(&self) -> [u8; PASSPORT_CORE_LEN] {
        let mut out = [0u8; PASSPORT_CORE_LEN];
        let mut i = 0;
        out[i] = self.version;
        i += 1;
        out[i..i + 32].copy_from_slice(&self.circuit_hash);
        i += 32;
        out[i..i + 32].copy_from_slice(&self.issuer);
        i += 32;
        out[i] = self.capability.tier();
        i += 1;
        out[i] = self.status.code();
        i += 1;
        out[i..i + 8].copy_from_slice(&self.issued_at.to_le_bytes());
        i += 8;
        out[i..i + 8].copy_from_slice(&self.expires_at.to_le_bytes());
        i += 8;
        out[i..i + 32].copy_from_slice(&self.evidence_hash);
        i += 32;
        out[i..i + 2].copy_from_slice(&self.advisory_count.to_le_bytes());
        out
    }

    /// Domain-separated signing preimage.
    pub fn signing_preimage(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(SIGNING_DOMAIN.len() + PASSPORT_CORE_LEN);
        v.extend_from_slice(SIGNING_DOMAIN);
        v.extend_from_slice(&self.canonical_bytes());
        v
    }

    /// Resolve the status a consumer should act on, given its own clock.
    ///
    /// The ordering matters: an explicit `Invalid` always wins, and an expired
    /// passport degrades to `Stale` rather than remaining `Valid`. A consumer
    /// that never receives an update therefore blocks, so an actor who
    /// suppresses delivery cannot hold a circuit open.
    pub fn status_at(&self, now: i64) -> EffectiveStatus {
        match self.status {
            Status::Invalid => EffectiveStatus::Invalid,
            Status::Valid if now >= self.expires_at => EffectiveStatus::Stale,
            Status::Valid => EffectiveStatus::Valid,
        }
    }
}

// ---------------------------------------------------------------------------
// Off-chain evidence
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    /// Registry checksum from the lockfile, when present.
    pub checksum: Option<String>,
    /// Advisory identifiers matched against this exact name+version.
    pub advisories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subject {
    pub name: String,
    pub repository: String,
    pub commit: String,
    pub proof_system: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Method {
    pub id: String,
    pub description: String,
}

/// The auditable record. Field order here *is* the canonical order; the
/// dependency vector is sorted by (name, version) before hashing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub evidence_version: u8,
    pub subject: Subject,
    pub capability: Capability,
    pub methods: Vec<Method>,
    pub advisory_source: String,
    pub dependencies: Vec<Dependency>,
}

impl Evidence {
    /// Sort dependencies so the encoding is independent of discovery order.
    pub fn normalize(&mut self) {
        self.dependencies.sort();
        for d in &mut self.dependencies {
            d.advisories.sort();
            d.advisories.dedup();
        }
        self.methods.sort_by(|a, b| a.id.cmp(&b.id));
    }

    pub fn canonical_json(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn hash(&self) -> Result<[u8; 32], serde_json::Error> {
        Ok(sha256(&self.canonical_json()?))
    }

    /// Every advisory hit across the closure.
    pub fn advisory_count(&self) -> usize {
        self.dependencies.iter().map(|d| d.advisories.len()).sum()
    }

    /// Deterministic status derivation. No model, no judgement: a matched
    /// advisory in the closure invalidates the passport.
    pub fn derive_status(&self) -> Status {
        if self.advisory_count() > 0 {
            Status::Invalid
        } else {
            Status::Valid
        }
    }

    /// v1 circuit identity: a build fingerprint over the declared subject and
    /// the resolved dependency closure.
    ///
    /// This binds a passport to *how the circuit was built*, not to the
    /// constraint system it compiles to. Two builds with identical closures and
    /// commit collide by design; a source change that does not move any
    /// dependency is invisible to it. Binding to the verifying key is the v2
    /// intent and is deliberately not claimed here.
    pub fn circuit_hash(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"halflife-circuit-id-v1\0");
        h.update(self.subject.repository.as_bytes());
        h.update([0]);
        h.update(self.subject.commit.as_bytes());
        h.update([0]);
        h.update(self.subject.proof_system.as_bytes());
        h.update([0]);
        let mut deps: Vec<&Dependency> = self.dependencies.iter().collect();
        deps.sort();
        for d in deps {
            h.update(d.name.as_bytes());
            h.update(b"@");
            h.update(d.version.as_bytes());
            h.update([0]);
        }
        h.finalize().into()
    }
}

// ---------------------------------------------------------------------------
// Signed passport
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedPassport {
    pub core: PassportCore,
    /// Base58 ed25519 public key of the issuer, i.e. a Solana address.
    pub issuer_id: String,
    #[serde(with = "hex64")]
    pub signature: [u8; 64],
}

pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().into()
}

// ---------------------------------------------------------------------------
// hex serde helpers
// ---------------------------------------------------------------------------

macro_rules! hex_mod {
    ($name:ident, $n:expr) => {
        pub mod $name {
            use serde::{Deserialize, Deserializer, Serializer};
            pub fn serialize<S: Serializer>(v: &[u8; $n], s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&hex::encode(v))
            }
            pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; $n], D::Error> {
                let s = String::deserialize(d)?;
                let raw = hex::decode(&s).map_err(serde::de::Error::custom)?;
                let arr: [u8; $n] = raw
                    .try_into()
                    .map_err(|_| serde::de::Error::custom(concat!("expected ", $n, " bytes")))?;
                Ok(arr)
            }
        }
    };
}

hex_mod!(hex32, 32);
hex_mod!(hex64, 64);

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn core(status: Status, expires_at: i64) -> PassportCore {
        PassportCore {
            version: PASSPORT_VERSION,
            circuit_hash: [7u8; 32],
            issuer: [9u8; 32],
            capability: Capability::C1,
            status,
            issued_at: 1_000,
            expires_at,
            evidence_hash: [3u8; 32],
            advisory_count: 0,
        }
    }

    #[test]
    fn canonical_encoding_is_fixed_width() {
        assert_eq!(core(Status::Valid, 2_000).canonical_bytes().len(), 117);
        assert_eq!(PASSPORT_CORE_LEN, 117);
    }

    #[test]
    fn signing_preimage_is_domain_separated() {
        let c = core(Status::Valid, 2_000);
        assert!(c.signing_preimage().starts_with(SIGNING_DOMAIN));
    }

    #[test]
    fn expiry_degrades_to_stale_not_valid() {
        let c = core(Status::Valid, 2_000);
        assert_eq!(c.status_at(1_999), EffectiveStatus::Valid);
        assert_eq!(c.status_at(2_000), EffectiveStatus::Stale);
        assert_eq!(c.status_at(9_999), EffectiveStatus::Stale);
    }

    #[test]
    fn invalid_outranks_the_clock() {
        // An invalidated passport never reads as merely stale, at any time.
        let c = core(Status::Invalid, i64::MAX);
        assert_eq!(c.status_at(0), EffectiveStatus::Invalid);
        assert_eq!(c.status_at(i64::MAX), EffectiveStatus::Invalid);
    }

    #[test]
    fn evidence_hash_is_order_independent() {
        let mk = |deps: Vec<Dependency>| Evidence {
            evidence_version: 1,
            subject: Subject {
                name: "orchard".into(),
                repository: "https://github.com/zcash/orchard".into(),
                commit: "abc".into(),
                proof_system: "halo2".into(),
            },
            capability: Capability::C1,
            methods: vec![],
            advisory_source: "osv.dev".into(),
            dependencies: deps,
        };
        let d = |n: &str, v: &str| Dependency {
            name: n.into(),
            version: v.into(),
            checksum: None,
            advisories: vec![],
        };
        let mut a = mk(vec![d("b", "1"), d("a", "1")]);
        let mut b = mk(vec![d("a", "1"), d("b", "1")]);
        a.normalize();
        b.normalize();
        assert_eq!(a.hash().unwrap(), b.hash().unwrap());
        assert_eq!(a.circuit_hash(), b.circuit_hash());
    }

    #[test]
    fn a_matched_advisory_invalidates() {
        let mut e = Evidence {
            evidence_version: 1,
            subject: Subject {
                name: "orchard".into(),
                repository: "r".into(),
                commit: "c".into(),
                proof_system: "halo2".into(),
            },
            capability: Capability::C1,
            methods: vec![],
            advisory_source: "osv.dev".into(),
            dependencies: vec![Dependency {
                name: "halo2_gadgets".into(),
                version: "0.4.0".into(),
                checksum: None,
                advisories: vec!["GHSA-ww9q-8r59-xv46".into()],
            }],
        };
        e.normalize();
        assert_eq!(e.derive_status(), Status::Invalid);
        assert_eq!(e.advisory_count(), 1);
    }

    #[test]
    fn moving_one_dependency_version_changes_circuit_identity() {
        let mk = |ver: &str| Evidence {
            evidence_version: 1,
            subject: Subject {
                name: "orchard".into(),
                repository: "r".into(),
                commit: "c".into(),
                proof_system: "halo2".into(),
            },
            capability: Capability::C1,
            methods: vec![],
            advisory_source: "osv.dev".into(),
            dependencies: vec![Dependency {
                name: "halo2_gadgets".into(),
                version: ver.into(),
                checksum: None,
                advisories: vec![],
            }],
        };
        assert_ne!(mk("0.4.0").circuit_hash(), mk("0.5.0").circuit_hash());
    }
}
