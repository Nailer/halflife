# Solana programs

A **separate Cargo workspace** from the host crates, deliberately.

Solana's SBF toolchain pins rustc/cargo 1.84, while the host crates (rusqlite,
ureq, clap) pull dependencies requiring edition 2024 and rustc 1.85+. A shared
lockfile forces one side onto versions the other cannot build. Two workspaces,
two lockfiles, no negotiation.

`Cargo.lock` here carries deliberate downgrades to stay within the SBF
toolchain's reach:

| Crate | Pinned | Reason |
|---|---|---|
| `proc-macro-crate` | 3.2.0 | 3.5.0 pulls `toml_edit` → `toml_datetime` 1.x, which needs edition 2024 |
| `zeroize` | 1.8.1 | 1.9.0 needs edition 2024 |
| `indexmap` | 2.7.1 | 2.14.2 needs edition 2024 |
| `unicode-segmentation` | 1.12.0 | 1.13.3 requires rustc 1.85 |

Do not run `cargo update` here without rebuilding. `anchor build` is the check.

## Programs

- **`halflife-passport`** — the registry. Verifies the issuer's off-chain
  ed25519 signature over the canonical 125-byte core via the sigverify
  precompile, so publication is permissionless and issuers never need SOL or a
  Solana keypair. Enforces sequence monotonicity, issuer revocation, and clock
  bounds.
- **`halflife-consumer`** — a reference consumer that *refuses to proceed*.
  Takes the passport as a read-only account, so concurrent consumers do not
  contend on shared state.

## Build

```bash
cd solana && anchor build
```
