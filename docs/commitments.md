# Commitments

Every promise made while designing Halflife, with where it is enforced and how
to check it. If something here has no enforcement point, it is drift — and the
whole project is premised on not letting unmeasured claims stand.

Check this file at every gate.

## The thesis

| # | Commitment | Status |
|---|---|---|
| T1 | Positioning leads with *security passports for zero-knowledge circuits*, not "control plane". The architecture is discovered by the reader, not announced. | `README.md` |
| T2 | Halflife never claims a circuit is secure. It records what was tested, at what capability, against which exact build. | `README.md`, `lib.rs` module docs |

## The invariant

| # | Commitment | Enforced at |
|---|---|---|
| I1 | No consumer may treat an unrefreshed claim as valid indefinitely | `PassportCore::status_at` |
| I2 | `STALE` is derived by the reader and is **not issuable** | `Status` has no `Stale` variant |
| I3 | `STALE ≠ INVALID`. Staleness is not a finding and must never be rendered as one | `Block::Stale` vs `Block::Invalid`; UI contract pending P7 |
| I4 | **Absence is not permission** | `decide(Option<..>)` → `Block::Missing` |
| I5 | A passport from the reader's future is rejected | `within_clock_bounds` |
| I6 | Suppressing delivery cannot hold a circuit open | expiry → `Stale` → block |

## Trust boundaries

| # | Commitment | Enforced at |
|---|---|---|
| B1 | Evidence ≠ Authority. The engine produces evidence; issuers make claims | `Evidence` / `SignedPassport` split |
| B2 | Authority ≠ Transport. Hyperlane authenticates transport; it never establishes that a claim is true | P3 — destination verifies the issuer signature, not the relayer |
| B3 | Transport ≠ Policy. The consumer owns policy; Halflife never sets it | `Policy` lives with the consumer, not the issuer |
| B4 | `disclosure` never enters the signed core — a core field would announce that an embargo exists | `Evidence.disclosure`; absent → `EMBARGOED` |

## Discipline rules

| # | Commitment | How to check |
|---|---|---|
| D1 | **No tier claimed without measurement.** Tool emits C1 only until C2+ recall is published | `Capability::C1` hardcoded in `scan` |
| D2 | Every published number reproducible from a committed script | Gate E benchmark, Gate G telemetry |
| D3 | The deterministic path stays model-free and reproducible offline | `--offline`, pinned OSV snapshot |
| D4 | Negative results are published, including poor engine recall | P5 exit criteria |
| D5 | **CLI before UI.** A capability not in the CLI is not built | P7 review rule |
| D6 | No manufactured scale. Small, real fleet only | Gate C — 6–9 systems, never 47 |
| D7 | **Measured, not simulated.** No displayed timing from a predetermined script | P7 exit criteria; event ledger |
| D8 | Embargo via staleness is quiet, not confidential — stated plainly, never implied otherwise | `docs/disclosure-policy.md` |

## Ecosystem necessity

Each must be load-bearing. If one could be swapped out without loss, it is decoration.

| # | Commitment | Test |
|---|---|---|
| E1 | **Zcash** is the real cryptographic target: halo2, Orchard, Ironwood, GHSA-ww9q-8r59-xv46 | Fixtures span the real advisory boundary with genuine checksums |
| E2 | **Solana** is canonical state and enforcement, justified by a measured CU cost — not by convenience | Gate E publishes the number whatever it says |
| E3 | **Hyperlane** makes security state portable across chains, with an Aggregation ISM, never the bare default | Gate F: cross-chain block, then relayer killed and block holds |

## Scope discipline

| # | Commitment |
|---|---|
| S1 | Two Fire Drill scenarios only: dependency compromise, relayer censorship |
| S2 | Issuer compromise and deployment drift are **defined and not armed**, shown as requiring P6 and P4B |
| S3 | Projection claims **circuits only** until consumer registration exists — no inferred protocol counts |
| S4 | P4B (artifact identity) must not block P4A (dependency intelligence) |
| S5 | No extra chain, token, DAO, marketplace, insurance layer, security score, or agent swarm |
| S6 | The verification engine is sequenced last, so its research risk cannot sink the project |

