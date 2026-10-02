# Control room

**https://halflife-control-room.vercel.app**

Five views, a guided tour, light and dark themes. Reads state produced by the
CLI; computes nothing of its own.

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

## The tour

Five steps, spotlighting real elements. It switches views when a step's target
lives elsewhere, waits for the view transition to settle before measuring, and
remembers that you have seen it. The `?` button in the header replays it.

The scrim is a single enormous `box-shadow` on one moving rectangle rather than
four stitched panels, which is what keeps the transition between steps smooth.

## Themes

Light and dark, toggled in the header and persisted. Status colours are
redefined per theme rather than reused — amber on white needs a different value
than amber on near-black to carry the same weight — but the meanings never move.

Every storage access is wrapped: a private window that throws on `localStorage`
loses the preference, not the page.
