# Exercise records

Controlled threat scenarios run against Solana devnet with the deployed
programs. Each record carries transaction signatures and slots so a third party
can re-check it:

```bash
halflife-exercise verify exercises/<record>.json
```

Every timing is derived from recorded timestamps at read time. No duration is
stored, so a reader recomputes it and either agrees or has found a problem.

Events marked `LOCAL` are labelled as **not independently verifiable** rather
than presented alongside on-chain evidence as if they carried the same weight.

## The two scenarios

**`dependency-compromise`** — a dependency becomes unsafe, a passport is
invalidated, and the consumer refuses to proceed. Demonstrates the fast path:
an invalidation is published and delivered.

**`relayer-censorship`** — the one that matters. A baseline passport is
published and then **nothing further is sent**. No invalidation, no update, no
second write. The consumer blocks anyway, on expiry alone.

That is the property the whole design rests on: withholding a message produces
the safe outcome. `verify-all.sh` asserts the censorship record contains
*exactly one* on-chain write, because if a second appeared, the block would have
been caused by something published rather than by absence.
