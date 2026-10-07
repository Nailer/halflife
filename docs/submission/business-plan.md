# Business plan

Colosseum asks for go-to-market, demand validation and distribution. This is
written to be checked: facts carry a source or a command, and hypotheses are
labelled as hypotheses. **Nothing here has been validated with a customer, and
the "Demand validation" section says exactly how little we have.**

## The customer and the pain

A team shipping zero-knowledge circuits (private payments, a rollup, a bridge, a
verifier program) depends on cryptographic libraries it did not write and cannot
audit. When one is found vulnerable, the team learns late, stops slowly (a
meeting, a vote, multisig signers, a redeploy), and cannot see the damage,
because ZK bugs are silent.

2026 supplied the evidence: Zcash Sprout and Orchard (four years each), Aztec V5,
and Coldcard ($116M). The Orchard bug was found by a framework that found
**nothing** in the same code a model generation earlier. An audit is perishable,
and nobody tracks how stale theirs is.

## What we sell

A **circuit breaker**: when a vulnerability is published in a library a circuit
depends on, programs that check the circuit's passport stop accepting proofs
within one slot, with no vote and no redeploy. Three layers, each a step toward
paying:

| Layer | What it is | Price hypothesis |
|---|---|---|
| Scan | `halflife scan`, the CI gate, the registry reads, the spec | **Free, always.** A security signal nobody can read is not a signal. |
| Monitor | Continuous re-checking of a team's circuits, alerts, auto-invalidation | Paid, per circuit per month. **Unpriced: discovered with design partners.** |
| Feed | A machine-readable list of which deployed systems are currently unsafe | Paid, for exchanges, custodians and insurers who carry the exposure. **Unpriced.** |

We are deliberately not stating prices. We have not asked anyone what they would
pay, and a number invented for this page would be the kind of unmeasured claim
this project exists to avoid.

## Market size, honestly

We have not measured a total addressable market, and we will not quote one.

What we can show, from sources anyone can re-run:

- **The beachhead is small and concentrated.** Only **6** crates on crates.io
  depend on `halo2_gadgets` directly (`curl crates.io/api/v1/crates/halo2_gadgets/reverse_dependencies`).
  But `halo2_gadgets` has **1.5M downloads** and `orchard` 1.4M, because the
  exposure is overwhelmingly **transitive**, through the Zcash node and wallet
  stack.
- **That is the product's point.** Direct dependents are what existing tooling
  lists. The damage flows through full dependency closures, which is what the
  registry enumerates.
- **The expansion is the same problem in other stacks.** Noir (`Nargo.toml`),
  circom, gnark and the zkVMs have their own dependency graphs and, as far as we
  can find, no advisory coverage built for them. The lineage layer is designed
  for that; **only Rust/Cargo is built today.**
- **The loss scale is public.** $116M (Coldcard, 2026) and $292M (KelpDAO, 2026)
  are single incidents.

How we would size it properly: count distinct on-chain verifier programs and the
circuits behind them across ecosystems, then the value they secure. That is a
task for the first month, not something to assert now.

## Distribution

The order matters, because the hardest problem is adoption deadlock: consumers
will not check passports until passports exist, and issuers will not publish
until someone checks.

1. **The CI gate breaks the deadlock.** `halflife scan --deny` is useful to one
   team on day one with no network effect: a pull request that moves a circuit
   onto an affected dependency fails the build. No other party has to adopt
   anything. (The gate is tested in both directions in CI.)
2. **Public goods build credibility before sales.** RustSec has no entry for
   `halo2_gadgets`, `orchard` or `zcash_primitives`, so `cargo audit` reports
   clean on an affected tree (re-verified 7 Oct 2026). Filing those advisories is
   a verifiable contribution and puts the name where teams already look.
3. **The fire drill is the demo.** A recorded, independently checkable exercise
   against devnet shows a team their own failure mode in about a minute.
4. **Design partners from the teams that just lived this.** The Zcash stack
   (the ZODL, Shielded Labs and Zcash Foundation teams) and ZK-native projects
   with halo2 or Noir circuits. These are *targets*, not partners.

## Why Solana, as a business decision

Enforcement has to be cheap enough to put in a hot path or nobody will leave it
on. A passport check costs **1,520 compute units** (0.76% of a default
transaction budget), measured by `cargo run --release -p halflife-bench`. If that
number were large the product would not be viable, which is why we measured it
before building anything on top.

## Why we might fail

- **Value concentrates in incidents**, which are rare by construction. Between
  them the product has to earn its keep through lineage and the CI gate, or it
  becomes shelfware.
- **Halting someone's protocol is a hard thing to ask for.** "So you can turn us
  off?" is the first question. The answer (the consumer owns the policy, requires
  *n*-of-*m* issuers, and chooses its own expiry) is correct and does not remove
  the friction. The realistic first adopters are CI pipelines and monitoring,
  where a false positive costs a red build, not frozen funds.
- **We have no engine above C1.** Reliable automated discovery of circuit
  soundness bugs is an open research problem. The product is sequenced so that it
  delivers value at C1 and does not depend on that research landing.
- **A stolen issuer key can grief at protocol scale.** That is the direct cost of
  failing closed. It is bounded by consumer quorum, not removable.
- **No external audit** of our own on-chain program, which has already had two
  access-control holes that we found ourselves (`docs/security-notes.md`).

## Demand validation

**Current evidence: none from customers.**

| | |
|---|---|
| Teams that have run it | 0 outside the founders |
| Paying or committed customers | 0 |
| Design-partner conversations | _to be filled with real numbers after outreach_ |
| Maintainers notified of affected crates | _to be filled_ |

The only demand evidence we can honestly cite is indirect: the incident record
above, and the observation that a team running `cargo audit` today gets a clean
result on a tree with a known counterfeiting vulnerability.

The validation plan is small and falsifiable: contact a handful of teams with
real halo2 or Noir circuits and ask one question, *would a continuously updated,
machine-readable record of which of your circuits depend on advisory-affected
code be useful to you, and what would you do on the day it flipped?* A "no, because
X" is as valuable as a "yes".

## Team

_To be completed by the founders: backgrounds, relevant experience, location,
and why this team can build it._
