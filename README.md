<div align="center">

# Halflife

**A use-by date for cryptographic security.**

Security passports for zero-knowledge circuits — signed, capability-versioned,
and enforceable on-chain.

[**Live control room →**](https://halflife-control-room.vercel.app)

</div>

---

## The problem

In 2026, four cryptographic systems that had each passed professional review
failed. Every flaw was years old. Every one was found with AI assistance.

| | | |
|---|---|---|
| 23 Mar | Zcash Sprout | proof verification skipped entirely · 4 years |
| 29 May | Zcash Orchard | counterfeiting circuit · 4 years |
| 27 Jul | Aztec V5 | proving system soundness |
| 30 Jul | Coldcard | $116M drained · 5 years |

The detail that turns this into a finding rather than an anecdote: the
researcher who found the Orchard bug had **already run the same framework over
the same code with the previous model generation, and it found nothing.** The
code never changed. The capability frontier moved.

**So a security audit is a perishable good.** "Audited in 2024" describes a
search conducted with tools that no longer represent the state of the art.

Zero-knowledge systems are the most exposed class of software to this, for a
reason specific to them: **their bugs are silent.** A broken contract reverts
and shows up in a block explorer. An under-constrained circuit produces a
valid-looking proof for an invalid statement. Nothing fails.

## What Halflife does

Gives a circuit a **passport**: a 125-byte signed claim about what was actually
verified, at what capability, about which exact build — and when that stops
being current. Programs read it and refuse to proceed without one.

```
dependency closure  →  advisory match  →  signed passport  →  Solana registry
                                                                    │
                                                              Hyperlane
                                                                    │
                                                       destination registry
                                                                    │
                                                        consumer: ALLOW / BLOCK
```

## The property that matters

**Suppressing delivery produces the safe outcome.**

`STALE` is derived by the reader against its own clock — a passport cannot
assert its own freshness. So a consumer that stops receiving updates blocks.
There is no path where losing the network means continuing to trust old state.

Demonstrated live on devnet, with nothing published:

```
relayer-censorship    41s containment    PassportStale
                      exactly one on-chain write — the baseline
```

## Try it

```bash
./scripts/verify-all.sh
```

Thirty checks across every layer. Every number it prints is measured during the
run, not quoted.

```
30 passed   0 failed   0 skipped
```

## Measured, not claimed

| | |
|---|---|
| Consumer passport check | **1,520 CU** — 0.76% of a default transaction budget |
| Publish a signed passport | 11,550 CU |
| Containment, dependency compromise | 4s |
| Containment, relayer censorship | 41s, with no invalidation published |

Reproduce with `cargo run --release -p halflife-bench`.

## Deployed

| | |
|---|---|
| Control room | https://halflife-control-room.vercel.app |
| Passport program | [`CkDhRfJRi…`](https://explorer.solana.com/address/CkDhRfJRiGEa3kgnEUEvCBgyht62MTkDD6e754DLtB2?cluster=devnet) · Solana devnet |
| Consumer program | [`BAcrrJYj5…`](https://explorer.solana.com/address/BAcrrJYj5Y5DfcqnHgDwm5rhJUJdowZh25NvFLJAUzSW?cluster=devnet) · Solana devnet |

## Why these three ecosystems

Each is load-bearing. Remove any one and the system does not work.

**Zcash** is the real cryptographic target. `halo2_gadgets` before 0.5.0 carried
a missing copy constraint in variable-base scalar multiplication —
[GHSA-ww9q-8r59-xv46](https://osv.dev/vulnerability/GHSA-ww9q-8r59-xv46) — which
let a malicious prover produce a valid proof for an under-constrained Orchard
Action. The fixtures span that real boundary with genuine checksums.

**Solana** is canonical state and enforcement. Passport accounts are **read-only
in the consumer path**, so Sealevel never serialises them and any number of
programs can check the same passport in one slot. That is why a check can sit
inside a hot path — and the 1,520 CU measurement is the argument, not the
convenience.

**Hyperlane** makes security state portable. The dispatch is a CPI signed by a
PDA under the passport program, because the mailbox *requires* that: a program
cannot dispatch as a sender it does not control. Which is exactly why the
destination's `originSender` check carries weight.

## What is not claimed

Halflife does not prove a circuit is secure. It records what was tested, with
which capability, against which exact build, and makes that machine-readable.

The current tier is **C1** — dependency closure resolution and advisory
matching, with no model involved anywhere in that path. Higher tiers exist in
the specification and are not claimed, because **no tier is claimed without a
published measurement behind it.**

Three further limits, stated rather than implied:

- `circuit_hash` v1 is a **build fingerprint**, not a binding to the constraint
  system or the verifying key.
- Embargo via staleness is **quieter than publishing an invalidation, not
  confidential.** Passport state is public on-chain and correlatable.
- A destination chain **inherits Solana's verification** — it cannot check
  ed25519 itself, so Hyperlane authenticates transport, not truth.

## Documentation

| | |
|---|---|
| [`docs/passport-spec.md`](docs/passport-spec.md) | The frozen wire format |
| [`docs/capability-model.md`](docs/capability-model.md) | The C0–C5 ladder |
| [`docs/disclosure-policy.md`](docs/disclosure-policy.md) | What we scan and publish |
| [`docs/commitments.md`](docs/commitments.md) | Every design promise and where it is enforced |
| [`docs/build-order.md`](docs/build-order.md) | Execution plan and progress |
| [`docs/ism-config.md`](docs/ism-config.md) | Interchain security configuration |

## Repository

```
crates/halflife-core         the passport primitive
crates/halflife-lineage      closure resolution + advisory ingestion
crates/halflife-registry     store + reverse dependency index
crates/halflife-projection   impact projection, disclosure-aware
crates/halflife-exercise     fire drills with reconstructable telemetry
crates/halflife-bench        measured on-chain cost
solana/programs              Anchor: registry + reference consumer
evm/src                      destination registry + consumer
apps/control-room            the interface
packages/passport-ts         independent format implementation
```

## Three implementations agree

The 125-byte format is reproduced byte-for-byte by **Rust**, a zero-dependency
**Node** implementation, and **Solidity** — including `u64::MAX`, `i64::MIN`,
`i64::MAX` and `u16::MAX`.

That second implementation is not a convenience. It is the only thing that
separates *the specification is reproducible* from *two copies of one library
agree with each other* — and it earned its place by catching a real defect
before the format froze: 64-bit fields were JSON numbers, and a JSON number is
float64 everywhere that matters.

---

<div align="center">
<sub>Apache-2.0</sub>
</div>
