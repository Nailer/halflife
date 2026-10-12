# Development provenance

Colosseum asks teams to disclose all relevant past development work. Stated
plainly:

## What existed before the hackathon

**Nothing in this repository.** The hackathon window opened on 14 September 2026.
The first commit is **23 September 2026**, and the repository has 37 commits
through 7 October 2026 (`git log --reverse`).

A research phase began on 21 September, which produced reports on Solana, Zcash,
Hyperlane, Monero, Aztec, Penumbra and Namada. That is analysis, not code, and it
is also inside the window.

The founder's other repositories (a personal portfolio site) are unrelated and are
not part of this submission.

## Third-party code

Everything we depend on is open source and unmodified: the Solana and Anchor
toolchains, Hyperlane's deployed mailbox (we call it, we do not fork it), the
crates in `Cargo.lock`, and the packages in `package-lock.json`.

The test fixtures contain **genuine crates.io versions and checksums**. The
closures for the six real packages in `fixtures/fleet/` are *derived* (each
published requirement resolved to the highest satisfying release), not those
crates' own lockfiles, and each fixture says so in its header.

`LICENSE` is the unmodified Apache-2.0 text.

## How it was built

**The code was written by Claude (Anthropic), working through Claude Code under
the founder's direction.** The founder set the thesis and the decisions, reviewed
the output, ran the deployments, and owns the result. The implementation, tests,
documentation and interface were produced by the model.

We state this because it is true, because the rules treat misrepresenting
development history as grounds for disqualification, and because the project's
whole argument is about AI-assisted work: that capability changes between
releases, and that claims about what was verified should say how.

Two consequences worth stating. The tests were written by the same model as the
code, so they share its blind spots, which is why the wire format is checked by
three implementations and why we ran a negative control that must fail. And the
model made mistakes that we found and recorded rather than hid: a replay test that
passed for the wrong reason, a public export that leaked embargoed names, and two
access-control holes in the on-chain program (`docs/security-notes.md`).
