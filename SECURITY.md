# Security

## Reporting

Report vulnerabilities privately. We acknowledge within 72 hours and accept
encrypted reports.

Do not open a public issue for a security finding.

## Our own disclosure conduct

Halflife computes findings about software other people maintain. How we handle
those is governed by [`docs/disclosure-policy.md`](docs/disclosure-policy.md),
which exists before our first scan rather than after our first finding.

Summary: we notify maintainers before publishing anything that names them, we
publish no exploit code for unfixed vulnerabilities, and we state plainly that
embargo via staleness is quieter than publishing `INVALID` but is **not**
confidential.

## Status

v1.0.0 freezes the passport wire format. The verification engine is not built;
the tool claims capability **C1** — dependency and advisory analysis — and
nothing above it. No tier is claimed without measurement.
