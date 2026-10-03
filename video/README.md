# Demo video — how it is made

**Output:** `docs/submission/halflife-demo.mp4` (1920×1080, 30fps, ~2:57) and
`halflife-demo.srt` (captions, also burned in).

This follows [`docs/submission/video-brief.md`](../docs/submission/video-brief.md):
**every product frame is the real control room**, not generated imagery.

## What is real, and how

| Shot | Source |
|---|---|
| Control room (Overview, Impact, Fire drills, Public/Operator) | The actual `apps/control-room`, driven in headless Chromium, reading the committed `state.json` / `state.operator.json` |
| CLI | Verbatim output of `OFFLINE=1 ./scripts/demo.sh` (pinned OSV snapshot) replayed in a terminal frame |
| Self-passport | Figures from `docs/self-passport.md` (653 packages, 15 advisories, INVALID) |
| Title / problem / passport / why-Solana cards | Typography and diagrams only; every number is one the repo measures or documents |

Overlays (cursor, highlight rings, callouts, captions) annotate the UI; they never
change what it renders. The fire-drill playback runs on the app's own timers.

## Deterministic capture

`rec.cjs` steps time instead of screen-recording it: Playwright's fake clock
(paused) drives the app's timers, and every CSS animation/transition is paused
and seeked to the frame time. Each frame is a lossless 1920×1080 screenshot, so
the result is perfectly smooth regardless of machine speed.

## Rebuild

Requires Node 22 with Playwright + Chromium, ffmpeg, Python 3 with
`kokoro-onnx soundfile numpy`, and the Kokoro model files
(`kokoro-v1.0.onnx`, `voices-v1.0.bin` from the kokoro-onnx GitHub releases)
in this directory. Fonts: Archivo + JetBrains Mono woff2 in `fonts/` with
`fonts.css` (Google Fonts CSS, URLs rewritten to `/__fonts/<file>`).

```bash
npm run dev --prefix apps/control-room -- --port 5174 --host 127.0.0.1 &
cd video
python3 tts.py am_michael 1.2      # narration -> audio/*.wav + timeline.json
node scenes_app.cjs                # real control-room scenes -> clips/
node scenes_cards.cjs              # typography scenes -> clips/
python3 music.py 177.2             # synthesized ambient bed -> music.wav
python3 assemble.py ../docs/submission/halflife-demo.mp4
```

The narration script is `script.py`. Voice: Kokoro `am_michael` (open-weight TTS).
Music: synthesized in `music.py`, so it carries no licence.
