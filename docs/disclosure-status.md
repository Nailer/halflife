# Disclosure status

Live tracking of our obligations under [`disclosure-policy.md`](disclosure-policy.md).

## Current position

**Four published crates in the registry resolve to `halo2_gadgets` versions
affected by [GHSA-ww9q-8r59-xv46](https://osv.dev/vulnerability/GHSA-ww9q-8r59-xv46).**
They are not named on any public surface, and this repository is private, until
the notification period has run.

| | |
|---|---|
| Finding derived | 28 September 2026 |
| Maintainers notified | *pending* |
| Notice period | 14 days from notification |
| Public naming permitted | 14 days after notification |

## Why the repository is private

Pushing this repository publicly would name those four projects as affected.
Our own policy calls that a disclosure — *"this advisory exists"* and *"these
four named projects are exposed to it"* are different statements with different
consequences — and requires 14 days' notice first.

The 14-day clock does not finish before the Colosseum deadline of 12 October.
We considered amending the policy so the dates worked out. We did not, because a
project whose premise is *no claim without measurement* cannot quietly relax its
own rules when they become inconvenient. The policy was written first precisely
so it would bind under pressure.

## How the finding is presented before the clock runs

The system already handles this; it is what disclosure-aware projection is for.

- **Public audience** — `halflife impact halo2_gadgets@0.4.0` returns only the
  fixtures. The four real crates are absent entirely, and the result is
  indistinguishable from a dependency nothing uses.
- **Operator audience** — `--operator` returns the full affected set.

So the demonstration shows both views side by side. That is not a workaround for
a limitation; it is the embargo mechanism running on a real embargo, which is
better evidence than the names would have been.

What is said publicly before the clock runs: *four published crates, unnamed,
resolve to affected versions; maintainers have been notified; they will be named
once the notice period ends.*

## Known limit, restated

Embargo via staleness is **quieter than publishing `INVALID`, not
confidential**. Passport state is public on-chain and correlatable. Anyone
relying on this for secrecy should not.
