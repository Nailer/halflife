import { useEffect, useRef, useState } from "react";
import type { Exercise, ExerciseEvent } from "../types";
import { Play, Check, Alert, Idle } from "../icons";

type Gate = "ALLOW" | "BLOCK" | "STALE" | "IDLE";

const GATE: Record<Gate, { head: string; sub: string }> = {
  IDLE: { head: "IDLE", sub: "Run a drill to exercise the consumer." },
  ALLOW: { head: "PROCEEDING", sub: "Passport is current. The consumer does its work." },
  BLOCK: { head: "BLOCKED", sub: "An accepted issuer reported this circuit should not be trusted." },
  STALE: { head: "BLOCKED — STALE", sub: "Evidence is no longer current. No finding was published." },
};

const EXPLORER = "https://explorer.solana.com/tx";

function term(e: ExerciseEvent) {
  if (e.kind === "CONSUMER_BLOCKED") return "block";
  if (e.kind === "RELAYER_STOPPED" || e.kind === "PASSPORT_EXPIRED") return "stop";
  return undefined;
}

function Evidence({ e }: { e: ExerciseEvent }) {
  const ev = e.evidence;
  if (ev.type === "SOLANA_TRANSACTION")
    return (
      <>
        <a href={`${EXPLORER}/${ev.signature}?cluster=devnet`} target="_blank" rel="noreferrer">
          {ev.signature.slice(0, 10)}…
        </a>
        <br />
        slot {ev.slot}
      </>
    );
  if (ev.type === "HYPERLANE_MESSAGE") return <>msg {ev.message_id.slice(0, 10)}…</>;
  // Labelled, never dressed up as on-chain evidence.
  return <span className="loc">local</span>;
}

function Lamp({ gate }: { gate: Gate }) {
  const c = GATE[gate];
  return (
    <div className="gate" data-g={gate} data-tour="gate">
      <div className="lamp" style={{ color: gate === "ALLOW" ? "var(--valid)" : gate === "STALE" ? "var(--stale)" : gate === "BLOCK" ? "var(--invalid)" : "var(--faint)" }}>
        {gate === "ALLOW" ? <Check /> : gate === "IDLE" ? <Idle /> : <Alert />}
      </div>
      <div className="txt">
        <b>{c.head}</b>
        <span>{c.sub}</span>
      </div>
    </div>
  );
}

export function Drills({ exercises }: { exercises: Exercise[] }) {
  const [idx, setIdx] = useState(0);
  const [fired, setFired] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [gate, setGate] = useState<Gate>("IDLE");
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
    setGate("IDLE");
  }, [idx]);

  if (!ex)
    return (
      <div className="view">
        <div className="card">
          <header><h2>Fire drill</h2></header>
          <div className="body">
            <p className="note">
              No exercise records. Run one against devnet:
              <br />
              <code>halflife-exercise run dependency-compromise</code>
            </p>
          </div>
        </div>
      </div>
    );

  const first = ex.events[0]?.observed_at ?? 0;
  const last = ex.events[ex.events.length - 1]?.observed_at ?? 0;
  const span = Math.max(1, last - first);
  const scale = Math.min(1, 6 / span);

  const play = () => {
    clear();
    setFired(0);
    setPlaying(true);
    setGate("ALLOW");
    ex.events.forEach((e, i) => {
      const at = (e.observed_at - first) * scale * 1000 + 240;
      timers.current.push(
        window.setTimeout(() => {
          setFired(i + 1);
          if (e.kind === "CONSUMER_BLOCKED")
            setGate(ex.scenario === "RELAYER_CENSORSHIP" ? "STALE" : "BLOCK");
          if (i === ex.events.length - 1) setPlaying(false);
        }, at)
      );
    });
  };

  const verifiable = ex.events.filter((e) => e.evidence.type !== "LOCAL").length;
  const pct = (fired / ex.events.length) * 100;

  return (
    <div className="view">
      <div className="grid g-2">
        <div className="col">
          <div className="card">
            <header><h2>Consumer gate</h2></header>
            <Lamp gate={gate} />
          </div>

          <div className="card">
            <header><h2>Scenario</h2></header>
            <div className="body">
              <p className="note">
                {ex.scenario === "RELAYER_CENSORSHIP" ? (
                  <>
                    <strong>Nothing is published after the baseline.</strong> No
                    invalidation, no second write — the consumer blocks on expiry
                    alone. Withholding a message produces the safe outcome, which
                    is the property the whole design rests on.
                  </>
                ) : (
                  <>
                    A dependency becomes unsafe, a passport is invalidated, and
                    the consumer refuses to proceed. This is the <em>fast</em>{" "}
                    path — a message is published and delivered.
                  </>
                )}
              </p>
              <p className="note">
                Every event below was recorded against Solana devnet. Signatures
                link to the live explorer; events computed locally say so rather
                than sitting alongside on-chain evidence as equals.
              </p>
            </div>
          </div>
        </div>

        <div className="card" data-tour="drill">
          <header>
            <h2>Fire drill</h2>
            <span className="mono" style={{ fontSize: 10, color: "var(--faint)" }}>{ex.cluster}</span>
          </header>

          <div className="drill-head">
            <div className="seg">
              {exercises.map((e, i) => (
                <button key={e.exercise_id} data-on={i === idx} onClick={() => setIdx(i)}>
                  {e.scenario === "RELAYER_CENSORSHIP" ? "RELAYER CENSORSHIP" : "DEPENDENCY COMPROMISE"}
                </button>
              ))}
            </div>
            <div className="spacer" />
            <button className="play" onClick={play} disabled={playing}>
              <Play /> {playing ? "RUNNING…" : fired ? "REPLAY" : "RUN DRILL"}
            </button>
          </div>

          <div className="progress"><i style={{ width: `${pct}%` }} /></div>

          <div className="timeline">
            {ex.events.map((e, i) => (
              <div className="tl-row" key={e.seq} data-fired={i < fired} data-term={term(e)}>
                <div className="sq">{String(e.seq).padStart(2, "0")}</div>
                <div className="rail"><div className="mk" /></div>
                <div>
                  <div className="k">{e.kind.replace(/_/g, " ")}</div>
                  <div className="d">{e.detail}</div>
                </div>
                <div className="ev"><Evidence e={e} /></div>
              </div>
            ))}
          </div>

          <div className="containment">
            <b>{last - first}s</b>
            <span>containment, measured</span>
            <div className="spacer" />
            <span>
              {verifiable}/{ex.events.length} verifiable
              {scale < 1 && ` · playback ${Math.round(1 / scale)}×`}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
