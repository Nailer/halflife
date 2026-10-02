import { useEffect, useRef, useState } from "react";
import type { Exercise, ExerciseEvent } from "./types";

const EXPLORER = "https://explorer.solana.com/tx";

function kindLabel(k: string) {
  return k.replace(/_/g, " ");
}

/** Terminal markers get their own colour: block is red, a deliberate stop amber. */
function term(e: ExerciseEvent): string | undefined {
  if (e.kind === "CONSUMER_BLOCKED") return "block";
  if (e.kind === "RELAYER_STOPPED" || e.kind === "PASSPORT_EXPIRED") return "stop";
  return undefined;
}

function evidenceNode(e: ExerciseEvent) {
  const ev = e.evidence;
  if (ev.type === "SOLANA_TRANSACTION") {
    return (
      <>
        <a href={`${EXPLORER}/${ev.signature}?cluster=devnet`} target="_blank" rel="noreferrer">
          {ev.signature.slice(0, 10)}…
        </a>
        <br />
        slot {ev.slot}
      </>
    );
  }
  if (ev.type === "HYPERLANE_MESSAGE") return <>msg {ev.message_id.slice(0, 10)}…</>;
  // Labelled, never dressed up as on-chain evidence.
  return <span className="loc">local</span>;
}

/**
 * Plays a recorded exercise back at its real intervals.
 *
 * The gaps between rows are the gaps in the record — scaled so a 41-second
 * censorship drill is watchable, but never invented. The scale factor is shown,
 * because a timeline that silently compresses time is a timeline that lies.
 */
export function FireDrill({
  exercises,
  onGate,
}: {
  exercises: Exercise[];
  onGate: (g: "ALLOW" | "BLOCK" | "STALE" | "IDLE") => void;
}) {
  const [idx, setIdx] = useState(0);
  const [fired, setFired] = useState<number>(0);
  const [playing, setPlaying] = useState(false);
  const timers = useRef<number[]>([]);

  const ex = exercises[idx];

  const clear = () => {
    timers.current.forEach(clearTimeout);
    timers.current = [];
  };

  useEffect(() => clear, []);

  useEffect(() => {
    clear();
    setFired(0);
    setPlaying(false);
    onGate("IDLE");
  }, [idx]);

  if (!ex) {
    return (
      <div className="card">
        <header><h2>Fire drill</h2></header>
        <div className="body">
          <p className="note">
            No exercise records found. Run one against devnet:
            <br />
            <code>halflife-exercise run dependency-compromise</code>
          </p>
        </div>
      </div>
    );
  }

  const first = ex.events[0]?.observed_at ?? 0;
  const last = ex.events[ex.events.length - 1]?.observed_at ?? 0;
  const span = Math.max(1, last - first);
  // Fit any drill into roughly six seconds of playback.
  const scale = Math.min(1, 6 / span);

  const play = () => {
    clear();
    setFired(0);
    setPlaying(true);
    onGate("ALLOW");
    ex.events.forEach((e, i) => {
      const at = (e.observed_at - first) * scale * 1000;
      const t = window.setTimeout(() => {
        setFired(i + 1);
        if (e.kind === "CONSUMER_BLOCKED") {
          onGate(ex.scenario === "RELAYER_CENSORSHIP" ? "STALE" : "BLOCK");
        }
        if (i === ex.events.length - 1) setPlaying(false);
      }, at + 220);
      timers.current.push(t);
    });
  };

  const containment = last - first;
  const verifiable = ex.events.filter((e) => e.evidence.type !== "LOCAL").length;

  return (
    <div className="card">
      <header>
        <h2>Fire drill</h2>
        <span className="mono" style={{ fontSize: 10, color: "var(--faint)" }}>
          {ex.cluster}
        </span>
      </header>

      <div className="drill-head">
        <div className="scen">
          {exercises.map((e, i) => (
            <button key={e.exercise_id} data-on={i === idx} onClick={() => setIdx(i)}>
              {e.scenario === "RELAYER_CENSORSHIP" ? "RELAYER CENSORSHIP" : "DEPENDENCY COMPROMISE"}
            </button>
          ))}
        </div>
        <div className="spacer" />
        <button className="play" onClick={play} disabled={playing}>
          {playing ? "RUNNING…" : fired ? "REPLAY" : "RUN DRILL"}
        </button>
      </div>

      {ex.scenario === "RELAYER_CENSORSHIP" && (
        <div style={{ padding: "12px 15px", borderBottom: "1px solid var(--line)" }}>
          <p className="note">
            <strong>Nothing is published after the baseline.</strong> No
            invalidation, no second write — the consumer blocks on expiry alone.
            Withholding a message produces the safe outcome.
          </p>
        </div>
      )}

      <div className="timeline">
        {ex.events.map((e, i) => (
          <div className="tl-row" key={e.seq} data-fired={i < fired} data-term={term(e)}>
            <div className="sq">{String(e.seq).padStart(2, "0")}</div>
            <div className="rail"><div className="mk" /></div>
            <div>
              <div className="k">{kindLabel(e.kind)}</div>
              <div className="d">{e.detail}</div>
            </div>
            <div className="ev">{evidenceNode(e)}</div>
          </div>
        ))}
      </div>

      <div className="containment">
        <b>{containment}s</b>
        <span>containment, measured</span>
        <div className="spacer" />
        <span>
          {verifiable}/{ex.events.length} independently verifiable
          {scale < 1 && ` · playback ${Math.round(1 / scale)}×`}
        </span>
      </div>
    </div>
  );
}
