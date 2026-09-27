//! Conformance vectors — the specification's teeth.
//!
//! Any implementation that cannot reproduce every vector byte-for-byte is not
//! an implementation. Vectors are generated from a fixed, published test seed so
//! they are reproducible by anyone, and they pin three things independently:
//! the canonical encoding, the signing preimage, and the resulting signature.

use anyhow::{anyhow, Context, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier};
use halflife_core::{
    Capability, PassportCore, Status, PASSPORT_CORE_LEN, PASSPORT_VERSION, SIGNING_DOMAIN,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Published, deterministic, and useless for anything but conformance. Never
/// use this key to issue a real passport.
const TEST_SEED: [u8; 32] = [
    0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c, 0xc4,
    0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae, 0x7f, 0x60,
];

#[derive(Serialize, Deserialize)]
struct VectorFile {
    spec: String,
    note: String,
    test_seed_hex: String,
    issuer_pubkey_hex: String,
    core_len: usize,
    signing_domain: String,
    vectors: Vec<Vector>,
}

#[derive(Serialize, Deserialize)]
struct Vector {
    name: String,
    core: PassportCore,
    /// Hex of the 125-byte canonical encoding.
    canonical_hex: String,
    /// Hex of `SIGNING_DOMAIN || canonical`.
    preimage_hex: String,
    /// Hex of the ed25519 signature over the preimage.
    signature_hex: String,
}

fn cases(issuer: [u8; 32]) -> Vec<(String, PassportCore)> {
    let base = PassportCore {
        version: PASSPORT_VERSION,
        circuit_hash: [0x11; 32],
        issuer,
        sequence: 1,
        capability: Capability::C1,
        status: Status::Valid,
        issued_at: 1_700_000_000,
        expires_at: 1_700_086_400,
        evidence_hash: [0x22; 32],
        advisory_count: 0,
    };

    let mut invalid = base.clone();
    invalid.status = Status::Invalid;
    invalid.sequence = 2;
    invalid.advisory_count = 3;

    let mut high_cap = base.clone();
    high_cap.capability = Capability::C5;
    high_cap.sequence = 7;

    // Boundary values, because off-by-one encoding bugs live here.
    let mut extremes = base.clone();
    extremes.sequence = u64::MAX;
    extremes.issued_at = i64::MIN;
    extremes.expires_at = i64::MAX;
    extremes.advisory_count = u16::MAX;
    extremes.capability = Capability::C0;

    vec![
        ("valid_c1_seq1".into(), base),
        ("invalid_three_advisories".into(), invalid),
        ("valid_c5_seq7".into(), high_cap),
        ("boundary_extremes".into(), extremes),
    ]
}

pub fn generate(out: &Path) -> Result<()> {
    let sk = SigningKey::from_bytes(&TEST_SEED);
    let issuer = sk.verifying_key().to_bytes();

    let vectors = cases(issuer)
        .into_iter()
        .map(|(name, core)| {
            let canonical = core.canonical_bytes();
            let preimage = core.signing_preimage();
            Vector {
                name,
                canonical_hex: hex::encode(canonical),
                preimage_hex: hex::encode(&preimage),
                signature_hex: hex::encode(sk.sign(&preimage).to_bytes()),
                core,
            }
        })
        .collect();

    let file = VectorFile {
        spec: "halflife-passport-v1".into(),
        note: "Generated from a published test seed. This key must never issue a real passport."
            .into(),
        test_seed_hex: hex::encode(TEST_SEED),
        issuer_pubkey_hex: hex::encode(issuer),
        core_len: PASSPORT_CORE_LEN,
        signing_domain: String::from_utf8_lossy(SIGNING_DOMAIN).into_owned(),
        vectors,
    };

    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    std::fs::write(out, serde_json::to_vec_pretty(&file)?)
        .with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {} vectors to {}", file.vectors.len(), out.display());
    Ok(())
}

pub fn verify(path: &Path) -> Result<()> {
    let file: VectorFile = serde_json::from_slice(
        &std::fs::read(path).with_context(|| format!("reading {}", path.display()))?,
    )?;

    if file.core_len != PASSPORT_CORE_LEN {
        return Err(anyhow!(
            "core length disagrees: vectors say {}, this build says {}",
            file.core_len,
            PASSPORT_CORE_LEN
        ));
    }

    let sk = SigningKey::from_bytes(&TEST_SEED);
    let vk = sk.verifying_key();
    if hex::encode(vk.to_bytes()) != file.issuer_pubkey_hex {
        return Err(anyhow!("test key disagrees with the vector file"));
    }

    for v in &file.vectors {
        let canonical = hex::encode(v.core.canonical_bytes());
        if canonical != v.canonical_hex {
            return Err(anyhow!(
                "{}: canonical encoding differs\n  expected {}\n  got      {}",
                v.name,
                v.canonical_hex,
                canonical
            ));
        }
        let preimage = v.core.signing_preimage();
        if hex::encode(&preimage) != v.preimage_hex {
            return Err(anyhow!("{}: signing preimage differs", v.name));
        }

        // Re-signing must be byte-identical (ed25519 is deterministic) and the
        // recorded signature must verify. Both, because either alone can pass
        // while the other is broken.
        let resigned = hex::encode(sk.sign(&preimage).to_bytes());
        if resigned != v.signature_hex {
            return Err(anyhow!("{}: signature differs on re-signing", v.name));
        }
        let raw: [u8; 64] = hex::decode(&v.signature_hex)
            .context("signature is not hex")?
            .try_into()
            .map_err(|_| anyhow!("{}: signature is not 64 bytes", v.name))?;
        vk.verify(&preimage, &Signature::from_bytes(&raw))
            .map_err(|_| anyhow!("{}: recorded signature does not verify", v.name))?;

        println!("  ok  {}", v.name);
    }

    println!(
        "{} vectors reproduced byte-for-byte ({} bytes per core)",
        file.vectors.len(),
        file.core_len
    );
    Ok(())
}