## The MVP causal chain

All ten steps, in order. Steps 3 and 10 are the legibility layer; the rest are the mechanism.

| # | Step | Stage |
|---|---|---|
| 1 | A real dependency/security event occurs | 2 |
| 2 | Halflife identifies the affected circuits, unprompted | 3 |
| 3 | Projection shows the affected closure | 3 |
| 4 | A signed passport changes state | 1 |
| 5 | Solana records the new canonical state | 4 |
| 6 | Hyperlane propagates it | 5 |
| 7 | A destination consumer blocks | 5 |
| 8 | The relayer is killed | 6 |
| 9 | Expiry → `STALE` → block, with no message delivered | 6 |
| 10 | Projection on a healthy dependency returns nothing affected | 3 |

Step 9 is the one most likely to be dropped and the one that matters most: steps
1–8 show the fast path, step 9 shows the actual safety property.

Step 10 is not decoration. Without it the demo proves Halflife raises alarms;
with it, the demo proves Halflife discriminates.

## Known limits we must keep stating

| # | Limit |
|---|---|
| L1 | `circuit_hash` v1 is a **build fingerprint**, not a binding to the constraint system or verifying key |
| L2 | Embargo via staleness is correlatable from public chain state |
| L3 | Cheap renewal makes staleness rare, and rare staleness is informative — this tension is unresolved |
| L4 | `advisory_count` cannot distinguish three hits on one advisory from three advisories; it is a signal, never a policy input |
| L5 | A fail-safe design means a stolen issuer key can grief at protocol scale. Bounded by quorum, not removable |
| L6 | Until P4B, a passport describes a registered source and build — not necessarily what is running |
| L7 | The fleet's closures are **derived**: each published requirement resolved to the highest release satisfying it, optional dependencies included on the conservative reading. They are not the crates' own lockfiles, and each fixture says so |

## Deferred, with a reason

| Item | Why not now | Where it belongs |
|---|---|---|
| Yank status as tracked data | `halo2_gadgets` 0.1.0–0.4.0 are all yanked. "Yanked **and** carrying an advisory" is a stronger signal than either alone, but modelling it is dependency intelligence, not Stage 2 storage | P4A |
| Full transitive closure | The fleet tracks four crates rather than resolving whole graphs | P4A |
| Optional/dev dependency distinction in the data model | Currently a fixture comment only | P4A |

## Measured, on the record

| What | Measured | Where |
|---|---|---|
| Consumer passport check | **1,520 CU** | `cargo run --release -p halflife-bench` |
| Publish a signed passport | 11,550 CU | same |
| Register an issuer | 7,406 CU | same |

1,520 CU is 0.76% of a default 200,000 CU transaction budget, and 0.0015% of the
100M CU block limit. The check is affordable inside a hot path, which is the
claim the Solana layer rests on (E2). The figure includes account
deserialization and instruction dispatch — bracketing only the comparisons would
produce a smaller number nobody could reproduce in practice.

## Added 7 October 2026

| # | Commitment | Enforced at |
|---|---|---|
| X1 | Only the registry can speak as the registry: routes are admin-only **and** the mailbox must be a known Hyperlane mailbox | `set_route`, `dispatch`; six attack tests in the bench |
| X2 | Nobody can register an issuer key they do not hold | `register_issuer` ed25519 proof |
| X3 | Hyperlane delivery to a destination chain is **not** claimed; the origin leg is | README, submission, interface |
| X4 | We do not claim a hackathon track we do not fit | Zcash track dropped: no ZEC or chain integration |
| L8 | `init_config` is first-caller-wins; the upgrade authority is a single key; no external audit | `docs/security-notes.md` |
