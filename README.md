# Halflife

Security passports for zero-knowledge circuits.

A cryptographic audit tells you what someone knew then. A passport tells your
application what has actually been verified, at what capability, about the exact
build you are trusting — and lets that status travel.

## Status: v1.0.0 — wire format frozen

Deterministic path only. No model is involved anywhere in this repository, and
the tool claims capability **C1** — nothing above it.

- [x] Lineage: resolve a circuit's dependency closure and match it against advisories
- [x] Passport: 125-byte signed core + hash-bound off-chain evidence
- [x] Fail-safe expiry — `STALE` is derived by the reader, never issuable
- [x] `sequence`, supersession, forward-only issuer revocation, clock bounds
- [x] `decide()` — absence is not permission
- [x] Conformance vectors, reproduced by an independent implementation
- [x] Disclosure policy, published before the first scan
- [ ] Registry + reverse index
- [ ] Projection (`halflife impact`)
- [ ] Solana registry program
- [ ] Hyperlane propagation
- [ ] Constraint analysis (C2+)

## Quickstart

```bash
cargo build --release
./target/release/halflife keygen --out issuer.json
./target/release/halflife scan fixtures/orchard-vulnerable --out out/vulnerable
./target/release/halflife verify out/vulnerable/passport.json --evidence out/vulnerable/evidence.json
```

Or run the whole sequence: `./scripts/demo.sh`

Check the format against both implementations:

```bash
./target/release/halflife vectors verify fixtures/vectors.json
node packages/passport-ts/verify-vectors.mjs fixtures/vectors.json
```

## Why this exists

`halo2_gadgets` versions before 0.5.0 carried a missing copy constraint in
variable-base scalar multiplication — [GHSA-ww9q-8r59-xv46] — which allowed a
malicious prover to produce a valid proof for an under-constrained Orchard
Action. It shipped in May 2022 and was found in May 2026.

Two things follow, and both are checkable rather than rhetorical:

**RustSec has no entry for `halo2_gadgets`, `orchard` or `zcash_primitives`.**
So `cargo audit` returns clean on a tree pinned to the vulnerable versions. OSV
federates the GitHub advisory and does resolve them, which is why this tool
queries OSV.

**The advisory's affected set ends with "and any dependents thereof"** — and
nothing enumerates them. You cannot currently go from a deployed on-chain
program back to the crate versions that built its circuit, and nothing consumes
an advisory on-chain. A program cannot halt on a GHSA identifier.

That gap is the product.

## Fixtures

Two resolved closures using genuine crates.io versions and checksums, spanning
the real advisory boundary. The fix was a coordinated same-day release on
2026-06-03: `halo2_gadgets 0.5.0`, `orchard 0.14.0`, `zcash_primitives 0.28.0`.

| Fixture | halo2_gadgets | orchard | zcash_primitives | Result |
|---|---|---|---|---|
| `orchard-vulnerable` | 0.4.0 | 0.13.1 | 0.27.1 | INVALID |
| `orchard-patched` | 0.5.0 | 0.14.0 | 0.28.0 | VALID |

`--offline` uses `fixtures/osv-snapshot.json`, pinned from live OSV, so results
stay reproducible without a network round trip.

## Documentation

- [`docs/passport-spec.md`](docs/passport-spec.md) — wire format, signing, the fail-safe
- [`docs/capability-model.md`](docs/capability-model.md) — the C0–C5 ladder
- [`docs/disclosure-policy.md`](docs/disclosure-policy.md) — what we scan, what we publish, and the limits of embargo

## What this does not claim

Halflife does not prove a circuit is secure. It records what was tested, with
which capability, against which exact build, and makes that record
machine-readable and portable. Every stronger reading is wrong.

[GHSA-ww9q-8r59-xv46]: https://osv.dev/vulnerability/GHSA-ww9q-8r59-xv46
