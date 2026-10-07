# Halflife — business plan

Colosseum asks for go-to-market, demand validation and distribution. This plan is
written to be checked: facts carry a source or a command, and hypotheses are
labelled as hypotheses. **Nothing here has been validated with a customer. The
"Demand validation" section says exactly how little we have, and what we will do
about it.**

*Solo founder. Status date: 7 October 2026.*

---

## 1. The problem

A team shipping zero-knowledge circuits (private payments, a rollup, a bridge, a
verifier program) depends on cryptographic libraries it did not write and cannot
audit. When one of them is found vulnerable the team:

1. **learns late**: the advisory names a handful of crates, not the systems built
   on them;
2. **stops slowly**: a meeting, a vote, multisig signers, a redeploy;
3. **cannot see the damage**: zero-knowledge failures are silent. A counterfeit
   proof verifies. Nothing reverts.

2026 supplied the evidence. The `halo2_gadgets` soundness bug in Zcash's Orchard
(CVE-2026-54496, published 6 July 2026) sat in the code for years and was found
by a framework that found **nothing** in the same code a model generation
earlier. An audit is a perishable good, and nobody tracks how stale theirs is.

## 2. The product

Halflife is **a use-by date for cryptographic security**. A signed, versioned
*security passport* records what has been verified about a circuit, at what depth,
and until when. A program on Solana checks it before accepting a proof. When the
passport lapses or is invalidated, the program stops accepting proofs, in one slot,
with no vote and no redeploy.

The design rule: **no consumer may treat an unrefreshed security claim as valid
indefinitely.** Withholding messages therefore produces the safe outcome, not the
dangerous one.

| Layer | What it is | Status | Price hypothesis |
|---|---|---|---|
| **Scan** | `halflife scan`, the CI gate, registry reads, the open spec | Built, free forever | **Free.** A security signal nobody can read is not a signal. |
| **Monitor** | Continuous re-checking of a team's circuits, alerts, auto-invalidation | Not built (the engine is the C1 scan on a schedule) | Paid, per circuit per month. **Unpriced, to be discovered with design partners.** |
| **Feed** | Machine-readable list of which deployed systems are currently unsafe | Not built | Paid, for exchanges, custodians, insurers. **Unpriced.** |

We deliberately state no prices. We have not asked anyone what they would pay, and
a number invented for this page would be exactly the unmeasured claim this project
exists to avoid.

## 3. What is built and measured today

Every line is reproducible (`./scripts/verify-all.sh`, 39 checks).

- A **125-byte frozen wire format**, signed with ed25519, checked by three
  independent implementations (Rust, Node, on-chain).
- **Solana enforcement** deployed to devnet. A consumer check costs **1,520
  compute units** (0.76% of a default transaction budget).
- A **registry** that resolves full dependency closures and answers "what is
  exposed to this advisory?" for 8 real fixtures.
- A **real Hyperlane dispatch** from Solana devnet (origin leg), independently
  verifiable. Delivery to a destination chain is *not* demonstrated; the
  destination contracts are exercised only locally.
- **Fire drills** recorded as signed ledgers a third party can re-verify.
- A **CI gate** (`halflife scan --deny`), tested in both directions in CI.
- Our own on-chain program had **two access-control holes that we found and
  fixed** (`docs/security-notes.md`), with attack tests.

**What it is not:** it does not discover vulnerabilities. It claims only
capability tier **C1**: dependency closure plus advisory matching, no model. Higher
tiers (circuit-level soundness analysis) are an open research problem and are not
built.

## 4. Competitive landscape

Honest framing first: **no one sells exactly this.** That can mean a gap, or it
can mean no market. We do not know which, and section 8 is how we find out.

What a buyer already has, and where each stops. Sources are linked or checkable.

