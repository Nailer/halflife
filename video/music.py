# Ambient bed: slow minor pad + soft clock tick. Synthesised, so no licensing.
import json, sys
import numpy as np, soundfile as sf

SR = 48000
L = float(sys.argv[1])
n = int(L * SR)
t = np.arange(n) / SR
rng = np.random.default_rng(7)

def note(f):
    return 440.0 * 2 ** ((f - 69) / 12)

# Am9 - Fmaj7 - Cadd9 - Em7, 8 s each, looping
CH = [[45, 57, 60, 64, 71], [41, 53, 57, 60, 64], [48, 55, 60, 62, 67], [40, 52, 55, 59, 62]]
SEG = 8.0
pad = np.zeros(n)
for i in range(int(L // SEG) + 2):
    ch = CH[i % 4]
    s0 = i * SEG - 1.5
    a = max(0, int(s0 * SR)); b = min(n, int((s0 + SEG + 3.0) * SR))
    if a >= b:
        continue
    tt = t[a:b] - s0
    env = np.clip(tt / 2.2, 0, 1) * np.clip((SEG + 3.0 - tt) / 2.6, 0, 1)
    env = env ** 1.6
    seg = np.zeros(b - a)
    for k, m in enumerate(ch):
        f = note(m)
        for det in (-0.12, 0.0, 0.12):
            ph = rng.uniform(0, 2 * np.pi)
            seg += (np.sin(2 * np.pi * f * (1 + det / 100) * tt + ph)
                    + 0.18 * np.sin(2 * np.pi * 2 * f * tt + ph)) * (0.55 if k == 0 else 0.32)
    pad[a:b] += seg * env

# slow movement
pad *= 0.75 + 0.25 * np.sin(2 * np.pi * t / 11.0)

# one-pole lowpass for warmth
def lp(x, fc):
    a = np.exp(-2 * np.pi * fc / SR); y = np.zeros_like(x); acc = 0.0
    # vectorised-ish via scipy-free IIR in chunks
    for i in range(0, len(x), 48000):
        ch = x[i:i + 48000]; out = np.empty_like(ch)
        for j, v in enumerate(ch):
            acc = (1 - a) * v + a * acc; out[j] = acc
        y[i:i + 48000] = out
    return y
pad = lp(pad, 1400)

# clock tick: one per second, very soft, filtered noise click
tick = np.zeros(n)
click_len = int(0.03 * SR)
ce = np.exp(-np.arange(click_len) / (0.004 * SR))
for s in np.arange(1.0, L - 1.0, 1.0):
    i = int(s * SR)
    c = rng.standard_normal(click_len) * ce
    c = np.diff(np.concatenate([[0], c]))  # brighten
    tick[i:i + click_len] += c * (0.9 if int(s) % 4 == 0 else 0.55)

# sub pulse on the bar
sub = np.zeros(n)
for s in np.arange(0.0, L, 4.0):
    i = int(s * SR); m = min(n, i + int(1.6 * SR))
    tt = t[i:m] - s
    sub[i:m] += np.sin(2 * np.pi * 55 * tt) * np.exp(-tt * 2.2) * np.clip(tt / 0.02, 0, 1)

mix = pad / np.max(np.abs(pad)) * 0.55 + tick * 0.05 + sub * 0.22
# master fades
fi = np.clip(t / 2.5, 0, 1); fo = np.clip((L - t) / 3.0, 0, 1)
mix *= fi * fo
mix /= np.max(np.abs(mix)) + 1e-9
mix *= 0.8
st = np.stack([mix, np.roll(mix, int(0.011 * SR))], axis=1)  # tiny Haas width
sf.write("music.wav", st.astype(np.float32), SR)
print("music", L)
