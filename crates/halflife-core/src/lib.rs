//! Halflife core: the Security Passport primitive.
//!
//! Two artifacts, deliberately separated:
//!
//! * [`Evidence`] — the full off-chain record (dependency closure, advisory hits,
//!   methodology, disclosure state). Large, human-auditable, hashed into a
//!   single 32-byte digest.
//! * [`PassportCore`] — the 125-byte on-chain record. Fixed width, no heap, no
//!   strings. This is what the Solana program stores and what consumers read in
//!   their hot path.
//!
//! The signature covers `PassportCore` only, over a domain-separated canonical
//! encoding, using ed25519 — so the exact bytes signed here are verifiable on
//! Solana via the ed25519 sigverify precompile, and an issuer id *is* a Solana
//! pubkey.
//!
//! # The invariant
//!
//! **No consumer may treat an unrefreshed security claim as valid
//! indefinitely.** Everything below is machinery for holding that true. The
//! entry point that enforces it is [`decide`], which takes an `Option` on
//! purpose: absence is not permission.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Prepended before signing so a Halflife signature can never be replayed as a
/// signature over some other protocol's message.
pub const SIGNING_DOMAIN: &[u8] = b"halflife-passport-v1";

pub const PASSPORT_VERSION: u8 = 1;

/// Wire size of a canonically encoded [`PassportCore`].
pub const PASSPORT_CORE_LEN: usize = 1 + 32 + 32 + 8 + 1 + 1 + 8 + 8 + 32 + 2;

/// How far ahead of the reader's clock an `issued_at` may sit before the
/// passport is rejected. Without a bound, a compromised issuer could mint a
/// claim that outlives its own revocation.
pub const MAX_CLOCK_SKEW_SECS: i64 = 300;

// ---------------------------------------------------------------------------
// Capability ladder
// ---------------------------------------------------------------------------

/// What was actually *done* to the circuit — not how confident anyone feels.
///
/// A passport asserts a tier was executed. It never asserts the circuit is safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

    pub fn from_tier(t: u8) -> Option<Self> {
        Some(match t {
            0 => Capability::C0,
            1 => Capability::C1,
            2 => Capability::C2,
            3 => Capability::C3,
            4 => Capability::C4,
            5 => Capability::C5,
            _ => return None,
        })
    }
}

// ---------------------------------------------------------------------------
// Disclosure
// ---------------------------------------------------------------------------

/// Whether the finding behind a passport may be discussed publicly.
///
/// Lives on [`Evidence`], **never** on [`PassportCore`]. A core field would make
/// every passport announce that something is embargoed, which is precisely the
/// signal an embargo exists to suppress.
///
/// See `docs/disclosure-policy.md` for the limits of the mechanism — in short,
/// embargo via staleness buys quiet, not secrecy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DisclosureState {
    /// The underlying finding is published. Causal relationships may be shown.
    Public,
    /// Under embargo. Projection must be computed from authorized evidence
    /// only — never built in full and filtered afterwards.
    Embargoed,
}

impl Default for DisclosureState {
    fn default() -> Self {
        // Fail closed: evidence with no explicit disclosure state is treated as
        // embargoed, so a missing field cannot leak a relationship.
        DisclosureState::Embargoed
    }
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

/// Status as *asserted at issuance*.
///
/// Note what is absent: `STALE` is not issuable. Staleness is a function of the
/// reader's clock against `expires_at`, so a passport cannot claim to be fresh.
/// This is what makes suppression fail-safe.
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

/// What a consumer actually resolves to after applying its own clock and policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum EffectiveStatus {
    Valid,
    Stale,
    Invalid,
}

// ---------------------------------------------------------------------------
// Issuers
// ---------------------------------------------------------------------------

/// An issuer's lifecycle as a consumer sees it.
///
/// Revocation is forward-only: passports signed before `revoked_at` remain
/// cryptographically verifiable, because history should not become unreadable,
/// but they stop being *acceptable* from that moment on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuerRecord {
    pub key: [u8; 32],
    pub registered_at: i64,
    /// `None` while active.
    pub revoked_at: Option<i64>,
    /// Set when this key superseded another, for audit trails.
    pub rotated_from: Option<[u8; 32]>,
}

impl IssuerRecord {
    pub fn active(key: [u8; 32], registered_at: i64) -> Self {
        Self {
            key,
            registered_at,
            revoked_at: None,
            rotated_from: None,
        }
    }

