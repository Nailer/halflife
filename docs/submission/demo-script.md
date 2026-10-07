# Demo script

Three minutes. Everything shown is live; nothing is staged.

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
> thereof*. Nothing enumerates them. This does."

Edges draw out. Point at the red ones.

Now click **`halo2_gadgets 0.5.0`** — the patched version.

> "Same dependency, fixed version. It still reaches circuits. It affects none of
> them.
>
> Anyone can build an alarm. Showing what a healthy dependency *doesn't* touch
> is what proves this discriminates."

---

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

**"Is this just `cargo audit`?"**
RustSec has no entry for `halo2_gadgets`, `orchard` or `zcash_primitives`, so
`cargo audit` returns clean on a vulnerable tree. More importantly, nothing
consumes an advisory on-chain — a program cannot halt on a GHSA identifier.
That is the gap.

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
