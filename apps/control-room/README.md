# Control room

The interface for Halflife. Reads state produced by the CLI; computes nothing
of its own.

```bash
halflife export                      # public view  -> public/state.json
halflife export --operator           # operator view -> public/state.operator.json
npm install && npm run dev
```

## Why the UI derives nothing

Every number shown comes from `halflife export`. A dashboard that computes its
own figures can display something the system does not actually believe, and the
difference is invisible until it matters. So the counts, the affected sets and
the drill timings are all read, never recomputed.

Where the interface *does* transform anything, it says so: a drill is played
back at the gaps in its own record, scaled to stay watchable, and the scale
factor is printed next to it.

## The audience toggle

The headline control. **PUBLIC** and **OPERATOR** load different exports.

Embargoed findings are not redacted in the public view — they are **absent**. A
dependency used only by embargoed circuits looks exactly like one nothing uses.
Switching between the two is the clearest demonstration of the property, because
the counts change rather than a lock icon appearing.

## Colour

Chrome is monochrome deliberately. In an instrument panel colour carries
meaning, so it is reserved entirely for state:

| | |
|---|---|
| green | `VALID` |
| amber | `STALE` — evidence aged out, **not** a finding |
| red | `INVALID` — an issuer reported a problem |
| grey | `NONE` — no passport at all, which blocks |

Amber for stale is a deliberate refusal to paint it red. Collapsing the two
would make every delivery hiccup look like a vulnerability, which is exactly the
confusion the protocol is built to avoid.