    /// Whether a passport issued at `issued_at` is acceptable when read at `now`.
    pub fn accepts(&self, issued_at: i64, now: i64) -> bool {
        if issued_at < self.registered_at {
            return false;
        }
        match self.revoked_at {
            // Once revoked, nothing from this key is accepted forward, whatever
            // it claims about when it was issued.
            Some(r) => now < r && issued_at < r,
            None => true,
        }
    }
}

// ---------------------------------------------------------------------------
// On-chain core
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassportCore {
    pub version: u8,
    /// Build-identity fingerprint of the circuit. See `docs/passport-spec.md`
    /// for the v1 derivation and its stated limits.
    #[serde(with = "hex32")]
    pub circuit_hash: [u8; 32],
    /// Issuer ed25519 public key — equivalently, a Solana pubkey.
    #[serde(with = "hex32")]
    pub issuer: [u8; 32],
    /// Monotonic per `(circuit_hash, issuer)`. Defeats replay of a superseded
    /// passport, including onto a second chain that has not seen the newer one.
    #[serde(with = "str_u64")]
    pub sequence: u64,
    pub capability: Capability,
    pub status: Status,
    #[serde(with = "str_i64")]
    pub issued_at: i64,
    #[serde(with = "str_i64")]
    pub expires_at: i64,
    /// SHA-256 over the canonical encoding of the off-chain [`Evidence`].
    #[serde(with = "hex32")]
    pub evidence_hash: [u8; 32],
    /// Count of advisories matched. A cheap on-chain signal, never a policy
    /// input: it cannot distinguish three hits on one advisory from three
    /// separate advisories.
    pub advisory_count: u16,
}

impl PassportCore {
    /// Fixed-width little-endian encoding. This is the byte layout the Solana
    /// program deserializes and the exact preimage that gets signed.
    pub fn canonical_bytes(&self) -> [u8; PASSPORT_CORE_LEN] {
        let mut out = [0u8; PASSPORT_CORE_LEN];
        let mut i = 0;
        macro_rules! put {
            ($src:expr) => {{
                let s = $src;
                out[i..i + s.len()].copy_from_slice(&s);
                i += s.len();
            }};
        }
        put!([self.version]);
        put!(self.circuit_hash);
        put!(self.issuer);
        put!(self.sequence.to_le_bytes());
        put!([self.capability.tier()]);
        put!([self.status.code()]);
        put!(self.issued_at.to_le_bytes());
        put!(self.expires_at.to_le_bytes());
        put!(self.evidence_hash);
        put!(self.advisory_count.to_le_bytes());
        debug_assert_eq!(i, PASSPORT_CORE_LEN);
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

    /// Whether `self` replaces `other`.
    ///
    /// Only comparable within one `(circuit_hash, issuer)` pair — a passport
    /// from a different issuer never supersedes, it merely coexists, and the
    /// consumer's quorum policy decides what that means.
    pub fn supersedes(&self, other: &PassportCore) -> bool {
        self.circuit_hash == other.circuit_hash
            && self.issuer == other.issuer
            && self.sequence > other.sequence
    }

    /// A passport whose `issued_at` sits beyond tolerance in the reader's future
    /// is rejected outright.
    pub fn within_clock_bounds(&self, now: i64) -> bool {
        self.issued_at <= now.saturating_add(MAX_CLOCK_SKEW_SECS)
            && self.expires_at > self.issued_at
    }
}

// ---------------------------------------------------------------------------
// Consumer decision — the API that makes the invariant hard to violate
// ---------------------------------------------------------------------------

/// What a consumer requires before it will proceed.
#[derive(Debug, Clone)]
pub struct Policy {
    pub expected_circuit: [u8; 32],
    pub min_capability: Capability,
    /// Issuers this consumer will accept, in lifecycle form.
    pub trusted_issuers: Vec<IssuerRecord>,
}

/// Why a consumer blocked. Carried so an operator can tell a delivery problem
/// from a finding — `Stale` is not an incident, and must never be rendered as
/// one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Block {
    /// No passport at all. **Absence is not permission.**
    Missing,
    Stale,
    Invalid,
    WrongCircuit,
    InsufficientCapability,
    UntrustedIssuer,
    ClockOutOfBounds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Block(Block),
}

