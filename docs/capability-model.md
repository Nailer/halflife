# Capability model

A passport records **what was executed**, not how confident anyone feels. The
tier is a statement about method, which is checkable, rather than about safety,
which is not.

| Tier | Name | What was actually done |
|---|---|---|
| C0 | None | No automated verification. |
| C1 | Structural | Dependency closure resolved; every registry package matched against published advisories. |
| C2 | Constraint consistency | The circuit's own constraint system checked for internal consistency. |
| C3 | Differential | Behaviour compared against a known-good reference implementation. |
| C4 | Adversarial soundness | Active search for inputs that satisfy the constraint system but violate the intended statement. |
| C5 | Independent methodology | Re-verified under a methodology newer than the one that last cleared it. |

**v0.1 emits C1 only.** Nothing in this repository performs constraint analysis
yet, and claiming a higher tier would be the precise failure this project exists
to expose.

## Why a ladder and not a score

A decay percentage — "this circuit is 73% fresh" — is not defensible. It implies
a measurable rate of decay that nobody can derive.

The ladder makes the claim falsifiable instead:

```
circuit requires  C4
passport asserts  C1
                  -> INSUFFICIENT
```

That is a comparison a program can make and a human can argue with.

## What "half-life" actually means here

Not "audits expire after N days." A verification's *evidentiary strength* decays
when the analysis frontier advances past the methodology that produced it.

This is an argument from how analysis tools improve, not a measured result: we
do not claim a specific before-and-after run. The sourced incidents behind it,
including the `halo2_gadgets` flaw behind CVE-2026-54496, are listed in the
README.

So a historical audit answers *what was knowable and testable then*. It does not
answer *what is detectable now*. A C1 passport does not mean a circuit is
broken — it means nothing above C1 has been asserted about it, and a consumer
requiring C4 should say so.
