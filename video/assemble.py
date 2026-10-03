import json, subprocess, sys

X = 0.35  # crossfade
FPS = 30
tl = json.load(open("timeline.json"))
out = sys.argv[1] if len(sys.argv) > 1 else "halflife-demo.mp4"

def frames(p):
    r = subprocess.run(["ffprobe", "-v", "error", "-count_frames", "-select_streams", "v:0", "-show_entries",
                        "stream=nb_read_frames", "-of", "csv=p=0", p], capture_output=True, text=True)
    return int(r.stdout.strip())

durs = [frames(f"clips/{s['id']}.mp4") / FPS for s in tl]
for s, d in zip(tl, durs):
    if abs(d - s["dur"]) > 0.05:
        print("WARN duration", s["id"], d, s["dur"])

args = ["ffmpeg", "-y", "-loglevel", "error", "-stats"]
for s in tl:
    args += ["-i", f"clips/{s['id']}.mp4"]
for s in tl:
    args += ["-i", f"audio/{s['id']}.wav"]
args += ["-i", "music.wav"]
N = len(tl)

fc, offs = [], []
prev, acc = "[0:v]", 0.0
offs.append(0.0)
for i in range(1, N):
    acc += durs[i - 1] - X
    offs.append(acc)
    lab = f"[v{i}]"
    fc.append(f"{prev}[{i}:v]xfade=transition=fade:duration={X}:offset={acc:.3f}{lab}")
    prev = lab
total = acc + durs[-1]
fc.append(f"{prev}fade=t=out:st={total - 0.6:.3f}:d=0.6,format=yuv420p[vout]")

for i in range(N):
    ms = int(round(offs[i] * 1000))
    fc.append(f"[{N + i}:a]aresample=48000,aformat=channel_layouts=stereo,adelay={ms}|{ms},apad=whole_dur={total:.3f}[a{i}]")
fc.append("".join(f"[a{i}]" for i in range(N)) + f"amix=inputs={N}:normalize=0:duration=longest,atrim=0:{total:.3f}[voice]")
fc.append("[voice]asplit=2[vx][vsc]")
fc.append(f"[{2 * N}:a]atrim=0:{total:.3f},volume=0.30[mus]")
fc.append("[mus][vsc]sidechaincompress=threshold=0.02:ratio=6:attack=40:release=600:makeup=1[duck]")
fc.append("[vx][duck]amix=inputs=2:normalize=0:duration=first,loudnorm=I=-16:TP=-1.5:LRA=11[aout]")

args += ["-filter_complex", ";".join(fc), "-map", "[vout]", "-map", "[aout]",
         "-c:v", "libx264", "-preset", "slow", "-crf", "17", "-profile:v", "high", "-r", str(FPS),
         "-c:a", "aac", "-b:a", "192k", "-ar", "48000", "-movflags", "+faststart", out]
print(f"total {total:.2f}s")
subprocess.run(args, check=True)

# chapter list / captions file for the description and accessibility
def ts(x):
    h, r = divmod(x, 3600); m, s = divmod(r, 60)
    return f"{int(h):02d}:{int(m):02d}:{s:06.3f}".replace(".", ",")
srt, k = [], 1
for s, o in zip(tl, offs):
    for c in s["cues"]:
        srt.append(f"{k}\n{ts(o + c['start'])} --> {ts(o + c['end'] + 0.2)}\n{c['cap']}\n"); k += 1
open(out.rsplit(".", 1)[0] + ".srt", "w").write("\n".join(srt))
