# Security Passport — v1.0.0 (frozen)

A passport is a signed, machine-readable claim about **what was verified** on an
exact circuit build. It is not a claim that the circuit is safe, and no field in
it should be read that way.

## The split

Two artifacts, deliberately separated.

| | On-chain core | Off-chain evidence |
|---|---|---|
| Size | 125 bytes, fixed | Unbounded |
| Content | Digests, status, clock | Dependency closure, advisory hits, methodology |
| Consumer | Solana programs, in the hot path | Auditors, humans, CI |
| Binding | Signed directly | Bound via `evidence_hash` |

A program never reads the dependency graph. It reads 117 bytes and decides.

## Canonical encoding

`PassportCore` is fixed-width little-endian. This byte layout is both the
Solana account payload and the signing preimage — there is exactly one
serialization, so signatures produced off-chain verify on-chain unchanged.

| Offset | Len | Field |
|---:|---:|---|
| 0 | 1 | `version` |
| 1 | 32 | `circuit_hash` |
| 33 | 32 | `issuer` (ed25519 pubkey = Solana pubkey) |
| 65 | 8 | `sequence` (u64 LE) |
| 73 | 1 | `capability` (tier 0–5) |
| 74 | 1 | `status` (1 = VALID, 2 = INVALID) |
| 75 | 8 | `issued_at` (i64 LE, unix) |
| 83 | 8 | `expires_at` (i64 LE, unix) |
| 91 | 32 | `evidence_hash` |
| 123 | 2 | `advisory_count` (u16 LE) |

**Total: 125 bytes.**

## JSON encoding of 64-bit fields

`sequence`, `issued_at` and `expires_at` are JSON **strings**, and a reader must
**refuse** a JSON number for them.

This is not stylistic. A JSON number is a float64 in every runtime that matters,
so any value above 2^53 corrupts silently — a `sequence` of `u64::MAX` reads back
as 2^64 and then fails to encode at all. The conformance vectors caught this
before the format froze, which is exactly what they exist for.

Refusing numbers outright, rather than accepting small ones, is deliberate:
leniency would let a producer emit a corrupted value that passes everywhere
except at the boundary.

## Sequence and supersession

`sequence` is monotonic per `(circuit_hash, issuer)` and is inside the signed
core, so it cannot be renumbered without invalidating the signature.

A passport supersedes another only when `circuit_hash` and `issuer` both match
and `sequence` is strictly greater. A passport from a *different* issuer never
supersedes — it coexists, and the consumer's quorum policy decides what that
means.

Destinations keep a high-water mark per `(circuit_hash, issuer)` and reject
anything at or below it. That defeats replay of a superseded passport, including
onto a chain that has not yet seen the newer one, **without** binding a chain
domain into the core — binding a domain would require one signature per
destination and destroy the portability the design exists for.

## Signing

```
signature = Ed25519( "halflife-passport-v1" || canonical_bytes )
```

The domain separator prevents a Halflife signature from being replayed as a
signature over another protocol's message. Ed25519 is chosen because Solana's
sigverify precompile accepts it natively and because an issuer identity is then
just a Solana address — `solana-keygen` produces a valid issuer key.

`issuer_id` (base58) is carried alongside for readability, and a verifier MUST
reject any passport where it disagrees with the `issuer` bytes inside the signed
core. Otherwise a passport could advertise one issuer while being signed by
another.

## Status, and why `STALE` is not issuable

Only two statuses can be signed: `VALID` and `INVALID`. `STALE` is derived by
the reader, never asserted by the issuer:

```
INVALID                      -> INVALID   (at any time)
VALID and now >= expires_at  -> STALE
VALID and now <  expires_at  -> VALID
```

This is the system's central safety property. **A consumer that stops receiving
updates blocks rather than continuing to trust the last good state.** An actor
who can suppress delivery — a censoring relayer, a partitioned network, a failed
job — therefore cannot hold a circuit open. Suppression produces the safe
outcome, not the unsafe one.