| Category | Examples | What they do well | Where they stop (relative to this problem) |
|---|---|---|---|
| **Rust dependency scanners** | `cargo audit` (RustSec), `cargo-deny`, Dependabot, `osv-scanner` | Flag known-vulnerable crates in a lockfile; free; already in most pipelines | Report on *crates*, not *circuits*. Coverage is only as good as the database: on this very advisory **RustSec has no entry** (`cargo audit` is clean) while GitHub's database and OSV do (GHSA-ww9q-8r59-xv46). None produce a signed, machine-readable claim an on-chain program can consume. |
| **Commercial supply-chain security** | Snyk (reachability analysis), Socket (Rust support is enterprise/experimental, focused on malicious packages) | Triage and noise reduction for large engineering orgs; broad language coverage | Built for general application security, not for protocol-level halt decisions. No on-chain surface. |
| **ZK audit firms** | Trail of Bits, Veridise, Zellic, zkSecurity, OpenZeppelin, others | Deep human review of circuits; Veridise also ships tools (Picus, ZK Vanguard) and has worked with RISC Zero on continuous under-constrained-circuit detection | A report is a snapshot and has no expiry that anyone enforces. Not a data product, and not designed to be read by a program. **They are the most natural partners, not rivals:** an audit could be what a passport at a higher tier *attests*. |
| **ZK bug catalogues** | 0xPARC `zk-bug-tracker`, zkSecurity's zkBugs | Valuable human-readable record of bug classes and past incidents | Education and research, not a live feed of "which deployed systems are exposed right now". |
| **On-chain monitoring and automated response** | Hypernative, Forta, OpenZeppelin Defender | Detect *attacks in progress* (e.g. a malicious contract deployment) and pause contracts within seconds | They watch **chain behaviour**. A counterfeit zero-knowledge proof looks like a valid transaction, and their public materials do not cover dependency-level cryptographic advisories. Halflife acts *before* any attack, on a published weakness in code. **Complementary:** their pause button is one more consumer of our signal. |
| **Static / formal tools for circuits** | Picus, ZK Vanguard, Circomspect | Find under-constrained circuits in the author's own code | Answer "is my circuit wrong?", not "did a library I trust just become wrong?". Also different tiers of the same capability ladder. |

### Where Halflife is different

Stated narrowly, because over-claiming here would be the easiest way to lose
credibility with the judges and, later, with buyers:

1. **It joins two things that are normally separate.** Advisory data says a crate
   is bad; the dependency graph says which circuits sit on it; a signed passport
   says so in a form a program can verify; and an on-chain check turns that into a
   halt. Individually these exist. We know of no product that chains them.
2. **The unit is the circuit, not the crate.** Every scanner above stops at
   packages. A protocol's real question is "is *my system* exposed?".
3. **It fails safe by construction.** Passports expire. A censored relayer, a
   silent issuer or a dead service all produce "stop", never "keep trusting".
   Monitoring tools fail open: when they go quiet you hear nothing.
4. **It is honest about depth.** Tiers (C0–C5) state what was and was not
   verified, and only C1 is claimed. The product is built around the fact that
   AI-driven vulnerability discovery is making yesterday's "verified" claim stale
   faster than audits can follow, and says so in the data format.
5. **The free layer is the wedge.** A CI gate that is useful to one team on day
   one, with no network effect, breaks the usual two-sided adoption deadlock.

### Where we are weaker

- Incumbents have customers, brands, sales teams and audited code. We have none of
  these.
- Any of the incumbents could add an "advisory-aware" feature. Our defence is the
  open spec and a head start on the on-chain half, not a moat. **We do not claim a
  moat.**
- A well-funded rival could build the registry. What it cannot copy quickly is a
  neutral, open format that several issuers sign, which is why the format is
  open and frozen.

## 5. Market

We have not measured a total addressable market and we will not quote one.

What can be shown from sources anyone can re-run:

- **The beachhead is small and concentrated.** Only **6** crates on crates.io
  depend directly on `halo2_gadgets`
  (`curl crates.io/api/v1/crates/halo2_gadgets/reverse_dependencies`), but it has
  **1.5M downloads** and `orchard` 1.4M, because exposure is overwhelmingly
  *transitive* through the Zcash node and wallet stack. Direct dependents are what
  existing tools list; the damage flows through closures, which the registry
  enumerates.
- **The expansion is the same problem in other stacks:** Noir, circom, gnark and
  the zkVMs have their own dependency graphs. **Only Rust/Cargo is built today.**
- **The loss scale is public:** $116M (Coldcard) and $292M (KelpDAO), both 2026,
  are single incidents. They show the size of the loss class, not our share of any
  market.

How to size it properly, as a first-month task: count distinct on-chain verifier
programs and the circuits behind them across ecosystems, then the value each
secures. We will not assert it before doing it.

## 6. Go-to-market

The hardest problem is adoption deadlock: consumers will not check passports until
they exist, and issuers will not publish until someone checks. The sequence is
chosen to avoid needing both at once.

| Phase | Move | Why it works | Evidence it is done |
|---|---|---|---|
| **1. Free wedge** | CI gate in a team's pipeline: a PR that moves a circuit onto an affected dependency fails the build | Useful to one team alone; no network effect | Number of repos running the gate (**0 today**) |
| **2. Credibility** | File the missing RustSec advisories; notify maintainers of affected crates through proper channels | A verifiable public contribution; puts the name where teams already look | Merged advisory; maintainer replies |
| **3. Proof** | Public, independently checkable fire drill on devnet | Shows a team its own failure mode in about a minute | Drills run for named external teams |
| **4. Design partners** | 3–5 teams with real halo2 or Noir circuits: targets are the teams that lived through the 2026 incidents (Zcash stack teams) and ZK-native projects | Tell us the price, the false-positive tolerance and the real buying centre | Signed letters of intent, not "interested" |
| **5. Paid** | Monitor first (cheapest to deliver, closest to what teams already buy), Feed later | Recurring value between incidents | First paid design partner |

