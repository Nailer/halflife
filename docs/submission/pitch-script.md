# Pitch video script (2:30)

Colosseum requires **two** videos. This is the *presentation* (why this matters,
who it is for, why this team). The *demo* video (how it works) is
`demo-script.md`. They must not be the same video.

Face-to-camera or voiceover over simple typography. Screen-record the real
product for any product shot; do not generate imagery of it.

Items in `[brackets]` must be filled with true statements or deleted.

---

**0:00 — The hook (20s)**

> "In 2026, four cryptographic systems that had each passed professional review
> failed. Every flaw was years old. Every one was found with AI assistance.
>
> The researcher who found the Zcash Orchard bug had already run the same
> framework over the same code with the previous model, and found nothing. The
> code didn't change. The tools did."

**0:20 — The problem (25s)**

> "So an audit is a perishable good, and nobody tracks how stale theirs is.
>
> Worse, zero-knowledge bugs are silent. A broken contract reverts. A broken
> circuit produces a valid-looking proof for an invalid statement. Nothing fails.
>
> And when you do find out, stopping takes people: a meeting, a vote, three
> signers, a redeploy. Money moves the whole time."

**0:45 — What we built (30s)** *(product shot: the Impact view)*

> "Halflife is a circuit breaker for protocols built on zero-knowledge
> cryptography.
>
> Three things. One: tell me whether I'm affected, including through the
> dependencies of my dependencies. Two: fail my build before I ship it. Three:
> stop my program automatically, in one slot, when a vulnerability is published.
>
> That last one is the product, and nothing else does it."

**1:15 — Why it can be trusted (25s)** *(product shot: the censorship drill)*

> "The key property: a passport can't claim its own freshness. If a program stops
> receiving updates, it blocks. So an attacker who censors the message gains
> nothing; absence is what causes the block.
>
> And it's cheap enough to leave on. A check costs 1,520 compute units, under one
> percent of a transaction. We measured that before building anything on top."

**1:40 — The business (25s)**

> "The scanner, the CI gate and the registry reads are free, because a security
> signal nobody can read isn't one. Teams pay for continuous monitoring across
> their circuits; exchanges, custodians and insurers pay for the feed of which
> deployed systems are currently unsafe.
>
> We start with a free CI check, because it's useful to one team on day one with
> no network effect.
>
> Our first beachhead is small and concentrated: only six crates depend on
> halo2_gadgets directly, but it has 1.5 million downloads, because the exposure
> is transitive. That's the part existing tools miss."

**2:05 — Honest status (15s)**

> "Where we are: two programs on Solana devnet, a real message through Hyperlane's
> mailbox, and thirty-nine automated checks. We have no customers yet. [We've
> notified the maintainers of four affected crates and are talking to [N] teams.]
> We found and fixed two holes in our own program, and we published that."

**2:20 — Who you are and close (10s)**

> "[Solo: one sentence on who you are and why you can build this — e.g. what you shipped before.] Halflife: a use-by date for
> cryptographic security."

---

## Notes

- **Do not say** Halflife "detects" or "finds" vulnerabilities. It matches
  published advisories at C1. Say "tells you whether you're affected".
- **Do not say** Hyperlane delivers to other chains. The origin leg is real;
  delivery is not demonstrated.
- The bracketed outreach line must be true when you record it. If it isn't yet,
  cut it. "We have no customers yet" is stronger than a vague claim.