/// Resolve a consumer decision.
///
/// The `Option` is the point. The unsafe integration — *check the passport if
/// one exists* — inverts the invariant and hands a free pass to anyone who can
/// prevent registration. Taking an `Option` and returning [`Block::Missing`] for
/// `None` makes that mistake require deliberate effort rather than inattention.
pub fn decide(passport: Option<&PassportCore>, policy: &Policy, now: i64) -> Decision {
    let p = match passport {
        Some(p) => p,
        None => return Decision::Block(Block::Missing),
    };

    if p.circuit_hash != policy.expected_circuit {
        return Decision::Block(Block::WrongCircuit);
    }
    if !p.within_clock_bounds(now) {
        return Decision::Block(Block::ClockOutOfBounds);
    }
    let issuer_ok = policy
        .trusted_issuers
        .iter()
        .any(|i| i.key == p.issuer && i.accepts(p.issued_at, now));
    if !issuer_ok {
        return Decision::Block(Block::UntrustedIssuer);
    }
    if p.capability < policy.min_capability {
        return Decision::Block(Block::InsufficientCapability);
    }
    match p.status_at(now) {
        EffectiveStatus::Valid => Decision::Allow,
        EffectiveStatus::Stale => Decision::Block(Block::Stale),
        EffectiveStatus::Invalid => Decision::Block(Block::Invalid),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Subject {
    pub name: String,
    pub repository: String,
    pub commit: String,
    pub proof_system: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Method {
    pub id: String,
    pub description: String,
}

/// The auditable record. Field order here *is* the canonical order; the
/// dependency vector is sorted by (name, version) before hashing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Evidence {
    pub evidence_version: u8,
    pub subject: Subject,
    pub capability: Capability,
    #[serde(default)]
    pub disclosure: DisclosureState,
    pub methods: Vec<Method>,
    pub advisory_source: String,
    pub dependencies: Vec<Dependency>,
}

impl Evidence {
    /// Sort so the encoding is independent of discovery order.
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

    /// Whether causal detail from this evidence may appear on a public surface.
    pub fn is_public(&self) -> bool {
        self.disclosure == DisclosureState::Public
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

/// 64-bit integers are emitted as JSON **strings**, strictly.
///
/// A JSON number is a float64 everywhere it matters, so any value above 2^53
/// silently corrupts in a JavaScript reader — a `sequence` of `u64::MAX` comes
/// back as 2^64. This was caught by the conformance vectors before the format
/// froze, which is what they are for.
///
/// Deserialization deliberately refuses numbers rather than accepting them when
/// they happen to be small. Leniency here would let a producer emit a corrupted
/// number and have it pass everywhere except at the boundary, which is the
/// failure mode we are trying to make impossible.
macro_rules! str_int {
    ($name:ident, $t:ty) => {
        pub mod $name {
            use serde::{de::Error, Deserialize, Deserializer, Serializer};
            pub fn serialize<S: Serializer>(v: &$t, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&v.to_string())
            }
            pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<$t, D::Error> {
                let s = String::deserialize(d).map_err(|_| {
                    D::Error::custom(concat!(
                        stringify!($t),
                        " must be a JSON string, not a number (float64 loses precision above 2^53)"
                    ))
                })?;
                s.parse::<$t>().map_err(D::Error::custom)
            }
        }
    };
}

str_int!(str_u64, u64);
str_int!(str_i64, i64);

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn core(status: Status, expires_at: i64) -> PassportCore {
        PassportCore {
            version: PASSPORT_VERSION,
            circuit_hash: [7u8; 32],
            issuer: [9u8; 32],
            sequence: 1,
            capability: Capability::C1,
            status,
            issued_at: 1_000,
            expires_at,
            evidence_hash: [3u8; 32],
            advisory_count: 0,
        }
    }

    fn policy() -> Policy {
        Policy {
            expected_circuit: [7u8; 32],
            min_capability: Capability::C1,
            trusted_issuers: vec![IssuerRecord::active([9u8; 32], 0)],
        }
    }

    #[test]
    fn canonical_encoding_is_fixed_width() {
        assert_eq!(PASSPORT_CORE_LEN, 125);
        assert_eq!(core(Status::Valid, 2_000).canonical_bytes().len(), 125);
    }

    #[test]
    fn signing_preimage_is_domain_separated() {
        assert!(core(Status::Valid, 2_000)
            .signing_preimage()
            .starts_with(SIGNING_DOMAIN));
    }

    #[test]
    fn sequence_is_bound_into_the_signature() {
        // If sequence were outside the signed core, an old passport could be
        // renumbered and replayed.
        let a = core(Status::Valid, 2_000);
        let mut b = a.clone();
        b.sequence = 2;
        assert_ne!(a.canonical_bytes(), b.canonical_bytes());
    }

    #[test]
    fn expiry_degrades_to_stale_not_valid() {
        let c = core(Status::Valid, 2_000);
        assert_eq!(c.status_at(1_999), EffectiveStatus::Valid);
        assert_eq!(c.status_at(2_000), EffectiveStatus::Stale);
    }

    #[test]
    fn invalid_outranks_the_clock() {
        let c = core(Status::Invalid, i64::MAX);
        assert_eq!(c.status_at(0), EffectiveStatus::Invalid);
        assert_eq!(c.status_at(i64::MAX), EffectiveStatus::Invalid);
    }

    #[test]
    fn absence_is_not_permission() {
        assert_eq!(
            decide(None, &policy(), 1_500),
            Decision::Block(Block::Missing)
        );
    }

    #[test]
    fn decision_covers_each_policy_dimension() {
        let p = policy();
        let ok = core(Status::Valid, 2_000);
        assert_eq!(decide(Some(&ok), &p, 1_500), Decision::Allow);

        let mut wrong = ok.clone();
        wrong.circuit_hash = [1u8; 32];
        assert_eq!(
            decide(Some(&wrong), &p, 1_500),
            Decision::Block(Block::WrongCircuit)
        );

        let mut low = ok.clone();
        low.capability = Capability::C0;
        assert_eq!(
            decide(Some(&low), &p, 1_500),
            Decision::Block(Block::InsufficientCapability)
        );

        let mut other = ok.clone();
        other.issuer = [42u8; 32];
        assert_eq!(
            decide(Some(&other), &p, 1_500),
            Decision::Block(Block::UntrustedIssuer)
        );

        assert_eq!(decide(Some(&ok), &p, 2_500), Decision::Block(Block::Stale));
    }

    #[test]
    fn a_passport_from_the_future_is_rejected() {
        let c = core(Status::Valid, 9_000);
        // issued_at is 1000; reading at 500 puts it beyond tolerance.
        assert!(!c.within_clock_bounds(500));
        assert_eq!(
            decide(Some(&c), &policy(), 500),
            Decision::Block(Block::ClockOutOfBounds)
        );
        // Within skew tolerance it is fine.
        assert!(c.within_clock_bounds(1_000 - MAX_CLOCK_SKEW_SECS));
    }

    #[test]
    fn supersession_requires_same_circuit_and_issuer() {
        let a = core(Status::Valid, 2_000);
        let mut newer = a.clone();
        newer.sequence = 2;
        assert!(newer.supersedes(&a));
        assert!(!a.supersedes(&newer));

        let mut other_issuer = newer.clone();
        other_issuer.issuer = [42u8; 32];
        assert!(!other_issuer.supersedes(&a));
    }

    #[test]
    fn revocation_is_forward_only() {
        let mut rec = IssuerRecord::active([9u8; 32], 0);
        rec.revoked_at = Some(5_000);
        // Issued and read before revocation: accepted.
        assert!(rec.accepts(1_000, 4_000));
        // Read after revocation: refused, even though it was issued before.
        assert!(!rec.accepts(1_000, 6_000));
        // Issued after revocation: never.
        assert!(!rec.accepts(6_000, 6_500));
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
            disclosure: DisclosureState::Public,
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
    fn sixty_four_bit_fields_round_trip_as_strings() {
        // The defect the conformance vectors caught: a JSON number cannot carry
        // u64::MAX without corruption in a float64 reader.
        let mut c = core(Status::Valid, 2_000);
        c.sequence = u64::MAX;
        c.issued_at = i64::MIN;
        c.expires_at = i64::MAX;
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("\"18446744073709551615\""), "{json}");
        let back: PassportCore = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn a_json_number_is_refused_for_a_64_bit_field() {
        let json = r#"{"version":1,
            "circuit_hash":"0707070707070707070707070707070707070707070707070707070707070707",
            "issuer":"0909090909090909090909090909090909090909090909090909090909090909",
            "sequence":1,"capability":"C1","status":"VALID",
            "issued_at":"1000","expires_at":"2000",
            "evidence_hash":"0303030303030303030303030303030303030303030303030303030303030303",
            "advisory_count":0}"#;
        assert!(serde_json::from_str::<PassportCore>(json).is_err());
    }

    #[test]
    fn missing_disclosure_state_defaults_to_embargoed() {
        // A field absent from older evidence must fail closed, never leak.
        let json = r#"{
            "evidence_version":1,
            "subject":{"name":"x","repository":"r","commit":"c","proof_system":"halo2"},
            "capability":"C1",
            "methods":[],
            "advisory_source":"osv.dev",
            "dependencies":[]
        }"#;
        let e: Evidence = serde_json::from_str(json).unwrap();
        assert_eq!(e.disclosure, DisclosureState::Embargoed);
        assert!(!e.is_public());
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
            disclosure: DisclosureState::Public,
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
            disclosure: DisclosureState::Public,
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
