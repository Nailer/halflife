import json, sys
import numpy as np, soundfile as sf
from kokoro_onnx import Kokoro
from script import SCENES

VOICE = sys.argv[1] if len(sys.argv) > 1 else "am_michael"
SPEED = float(sys.argv[2]) if len(sys.argv) > 2 else 1.08
ONLY = sys.argv[3:]
import os
PREV = {x["id"]: x for x in json.load(open("timeline.json"))} if ONLY and os.path.exists("timeline.json") else {}
GAP = 0.22  # breath between sentences

k = Kokoro("kokoro-v1.0.onnx", "voices-v1.0.bin")
out, total = [], 0.0
for sc in SCENES:
    if ONLY and sc["id"] not in ONLY:
        out.append(PREV[sc["id"]]); total += PREV[sc["id"]]["dur"]; continue
    sr = 24000
    parts = [np.zeros(int(sc["lead"] * sr), dtype=np.float32)]
    t = sc["lead"]
    cues = []
    for ln in sc["lines"]:
        say, cap = ln[0], ln[1]
        hold = ln[2] if len(ln) > 2 else 0.0
        a, sr = k.create(say, voice=VOICE, speed=SPEED, lang="en-us")
        # trim leading/trailing near-silence so cue times are tight
        nz = np.where(np.abs(a) > 0.01)[0]
        if len(nz):
            a = a[max(0, nz[0] - 600): nz[-1] + 1200]
        d = len(a) / sr
        cues.append(dict(start=round(t, 3), end=round(t + d, 3), cap=cap))
        parts += [a.astype(np.float32), np.zeros(int((GAP + hold) * sr), dtype=np.float32)]
        t += d + GAP + hold
    parts.append(np.zeros(int(sc["tail"] * sr), dtype=np.float32))
    audio = np.concatenate(parts)
    dur = len(audio) / sr
    sf.write(f"audio/{sc['id']}.wav", audio, sr)
    out.append(dict(id=sc["id"], dur=round(dur, 3), cues=cues))
    total += dur
    print(f"{sc['id']:14s} {dur:6.2f}s")
json.dump(out, open("timeline.json", "w"), indent=1)
print(f"TOTAL {total:.1f}s")
