//! Issuer keys.
//!
//! Deliberately the Solana keypair file format — a JSON array of 64 bytes,
//! `secret || public`. An issuer identity is therefore just a Solana address,
//! and `solana-keygen` can produce one. Nothing here is Halflife-specific.

use anyhow::{anyhow, Context, Result};
use ed25519_dalek::{Signer, SigningKey};
use std::path::Path;

pub struct Issuer {
    signing: SigningKey,
}

impl Issuer {
    pub fn generate() -> Result<Self> {
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed).map_err(|e| anyhow!("gathering entropy: {e}"))?;
        Ok(Self {
            signing: SigningKey::from_bytes(&seed),
        })
    }

    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading keypair {}", path.display()))?;
        let bytes: Vec<u8> = serde_json::from_str(&raw)
            .with_context(|| format!("parsing keypair {}", path.display()))?;
        if bytes.len() != 64 {
            return Err(anyhow!(
                "expected a 64-byte Solana keypair, found {} bytes",
                bytes.len()
            ));
        }
        let seed: [u8; 32] = bytes[..32].try_into().unwrap();
        let signing = SigningKey::from_bytes(&seed);

        // A truncated or hand-edited file can carry a public half that does not
        // match the secret; signing with it would produce passports nobody can
        // verify against the advertised issuer id.
        if signing.verifying_key().to_bytes()[..] != bytes[32..] {
            return Err(anyhow!(
                "keypair is inconsistent: public half does not match the secret half"
            ));
        }
        Ok(Self { signing })
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let mut bytes = Vec::with_capacity(64);
        bytes.extend_from_slice(&self.signing.to_bytes());
        bytes.extend_from_slice(&self.signing.verifying_key().to_bytes());
        std::fs::write(path, serde_json::to_string(&bytes)?)
            .with_context(|| format!("writing keypair {}", path.display()))?;
        Ok(())
    }

    pub fn pubkey_bytes(&self) -> [u8; 32] {
        self.signing.verifying_key().to_bytes()
    }

    /// Base58 — the form a Solana address is normally written in.
    pub fn pubkey_b58(&self) -> String {
        bs58::encode(self.pubkey_bytes()).into_string()
    }

    pub fn sign(&self, msg: &[u8]) -> [u8; 64] {
        self.signing.sign(msg).to_bytes()
    }
}
