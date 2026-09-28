# Build order

The execution plan, in the repository so progress is tracked with the code
rather than beside it. Tick boxes move in the same commit as the work.

Companion documents:
- [`commitments.md`](commitments.md) — every design promise and its enforcement point
- [`passport-spec.md`](passport-spec.md) — the frozen wire format
- [`disclosure-policy.md`](disclosure-policy.md) — what we scan and publish

Gates are stopping points. Work halts at one until a decision is made.

---

## Stage 0 — Before anything touches live data — DONE

- [x] **B0.1** Disclosure policy, published before the first scan
- [x] **B0.2** Embargo limits written down: quieter than `INVALID`, not confidential

**Gate A** — approved. Scanning allowed; nothing published about third parties
until maintainers are notified.

## Stage 1 — Freeze the protocol — DONE (`v1.0.0-format`)

- [x] **B1.0** Passport primitive — 125-byte core, evidence binding, fail-safe expiry
- [x] **B1.1** `sequence: u64` inside the signed core
- [x] **B1.2** Supersession and cross-chain replay rules
- [x] **B1.3** `disclosure` on evidence, never on the core
- [x] **B1.4** Forward-only issuer revocation
- [x] **B1.5** Clock bounds; absence-is-not-permission in the spec and the API
- [x] **B1.6** Conformance vectors + independent implementation

**Gate B** — frozen. The vectors caught a real defect first: 64-bit fields were
JSON numbers, and `u64::MAX` corrupted in a float64 reader.

## Stage 2 — The registry — DONE

- [x] **B2.1** SQLite store and schema
- [x] **B2.2** `halflife register`, `registry list`, `registry deps`
- [x] **B2.3** Reverse index: `dep@version` to circuits
- [x] **B2.4** The real fleet — 8 systems, 6 from crates.io

**Gate C** — confirmed, 8 systems. Four named third parties now carry a
disclosure obligation under Gate A.

## Stage 3 — Projection — DONE

- [x] **B3.1** `halflife-projection` crate — pure, registry behind a port
- [x] **B3.2** Disclosure-aware closure, built from authorized evidence
- [x] **B3.3** `halflife impact <dep>@<version>`
- [x] **B3.4** Leak tests — an embargoed finding must not be reconstructable

**Gate D** — first legibility. Waived by the PM; evidence still reported.

## Stage 4 — Solana enforcement

- [ ] **B4.1** Anchor scaffold — passport and issuer PDAs
- [ ] **B4.2** `publish` / `supersede` / `revoke` with ed25519 verification
- [ ] **B4.3** Reference consumer that halts
- [ ] **B4.4** CU benchmark, reproducible from a committed script
- [ ] **B4.5** Devnet deployment

**Gate E** — does the number support the thesis? Hard gate. Cannot be
pre-approved: the decision depends on a measurement that does not exist yet.

## Stage 5 — Propagation

- [ ] **B5.1** Destination registry (EVM)
- [ ] **B5.2** Origin dispatch from Solana on state change
- [ ] **B5.3** Aggregation ISM, never the bare default
- [ ] **B5.4** Destination expiry and sequence high-water mark

**Gate F** — the slow path works. Kill the relayer; the consumer still blocks.

## Stage 6 — Fire Drill

- [ ] **B6.1** Event ledger with transaction signatures and message ids
- [ ] **B6.2** Scenario: dependency compromise
- [ ] **B6.3** Scenario: relayer censorship
- [ ] **B6.4** `exercise verify` — third-party reconstructable

**Gate G** — measured, not simulated.

---

*Proof boundary. Everything above is the demonstrable system.*

---

## Stage 7 — Surfaces

- [ ] **B7.1** Explorer
- [ ] **B7.2** Control room — projection, exercise, live telemetry
- [ ] **B7.3** CI action — the adoption wedge
- [ ] **B7.4** Consumer registration (only then may projection claim protocols)

## Stage 8 — Assurance

- [ ] **B8.1** Halflife's own passport
- [ ] **B8.2** Threshold signing
- [ ] **B8.3** Reproducible builds
- [ ] **B8.4** Key-compromise drill

## Stage 9 — Sustainability

- [ ] **B9.1** Free surfaces fixed by contract: registry reads, log, spec, SDKs
- [ ] **B9.2** Paid: continuous re-verification, private scanning, higher tiers

---

## The three hard gates

| Gate | Blocks | Cost of skipping |
|---|---|---|
| **A** Disclosure policy | Registering any third party | A finding about someone else's project with no agreed process |
| **B** Format freeze | Solana program, EVM contract, SDK | A migration across four runtimes and every published passport |
| **E** CU benchmark | The Solana thesis itself | Propagation built for enforcement nobody can afford to call |
