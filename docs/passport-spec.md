# Security Passport — v1

A passport is a signed, machine-readable claim about **what was verified** on an
exact circuit build. It is not a claim that the circuit is safe, and no field in
it should be read that way.

## The split

Two artifacts, deliberately separated.

| | On-chain core | Off-chain evidence |
|---|---|---|
| Size | 117 bytes, fixed | Unbounded |
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
| 65 | 1 | `capability` (tier 0–5) |
| 66 | 1 | `status` (1 = VALID, 2 = INVALID) |
| 67 | 8 | `issued_at` (i64 unix) |
| 75 | 8 | `expires_at` (i64 unix) |
| 83 | 32 | `evidence_hash` |
| 115 | 2 | `advisory_count` (u16) |

**Total: 117 bytes.**

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