Distribution channels, in order of expected yield: CI marketplace listing and
`cargo` ecosystem visibility; security-advisory channels; ZK-focused security
researchers and auditors as referrers; ecosystem grant programs. Paid acquisition
is not planned.

## 7. Why Solana, as a business decision

Enforcement has to be cheap enough to put in a hot path, or nobody will leave it
on. A passport check costs **1,520 compute units**, measured with
`cargo run --release -p halflife-bench`. Had that number been large, the product
would not have been viable, which is why it was measured before anything was built
on top. The format is chain-agnostic; Solana is the first enforcement point, not
the only one.

## 8. Demand validation

**Current evidence from customers: none.**

| Measure | Today |
|---|---|
| Teams that have run it outside the founder | 0 |
| Paying or committed customers | 0 |
| Design-partner conversations held | *to be filled with real numbers after outreach* |
| Maintainers of affected crates notified | 0 (notices drafted, not sent) |
| Public contributions (advisories filed) | 0 |

Indirect evidence only: the incident record in section 1, and the observation that
a team running `cargo audit` today gets a clean result on a tree with a known
counterfeiting vulnerability.

**The test, small and falsifiable.** Contact a handful of teams with real halo2 or
Noir circuits and ask one question: *would a continuously updated, machine-readable
record of which of your circuits depend on advisory-affected code be useful to you,
and what would you do on the day it flipped?* A "no, because X" is as valuable as a
"yes". We commit to publishing the count of conversations and the objections, not
only the encouraging ones.

**Kill criteria.** If after ten genuine conversations no team would put the gate in
CI or pay for monitoring, the Monitor and Feed layers do not get built; the project
remains a free public tool and the spec.

## 9. Risks

- **Value concentrates in incidents**, which are rare by construction. Between them
  the product has to earn its keep through lineage and the CI gate, or it becomes
  shelfware.
- **Halting someone's protocol is a hard ask.** "So you can turn us off?" is the
  first objection. The consumer owns the policy, requires *n*-of-*m* issuers and
  chooses its own expiry, which is a correct answer that does not remove the
  friction. Realistic first adopters are CI pipelines and monitoring, where a false
  positive costs a red build, not frozen funds.
- **No engine above C1.** Reliable automated discovery of soundness bugs is an open
  research problem. The product delivers value at C1 and does not depend on that
  research landing.
- **A stolen issuer key can grief at protocol scale.** That is the direct cost of
  failing closed; bounded by consumer quorum, not removable.
- **Our own program is unaudited** and single-keyed (the admin is the upgrade
  authority). Found and fixed two holes ourselves; first-caller-wins `init_config`
  remains. No external audit yet.
- **Competitors can copy the idea.** See section 4: we claim a head start and an
  open standard, not a moat.
- **Solo founder.** One person is a delivery risk and a bus-factor risk. Mitigation
  is scope discipline (free layer first, nothing paid until a partner exists) and
  an open, documented spec so others can implement it.

## 10. Team

Solo founder: **Emmanuel** (GitHub: Nailer).

*Background, experience, location and relevant shipped work: to be finalised by
the founder in the submission form. Shipped hackathon projects include Attestable
(on-chain parametric payout settled against a real Chainlink outage, 51/51 Foundry
tests), Agent Studio Marketplace, TokenLens and NIM Duel.*

**How this project was built.** The code was written by Claude (Anthropic) under
the founder's direction. The founder set the thesis and decisions, reviewed the
output, ran the deployments and owns the result (`docs/submission/provenance.md`).

## 11. What the next 90 days buy

| By | Outcome | Measured by |
|---|---|---|
| Day 30 | Maintainer notices sent and answered; RustSec advisories filed; 10 outreach conversations held | Counts, published |
| Day 60 | CI gate running in at least 2 external repos; sizing exercise of the on-chain verifier population | Repos; a published count |
| Day 90 | One design partner on the Monitor layer with a stated price, or the kill criterion applied | Letter of intent, or a written decision |

An independent audit of the on-chain program is funded from prize money or a grant
if either arrives; it precedes any claim that Halflife is safe to depend on in
production.

---

*Sources and commands for every number above are in `docs/` and the README.
Competitor descriptions come from the vendors' own public pages, checked 7 October
2026; where a vendor's pricing was not public we state none.*
