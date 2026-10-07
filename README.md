<div align="center">

# Halflife

**A circuit breaker for protocols built on zero-knowledge cryptography.**

When a vulnerability is published in cryptography your protocol depends on, your
program stops accepting proofs — in one slot, with no governance vote, no
redeploy, and nobody awake.

[**Live →**](https://halflife-control-room.vercel.app) · [Demo script](docs/submission/demo-script.md) · [How it works](#how-it-works)

</div>

---

## The problem

You built something on zero-knowledge cryptography — private payments, a bridge,
a rollup. It depends on crypto libraries you did not write and cannot audit
yourself.

One day a vulnerability is found in one of them. Today:

1. **You might not hear for weeks.** Nobody emails you.
2. **Stopping takes humans.** A meeting, a governance vote, three multisig
   signers, a redeploy.
3. **That takes hours or days.** Money moves the whole time.
4. **And ZK bugs are silent.** Nothing crashes. Nothing reverts. Nothing shows
   up in a block explorer. A broken circuit produces a *valid-looking proof for
   an invalid statement.*

This is not hypothetical. In 2026 four cryptographic systems that had each
passed professional review failed — Zcash Sprout, Zcash Orchard, Aztec V5, and
Coldcard at $116M. Every flaw was years old. Every one was found with AI
assistance.

The researcher who found the Orchard bug had **already run the same framework
over the same code with the previous model generation, and found nothing.** The
code never changed. The tools got better.

**So an audit is a perishable good.**

## What you get

### 1 · Am I affected?

```bash
halflife scan .
```

Tells you whether your crypto dependencies carry published advisories.

`cargo audit` already does this for ordinary Rust — but it **misses the ZK
crates entirely.** RustSec has no entry for `halo2_gadgets`, `orchard` or
`zcash_primitives`, so `cargo audit` reports clean on a tree pinned to versions
carrying a counterfeiting vulnerability. We checked.

### 2 · Don't let me ship it again

Drop the GitHub Action in. A pull request that pulls a vulnerable crypto
dependency into your closure **fails the build** before anyone merges it.

### 3 · Stop my protocol automatically

The part nothing else does.

Your on-chain program checks a passport before it accepts a proof:

```rust
// Read-only account. 1,520 compute units. No CPI.
match passport.effective(now) {
    Effective::Valid => { /* proceed */ }
    Effective::Stale => return err!(PassportStale),    // evidence aged out
    Effective::Invalid => return err!(PassportInvalid), // an issuer reported a problem
}
```

When a vulnerability is published, the passport flips and **your program stops
accepting proofs within one slot.** No meeting. No vote. No redeploy.

## The property that makes it trustworthy

**Suppressing the message produces the safe outcome.**

A passport cannot claim its own freshness — `STALE` is worked out by the reader
against its own clock and can never be issued. So a program that stops receiving
updates **blocks.** There is no path where losing the network means carrying on
with stale trust.

Demonstrated on devnet with nothing published:

```
relayer-censorship    41s containment    PassportStale
                      exactly one on-chain write — the baseline
```

An attacker who can censor delivery gains nothing, because **absence is what
causes the block.**

## Who it is for

| | What they touch |
|---|---|
| A developer | The CLI and the CI check |
| Their on-chain program | The passport — a read-only account |
| A security lead | The control room: fleet status, blast radius, fire drills |
| Everyone else | Nothing. Like a circuit breaker, it works unattended. |

## Try it

```bash
git clone https://github.com/Nailer/halflife && cd halflife
./scripts/verify-all.sh
```

Thirty-nine checks across every layer. Every number it prints is measured during
the run, not quoted.

```
39 passed   0 failed   0 skipped
```

Then see the blast radius for yourself:

```bash
cargo build --release
./target/release/halflife impact halo2_gadgets@0.4.0 --operator   # 3 affected
./target/release/halflife impact halo2_gadgets@0.5.0 --operator   # 0 affected
```

Both reach circuits. Only one affects them. **Anyone can build an alarm —
showing what a healthy dependency doesn't touch is what proves it
discriminates.**

## Measured, not claimed

| | |
|---|---|
| Consumer passport check | **1,520 CU** — 0.76% of a transaction budget |
| Publish a signed passport | 11,550 CU |
| Containment, dependency compromise | 4s |
| Containment, relayer censorship | 41s, with no invalidation published |

Reproduce with `cargo run --release -p halflife-bench`.

A devnet passport also drives a consumer on an EVM chain to the same decision
from the same 125 bytes (`./scripts/cross-chain.sh`). That run uses a **local
chain and a stand-in mailbox**, and says so in its own output.

## Hyperlane, for real

The registry has sent a real message through Hyperlane's deployed devnet mailbox,
accepted with **our program as the sender**. Reading the mailbox's own account
back confirms the sender, the destination, and that the body is exactly the 125
signed bytes:

```bash
./target/release/halflife-exercise verify-dispatch exercises/hyperlane-dispatch-*.json
```

**What it does not show:** delivery. No destination registry is deployed on a
public chain and no relayer was paid, so the destination leg runs only against
the local chain above.

## How it works

Five layers, each with one job. The boundaries between them *are* the design.

| | |
|---|---|
| **Lineage** | Resolve a circuit's dependency closure, match every package against published advisories. No model in this path; reproducible offline by anyone. |
| **Passport** | A 125-byte signed claim: which circuit, which issuer, what was verified, when it expires. Three independent implementations agree on every byte. |
| **Registry** | Published on Solana. Passport accounts are read-only in the consumer path, so any number of programs check the same passport in one slot without contending. |
| **Propagation** | The registry dispatches state through Hyperlane, signing as itself. Hyperlane authenticates *transport* — it establishes what Solana said, not that the claim is true. Solana did that. The origin leg is verified against the real devnet mailbox; delivery to a destination chain is not yet demonstrated. |
| **Enforcement** | The consumer owns the policy. Halflife publishes state and never decides what a program requires. |

## Why these three ecosystems

Each is load-bearing. Remove one and it stops working.

**Zcash** is the cryptographic target. `halo2_gadgets` before 0.5.0 carried a
missing copy constraint in variable-base scalar multiplication
([GHSA-ww9q-8r59-xv46](https://osv.dev/vulnerability/GHSA-ww9q-8r59-xv46)),
letting a prover produce a valid proof for an under-constrained Orchard Action.
The fixtures span that real boundary with genuine crates.io checksums.

**Solana** is where enforcement is affordable. Passport accounts are read-only in
the consumer path, so Sealevel never serialises them. That is why 1,520 CU buys
a check *inside* a hot path rather than beside one — the measurement is the
argument, not the convenience.

**Hyperlane** makes security state portable, and the reason is in its own source:
the mailbox requires a dispatching program to sign with a PDA derived under the
declared sender. A program cannot dispatch as a sender it does not control —
which is exactly why the destination's `originSender` check carries weight. (And
why our own `set_route` mattered: see [`security-notes.md`](docs/security-notes.md).)

## What is not claimed

Halflife does not prove a circuit is secure. It records what was tested, with
which capability, against which exact build.

The current tier is **C1** — dependency closure and advisory matching, with no
model anywhere in that path. Higher tiers are specified and **not claimed,
because no tier is claimed without a published measurement.**

It also issues a passport for **itself**, under the same rules. That passport
currently reads `INVALID` — 15 advisories in its own closure. We published that
rather than scoping the scan until it passed.

Three further limits, stated rather than implied:

- `circuit_hash` v1 is a **build fingerprint**, not a binding to the constraint
  system or the verifying key.
- Embargo via staleness is **quieter than publishing an invalidation, not
  confidential.** Passport state is public on-chain and correlatable.
- A destination chain **inherits Solana's verification** — it cannot check
  ed25519 itself.
- **No external audit.** We found and fixed two access-control holes in our own
  program ([`security-notes.md`](docs/security-notes.md)); that says something
  good about our process and nothing about what we have not found.

## Deployed

| | |
|---|---|
| Control room | https://halflife-control-room.vercel.app |
| Passport program | [`CkDhRfJRi…`](https://explorer.solana.com/address/CkDhRfJRiGEa3kgnEUEvCBgyht62MTkDD6e754DLtB2?cluster=devnet) · Solana devnet |
| Consumer program | [`BAcrrJYj5…`](https://explorer.solana.com/address/BAcrrJYj5Y5DfcqnHgDwm5rhJUJdowZh25NvFLJAUzSW?cluster=devnet) · Solana devnet |

## Documentation

| | |
|---|---|
| [`passport-spec.md`](docs/passport-spec.md) | The frozen wire format |
| [`capability-model.md`](docs/capability-model.md) | The C0–C5 ladder |
| [`disclosure-policy.md`](docs/disclosure-policy.md) | What we scan and publish |
| [`disclosure-status.md`](docs/disclosure-status.md) | Live obligations, including one still unresolved |
| [`security-notes.md`](docs/security-notes.md) | Holes we found in our own program, and what is still unsolved |
| [`self-passport.md`](docs/self-passport.md) | Halflife under its own instrument |
| [`commitments.md`](docs/commitments.md) | Every design promise and where it is enforced |

---

<div align="center">
<sub>Apache-2.0</sub>
</div>
