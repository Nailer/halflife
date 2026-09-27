# Disclosure policy

Halflife computes findings about software other people maintain. This policy
governs what we do with them. It exists before the first scan, not after the
first finding.

## What we scan

Publicly published packages and their resolved dependency closures, drawn from
public registries. We do not scan private repositories, and we do not attempt to
access anything not already public.

## What we publish, and when

A finding is **derived** the moment a scan completes. It is **published** only
after the steps below.

1. **Existing public advisories.** Where a finding consists solely of matching a
   package against an already-published advisory — the case for every C1
   finding — the underlying vulnerability is public and its identifier is
   citable. We may publish that a registered system's closure matches it.

2. **Named third parties.** Before publishing that a specific third-party
   package is affected, we notify its maintainers and allow **14 days** for
   response. This applies even when the advisory is already public, because
   aggregation is itself a disclosure: "this advisory exists" and "these four
   named projects are exposed to it" are different statements with different
   consequences.

3. **Novel findings.** Anything we discover that is not already published is
   treated as an embargoed vulnerability and follows coordinated disclosure:
   private report to maintainers, **90 days** or until a fix ships, whichever is
   sooner, then publication.

We publish no proof-of-concept exploit code for an unfixed vulnerability.

## Embargo handling

While a finding is embargoed, the affected passport is **not** published as
`INVALID`, because an explicit invalidation names the problem by implication.
Instead we **stop renewing** it. The passport expires, consumers resolve
`STALE`, and they block — with no public statement about the cause.

### The limit of that mechanism, stated plainly

**Embargo via staleness is quieter than publishing `INVALID`. It is not
confidential.**

Passports live on a public chain. If several circuits go stale together while
others stay valid, an observer can correlate that directly from chain state. No
interface decision prevents this, because we are not rendering the correlation —
we are publishing it.

Two partial mitigations exist and neither makes it silent:

- Randomised expiry windows per circuit, so some fraction is always near expiry
- Staggered renewal cadence, so benign staleness is ordinary rather than notable

There is a real tension here worth recording. Renewal must be cheap and
automatic, or operators widen expiry windows and the fail-safe erodes. But
reliable renewal makes staleness rare, and rare staleness is informative. The
embargo mechanism needs staleness to be common enough to hide in.

Anyone relying on embargo-via-staleness for confidentiality should not. It buys
quiet, not secrecy.

## Contact

Security reports: see `SECURITY.md`. We accept encrypted reports and will
acknowledge within 72 hours.

## What we will not do

- Publish a finding about a named third party without prior notification
- Publish exploit code for an unfixed vulnerability
- Represent an embargoed finding as confidential
- Publish a finding we cannot reproduce from a committed script
