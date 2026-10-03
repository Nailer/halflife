# Demo video brief

**Target: 2:30. Hard ceiling 3:00.**

## Read this before anything else

**Do not let a generative video model produce the product footage.**

Tools like Sora, Runway and Veo generate *invented* imagery. Ask one for "a
crypto security dashboard" and it will hallucinate an interface that does not
exist, with numbers nobody measured. For this project specifically that is
fatal: the entire thesis is *no claim without measurement*, and a judge who
compares the video to the live site and finds they do not match has learned the
one thing we cannot afford them to believe.

**Screen-record the real thing.** Every number in this project is real; the
video's job is to show that, not to illustrate it.

Use AI for these and nothing else:

| Safe | Why |
|---|---|
| Voiceover (ElevenLabs, Play.ht) | A clean read of a script you wrote |
| Captions and subtitles | Accessibility, and most people watch muted |
| Title cards and lower thirds | Typography, not fabricated product |
| Cutting and pacing | Assembling footage you recorded |

## Recording setup

- **1920×1080, 60fps.** Browser at 1440px wide, zoomed so text is legible when
  the video is scaled down.
- **Dark theme.** It is the default and the status colours carry further.
- **Hide bookmarks, close other tabs, clear notifications.**
- Run `./scripts/verify-all.sh` **before** recording. If anything is red, fix it
  rather than editing around it.
- Dismiss the guided tour before you start — you are giving it yourself.

## Shot list

| # | Time | On screen | Voiceover |
|---|---|---|---|
| 1 | 0:00–0:12 | Title card: **Halflife — a use-by date for cryptographic security** | "Four cryptographic systems failed in 2026. Every flaw was years old. Every one was found with AI assistance." |
| 2 | 0:12–0:30 | Static card listing the four: Zcash Sprout · Zcash Orchard · Aztec V5 · Coldcard $116M | "The researcher who found the Zcash Orchard bug had already run the same framework over the same code with the previous model generation — and found nothing. The code never changed. The tools got better." |
| 3 | 0:30–0:42 | Terminal: `cargo audit` on a vulnerable tree returning clean | "An audit is a perishable good. And ZK bugs are silent — nothing reverts, nothing shows up in a block explorer." |
| 4 | 0:42–1:05 | **Control room → Impact.** Click `halo2_gadgets 0.4.0`, edges draw out | "This advisory lists its affected crates and then says *and any dependents thereof*. Nothing enumerates them. This does." |
| 5 | 1:05–1:18 | Click `halo2_gadgets 0.5.0` — reaches circuits, affects none | "Same dependency, fixed version. Still reaches circuits. Affects none. Anyone can build an alarm — showing what a healthy dependency doesn't touch is what proves it discriminates." |
| 6 | 1:18–1:45 | **Fire drills → DEPENDENCY COMPROMISE → RUN DRILL.** Gate flips to BLOCKED | "A recorded exercise against Solana devnet. These signatures link to the live explorer. A third-party program refused to proceed — and that check costs 1,520 compute units, under one percent of a transaction budget." |
| 7 | 1:45–2:10 | **RELAYER CENSORSHIP → RUN DRILL.** Gate flips to BLOCKED — STALE | "Now the one that matters. A passport is published, then nothing else is sent. No invalidation. The relayer stops. It blocked anyway, on expiry alone. Withholding a message produces the *safe* outcome." |
| 8 | 2:10–2:22 | Toggle **PUBLIC → OPERATOR**, counts change 4→8, 1→3 | "We found real crates resolving to the vulnerable version. Our disclosure policy needs fourteen days' notice before naming them, so they're embargoed. In the public view they aren't redacted — they're *absent*." |
| 9 | 2:22–2:35 | Terminal: `./scripts/self-passport.sh` → **INVALID** | "Last thing. Halflife issues a passport for itself, under the same rules. It comes back invalid. We published that rather than scoping the scan until it passed." |
| 10 | 2:35–2:45 | End card: live URL, repo URL, two program addresses | — |

## Direction notes

**Let the UI breathe.** When edges draw out in the blast radius, hold two full
seconds without talking. The animation is the argument.

**Never cut away mid-claim.** If the voiceover says 1,520 CU, the number must be
on screen.

**No stock footage.** No abstract glowing networks, no hooded figures, no
spinning globes. Every frame is either the real product, real terminal output,
or plain typography. The restraint is the credibility.

**Pace:** slow for shots 1–3, brisk for 4–6, slowest of all for 7. Shot 7 is the
idea; let it land.

## If you record only 60 seconds

Shots 4, 5, 7, 8. That is: the enumeration nobody has, the proof it
discriminates, the fail-safe, and the embargo. Everything else is context.