The inverse design, where a passport stays `VALID` until an explicit
invalidation arrives, would make delivery a trusted dependency. It is not.

## Issuer lifecycle

Revocation is **forward-only**. A passport signed before `revoked_at` stays
cryptographically verifiable, because history should not become unreadable, but
it stops being *acceptable* from the moment of revocation. Concretely, a
consumer accepts a passport only if `issued_at >= registered_at` and, where the
issuer is revoked, both `now` and `issued_at` precede `revoked_at`.

Rotation records the predecessor key, so an audit trail survives the change.

## Clock rules

A passport whose `issued_at` sits more than `MAX_CLOCK_SKEW_SECS` (300) beyond
the reader's clock is rejected outright. Without that bound, a compromised
issuer could mint a claim timed to outlive its own revocation.

`expires_at` must be strictly greater than `issued_at`.

## Absence is not permission

A consumer that finds **no** passport for a circuit it depends on must block,
exactly as if it found a stale one.

The unsafe integration — *check the passport if one exists* — inverts the
invariant and hands a free pass to anyone able to prevent registration. The
reference API takes an `Option` and returns `Block::Missing` for `None`
specifically so that the mistake requires deliberate effort rather than
inattention. Any SDK in any language must make it equally hard to write.

## Disclosure state

`disclosure` lives on the evidence, never on the core. A core field would make
every passport announce that something is embargoed, which is the signal an
embargo exists to suppress. Evidence with no explicit disclosure state
deserializes as `EMBARGOED` — a missing field fails closed.

See `docs/disclosure-policy.md`, including the plainly stated limit: embargo via
staleness buys quiet, not secrecy.

## Conformance vectors

`fixtures/vectors.json` pins the encoding, the signing preimage and the
resulting signature, generated from a published test seed. Two implementations
must reproduce every vector byte-for-byte:

```bash
halflife vectors verify fixtures/vectors.json          # Rust
node packages/passport-ts/verify-vectors.mjs           # independent, zero deps
```

The second implementation is not a convenience. It is the only thing that
distinguishes *the specification is reproducible* from *two copies of one
library agree with each other*.

## `evidence_hash`

SHA-256 over the canonical JSON encoding of the evidence document. Canonical
means: struct field order as declared, dependencies sorted by `(name, version)`,
advisory id lists sorted and deduplicated. Discovery order never affects the
digest.

A verifier given both artifacts MUST confirm the evidence reproduces both
`evidence_hash` and `circuit_hash`, so a passport cannot be decoupled from the
evidence that justified it.

## `circuit_hash` — what it is, and what it is not

v1 derivation:

```
SHA-256( "halflife-circuit-id-v1\0"
         || repository \0 || commit \0 || proof_system \0
         || for each dependency sorted by (name, version): name "@" version \0 )
```

This is a **build-identity fingerprint**. State the limits plainly:

- It binds to *how the circuit was built* — its declared source and resolved
  dependency closure — not to the constraint system that build produces.
- Two builds with an identical commit and closure collide **by design**; that is
  what makes the identity stable.
- A source change that moves no dependency version is invisible to it.

Binding to the verifying key is the v2 intent. It is deliberately not claimed
here, because a fingerprint that sounds stronger than it is would be the exact
failure this project exists to make visible.

## Consumer policy

Halflife publishes state. It does not set policy. A consumer declares its own
requirements:

```
require status_at(now) == VALID
require capability     >= C4
require circuit_hash   == expected
require issuer         in {trusted set}
```

Keeping policy at the consumer is what stops Halflife becoming a centralized
security oracle — the same reasoning Hyperlane applies to ISMs, where the
destination application chooses its own verification rules rather than
inheriting the protocol's.

## Multi-issuer

The core binds exactly one issuer. Multiple issuers attesting to the same
circuit produce multiple passports over the same `circuit_hash`, and a consumer
requiring *n*-of-*m* checks that many accounts. No aggregate signature scheme is
introduced at v1; the consumer's loop is cheap and the trust set stays explicit
and inspectable.
