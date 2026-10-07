# Colosseum submission copy

Paste-ready text for each field. Trim to fit; the first sentence of each block
is the one that has to land.

---

## Name

**Halflife**

## One-liner (≤ 100 chars)

> A use-by date for cryptographic security. Signed passports for ZK circuits,
> enforceable on-chain.

*(93 characters)*

## Short description (≤ 280 chars)

> In 2026 four cryptographic systems failed — every flaw years old, every one
> found with AI assistance. An audit is now a perishable good. Halflife gives a
> ZK circuit a signed passport saying what was verified and when it expires, and
> programs refuse to proceed without one.

*(277 characters)*

## Full description

**An audit tells you what someone knew then. It does not tell you what is
detectable now.**

Between March and August 2026, four cryptographic systems that had each passed
professional review failed. Zcash Sprout — proof verification skipped entirely,
four years. Zcash Orchard — a counterfeiting circuit, four years. Aztec V5 — a
proving-system soundness bug. Coldcard — $116M drained from a five-year-old
flaw.

One detail turns that from a list into a finding. The researcher who found the
Orchard bug had **already run the same auditing framework over the same code
with the previous model generation, and it found nothing.** The code never
changed. The capability frontier moved.

Zero-knowledge systems are the most exposed class of software to this, for a
reason specific to them: **their bugs are silent.** A broken contract reverts
and shows up in a block explorer. An under-constrained circuit produces a
valid-looking proof for an invalid statement. Nothing fails, nothing reverts,
and "no confirmed exploitation" is a structurally weak claim.

### What Halflife is

A circuit gets a **passport** — a 125-byte signed claim about what was actually
verified, at what capability, about which exact build, and when that stops being
current. A Solana program reads it in 1,520 compute units and refuses to proceed
without one. The registry dispatches that state through Hyperlane; the origin
leg runs on devnet today, and delivery to a destination chain is not yet
demonstrated.

### The property it is built around

**Suppressing delivery produces the safe outcome.**

A passport cannot assert its own freshness — `STALE` is derived by the reader
against its own clock and is never issuable. So a consumer that stops receiving
updates blocks. There is no path where losing the network means continuing to
trust old state.

Demonstrated on devnet with nothing published: a baseline passport, then
silence. 41 seconds later the consumer blocked, on expiry alone, after exactly
one on-chain write.

### What it answers that nothing else does

The advisory for `halo2_gadgets` lists its affected crates and then says *"and
any dependents thereof."* **Nothing enumerates them.** RustSec carries no entry
for `halo2_gadgets`, `orchard` or `zcash_primitives`, so `cargo audit` returns
clean on a vulnerable tree — and nothing consumes an advisory on-chain at all. A
program cannot halt on a GHSA identifier.

### Measured, not claimed

| | |
|---|---|
| Consumer passport check | 1,520 CU — 0.76% of a transaction budget |
| Containment, dependency compromise | 4s |
| Containment, relayer censorship | 41s, no invalidation published |

`./scripts/verify-all.sh` runs 39 checks across every layer. Every number it
prints is measured during the run.

### What is not claimed

Halflife does not prove a circuit is secure. It records what was tested, with
which capability, against which exact build. The current tier is **C1** —
dependency closure and advisory matching, no model in that path. Higher tiers
are specified and not claimed, because no tier is claimed without a published
measurement.

It also issues a passport for **itself**, under the same rules. That passport
currently reads `INVALID` — 15 advisories in its own closure. We published that
rather than scoping the scan until it passed.

## Why Solana, Zcash and Hyperlane

Each is load-bearing; remove one and it stops working.

**Zcash** is the cryptographic target. `halo2_gadgets` before 0.5.0 carried a
missing copy constraint in variable-base scalar multiplication
(GHSA-ww9q-8r59-xv46), letting a prover produce a valid proof for an
under-constrained Orchard Action. The fixtures span that real boundary with
genuine crates.io checksums.

**Solana** is where enforcement is affordable. Passport accounts are read-only
in the consumer path, so Sealevel never serialises them and any number of
programs can check the same passport in one slot. That is why 1,520 CU buys a
check inside a hot path rather than beside one — the measurement is the
argument.

**Hyperlane** makes security state portable, and the reason is in its own
source: the mailbox requires a dispatching program to sign with a PDA derived
under the declared sender. A program cannot dispatch as a sender it does not
control — which is precisely why the destination's `originSender` check carries
weight. An off-chain relay would make the relayer trusted instead.

That is no longer only an argument. The registry has sent a real message through
Hyperlane's deployed devnet mailbox, which accepted it with our program as the
sender. Reading the mailbox's own account back confirms the sender, the
destination, and that the body is exactly the 125 signed bytes
(`halflife-exercise verify-dispatch` re-checks it independently).

**What that does not show:** delivery. No destination registry is deployed on a
public chain and no relayer was paid, so the destination leg is exercised
against a local chain with a stand-in mailbox. We say so in the product, the
README and here.

### A hole we found in our own program

While preparing that dispatch we re-read our own enforcement code and found two
access-control holes: anyone could set a dispatch route (choosing which program
receives the registry's signature, which would have let an attacker send forged
passports as us), and anyone could register someone else's public key as an
issuer. Both are fixed, each is pinned by an attack test, and the write-up is in
`docs/security-notes.md` alongside what is still unsolved. We would rather you
read that than discover it.

## Links

| | |
|---|---|
| Live demo | https://halflife-control-room.vercel.app |
| Repository | https://github.com/Nailer/halflife |
| Passport program | `CkDhRfJRiGEa3kgnEUEvCBgyht62MTkDD6e754DLtB2` (devnet) |
| Consumer program | `BAcrrJYj5Y5DfcqnHgDwm5rhJUJdowZh25NvFLJAUzSW` (devnet) |

## Track

**Solana** — on-chain registry and enforcement, two programs deployed to devnet
and measured.

We are not entering the Zcash track. It asks for products that integrate with the
Zcash blockchain or ZEC, and we integrate with neither: Zcash's `halo2` and
`orchard` crates are the cryptographic *target* we analyse, not something we
build on.

## Note for judges on the embargo

Four published crates in the registry resolve to affected `halo2_gadgets`
versions. They are **not named in the live control room**, in either view of its
public export.

They **are** named in the repository: `fixtures/fleet/` has a directory for each,
because the findings have to be reproducible. That is an inconsistency with our
own policy, which we record in `docs/disclosure-status.md` rather than hide. It
is being resolved by notifying the maintainers.

Our disclosure policy — written before the first scan, not after the first
finding — requires 14 days' maintainer notice before naming a third party. That
clock does not finish before the deadline. We considered amending the policy so
the dates worked, and did not.

The demo shows both views. In the public view the embargoed circuits are
**absent, not redacted** — a dependency used only by embargoed circuits is
indistinguishable from one nothing uses. That is the embargo mechanism running
on a real embargo rather than a hypothetical one.
