# Demo script

Three minutes **maximum**. Everything shown is live; nothing is staged.

## Timing budget (read this first)

The sections below were written at about 3:20 before the landing page, the
checker and the Action existed. To stay at or under 3:00 with them, record to this
budget and cut narration, not shots:

| Section | Seconds | Note |
|---|---|---|
| 0b Landing page | 8 | Let the intro play once, then scroll fast |
| 1 The problem | 20 | Trim to two sentences |
| 2 The enumeration | 30 | Impact view, one dependency |
| 2b Check yours | 15 | The strongest "try it" shot |
| 2c The GitHub Action | 8 | YAML on screen, one sentence |
| 3 Enforcement | 35 | Keep the 1,520 CU number |
| 4 The property | 30 | The censorship drill |
| 5 The embargo | 15 | Optional; drop first if over |
| 6 Close | 15 | |
| **Total** | **176** | about 2:56. Skip 5b entirely. |

Record each section as its own clip, then trim. If the total is over 3:00, drop
section 5 before shortening anything else.

Open **https://halflife-control-room.vercel.app** and have a terminal in the
repo. Skip the tour if it appears — you are giving it yourself.

---

## 0 · Before you start (30 seconds, off camera)

```bash
cd ~/Documents/Github/halflife && ./scripts/verify-all.sh
```

Leave the `39 passed · 0 failed` on screen if you want a cold open. If anything
is red, stop and fix it rather than talking over it.

---

## 0b · The landing page (8 seconds)

Start on the bare URL so the intro plays: the curve draws itself, then the page
rises in. Do not skip it. Scroll once, briskly, past the "how it works" steps.
Say: "Halflife gives a zero-knowledge circuit a use-by date. Your program stops
trusting it when it expires." Then press **Enter the control room**.

## 1 · The problem (25 seconds)

> "In 2026, four cryptographic systems that had each passed professional review
> failed. Every flaw was years old. Every one was found with AI assistance.
>
> The detail that matters: the researcher who found the Zcash Orchard bug had
> already run the same framework over the same code with the previous model
> generation — and found nothing. The code never changed. The tools got better.
>
> So an audit is a perishable good. And zero-knowledge bugs are silent: a broken
> contract reverts and shows up in a block explorer. An under-constrained
> circuit produces a valid-looking proof for an invalid statement. Nothing
> fails."

---

## 2 · The enumeration nobody has (40 seconds)

Go to **Impact**. Click **`halo2_gadgets 0.4.0`**.

> "This advisory lists its affected crates and then says *and any dependents
> thereof*. Nothing maps them to circuits. This does."

Edges draw out. Point at the red ones.

Now click **`halo2_gadgets 0.5.0`** — the patched version.

> "Same dependency, fixed version. It still reaches circuits. It affects none of
> them.
>
> Anyone can build an alarm. Showing what a healthy dependency *doesn't* touch
> is what proves this discriminates."

---

## 2b · Try it on your own project (20 seconds, optional)

Click **Check yours**. Press *Example: pinned to the vulnerable halo2_gadgets*:
three packages come back with the real advisory (GHSA-ww9q-8r59-xv46) and the
verdict reads **WOULD FAIL**. Press *Example: patched to 0.5.0* and it reads
**WOULD PASS**. Say: "Anyone can paste their own lockfile. It's the same lookup the
CLI does, unsigned, in your browser." Do not call it a passport: it isn't signed
and nothing is on-chain.

## 2c · The GitHub Action (8 seconds)

Show the three-line YAML on the landing page ("Add the GitHub Action") or in the
README, then the green **reusable action** job in the repository's Actions tab.
Say: "One line in CI fails a pull request that pulls in a vulnerable dependency."
It is tested through `uses:` in both directions; do not say it has users.

## 3 · Enforcement (45 seconds)

Go to **Fire drills**. Make sure **DEPENDENCY COMPROMISE** is selected. Hit
**RUN DRILL**.

> "This is a recorded exercise against Solana devnet, replayed at the gaps in
> its own record. These transaction signatures link to the live explorer — these
> actually happened."

Watch the gate flip to **BLOCKED**.

> "A third-party program refused to proceed. That check costs 1,520 compute
> units — under one percent of a transaction budget, which is why it can sit
> inside a hot path instead of beside one."

---

## 4 · The property that matters (40 seconds)

Switch to **RELAYER CENSORSHIP**. Hit **RUN DRILL**.

> "Now the interesting one. A passport is published, and then nothing else is
> sent. No invalidation. No second write. The relayer simply stops."

Gate flips to **BLOCKED — STALE**.

> "It blocked anyway, on expiry alone. One on-chain write in the whole record.
>
> That is the property the design rests on: **withholding a message produces the
> safe outcome.** An attacker who can censor delivery gains nothing, because
> absence is what causes the block."

---

## 5 · The embargo (30 seconds)

Toggle **PUBLIC → OPERATOR** in the header.

> "Four circuits become eight. The affected count goes from one to three.
>
> We found real published crates resolving to the vulnerable version. Our own
> disclosure policy requires fourteen days' maintainer notice before naming
> them, and that clock does not finish before this deadline. So they are
> embargoed.
>
> Watch what the public view does: they are not redacted, they are **absent**. A
> dependency used only by embargoed circuits is indistinguishable from one
> nothing uses. That is the embargo mechanism running on a real embargo."

---

## 5b · Across chains (optional, 20 seconds)

```bash
./scripts/cross-chain.sh
```

> "And the same passport, read back off devnet, delivered to an EVM chain. Same
> 125 bytes, same decision. The chain and the mailbox are local — a funded
> testnet swaps an RPC URL and changes nothing else."

---

## 6 · Close (20 seconds)

```bash
./scripts/self-passport.sh
```

> "Last thing. Halflife issues a passport for itself, under the same rules.
>
> It comes back INVALID — fifteen advisories in its own closure. We published
> that rather than scoping the scan until it passed.
>
> A project whose premise is *no claim without measurement* doesn't get to
> exempt itself the moment the measurement is unflattering."

---

## If asked

**"Did your tool find these bugs?"**
No. It finds which of *your* systems are exposed to bugs someone else disclosed.
The verification engine is deliberately sequenced last — reliable automated
soundness analysis is an open research problem, and leading with it would mean
having nothing if the research didn't land.

**"Won't it cry wolf?"**
Yes, at C1, and we say so. It flags any advisory in the lockfile closure, with no
severity or reachability filter. Run on a real Solana Groth16 verifier library's
workspace it flagged 14 packages, many in test tooling. That is why the first use
is a CI gate and monitoring, where a false positive is a red build, not a halted
protocol, and why filtering is the first thing to build with a design partner.

**"Is this just `cargo audit`?"**
Partly. RustSec, which `cargo audit` reads, has no entry for `halo2_gadgets`,
`orchard` or `zcash_primitives`, so it returns clean on a vulnerable tree. The
advisory *does* exist in GitHub's database and OSV (CVE-2026-54496), so
Dependabot and `osv-scanner` see it. What none of them do is map it to the
circuits that depend on it, and nothing consumes an advisory on-chain — a
program cannot halt on a GHSA identifier. That is the gap.

**"What's the capability tier really worth?"**
C1 — closure resolution and advisory matching, no model involved. Higher tiers
are specified and not claimed, because no tier is claimed without a published
measurement.

**"Why three ecosystems?"**
Remove any and it stops working. Zcash is the real cryptographic target. Solana
is where enforcement is affordable — read-only passport accounts don't contend
under Sealevel, hence 1,520 CU. Hyperlane's mailbox *requires* a dispatch
authority PDA, which is the only reason the destination's sender check means
anything.
