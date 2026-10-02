# Halflife's own passport

The system that asks other projects for cryptographic provenance is subject to
the same instrument, with no exemption.

```bash
./scripts/self-passport.sh
```

## Current result: INVALID

That is the honest output and it is published rather than quietly fixed before
anyone looks.

| | |
|---|---|
| Closure | 653 packages |
| Carrying published advisories | 15 |
| Status | `INVALID` |
| Capability claimed | C1 |

The findings are transitive dependencies of the Solana and test-harness stack —
`litesvm`, `solana-client`, `solana-sdk` — reaching crates like `bincode`,
`curve25519-dalek`, `ed25519-dalek` and `rustls-webpki`. They arrive through
`halflife-bench` and `halflife-exercise`, the crates that measure and exercise
the system, not through the passport primitive itself.

## Why this is published rather than suppressed

Three reasons, and the first is the one that matters.

**It is the only honest option.** A project whose entire premise is *no claim
without measurement* cannot exempt itself from its own measurement the moment
the measurement is unflattering. Scoping the scan to the crates that happen to
pass would be exactly the curation this tool exists to make visible.

**It demonstrates the tool works.** A self-scan that returned `VALID` would
prove nothing — the interesting evidence is that pointing the instrument at
ourselves produced a real, specific, actionable list rather than a clean bill of
health.

**It is the ordinary case.** Most real projects carry transitive advisories they
did not choose and cannot immediately remove. Halflife's job is to make that
legible and enforceable, not to pretend it is rare.

## What it does not mean

It does not mean the passport primitive is unsound. `halflife-core` has no
dependency carrying an advisory; the findings enter through the measurement and
exercise crates. The C1 tier says a closure was resolved and matched against
published advisories — nothing more is claimed about any of this code.

## What would change it

Pinning or replacing the affected transitives, or splitting the workspace so the
benchmark and exercise harnesses resolve separately from the shipped crates.
Both are real work and neither has been done, so the passport reports `INVALID`
until one of them is.
