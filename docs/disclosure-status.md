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

What is said publicly before the clock runs: *four published crates, unnamed in
the live exports, resolve to affected versions. Maintainers have **not** yet
been notified, so the notice period has not started.*

## Where this is still inconsistent

**The repository names them.** `fixtures/fleet/` contains a directory per crate
and `scripts/build-fleet.py` lists all four. The repository is public, so the
names are public, even though the live exports no longer carry them.

That is a real inconsistency and it is recorded here rather than left for
someone else to notice. The fixtures exist because the findings have to be
reproducible — a scan nobody can re-run is not evidence — and removing them
would trade one kind of honesty for another. The underlying facts are already
public: the advisory is published, and each crate's dependency requirement is
visible on crates.io. What our policy treats as disclosure is the *aggregation*,
and the aggregation is what the repository performs.

The resolution is to notify the maintainers, which has not been done.

## A leak that was live

The public export carried all four names until 3 October 2026.

The circuit list filtered correctly. The dependency index did not — a fixture's
own root package name appears inside its closure, so listing dependencies
without an audience filter published exactly the names the circuit filter was
hiding. The leak tests covered `reached_by` and `visible_circuits` and never
covered `known_dependencies`.

Fixed, with the test that should have existed
(`the_dependency_index_does_not_leak_embargoed_names`). The lesson is the
general one: filtering has to cover every surface, and a leak test that covers
the obvious path is the one most likely to miss.

## Known limit, restated

Embargo via staleness is **quieter than publishing `INVALID`, not
confidential**. Passport state is public on-chain and correlatable. Anyone
relying on this for secrecy should not.
