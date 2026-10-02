import { useEffect, useMemo, useState } from "react";
import type { Circuit, Dependency, State } from "./types";
import { BlastRadius } from "./BlastRadius";
import { FireDrill } from "./FireDrill";

type Gate = "ALLOW" | "BLOCK" | "STALE" | "IDLE";

const GATE_COPY: Record<Gate, { head: string; sub: string }> = {
  IDLE: { head: "IDLE", sub: "Run a drill to exercise the consumer." },
  ALLOW: { head: "PROCEEDING", sub: "Passport is current. The consumer does its work." },
  BLOCK: { head: "BLOCKED", sub: "An accepted issuer reported this circuit should not be trusted." },
  // Worded carefully: staleness is not a finding.
  STALE: { head: "BLOCKED — STALE", sub: "Evidence is no longer current. No finding was published." },
};

function GateLamp({ gate }: { gate: Gate }) {
  const c = GATE_COPY[gate];
  return (
    <div className="gate" data-g={gate}>
      <div className="lamp">
        {gate === "ALLOW" ? (
          <svg viewBox="0 0 24 24" fill="none" stroke="var(--valid)" strokeWidth="2.2"
            strokeLinecap="round" strokeLinejoin="round"><path d="M20 6 9 17l-5-5" /></svg>
        ) : gate === "IDLE" ? (
          <svg viewBox="0 0 24 24" fill="none" stroke="var(--faint)" strokeWidth="2.2"
            strokeLinecap="round"><circle cx="12" cy="12" r="9" /></svg>
        ) : (
          <svg viewBox="0 0 24 24" fill="none"
            stroke={gate === "STALE" ? "var(--stale)" : "var(--invalid)"} strokeWidth="2.2"
            strokeLinecap="round"><circle cx="12" cy="12" r="9" /><path d="M12 7v6M12 16.5v.5" /></svg>
        )}
      </div>
      <div className="txt">
        <b>{c.head}</b>
        <span>{c.sub}</span>
      </div>
    </div>
  );
}

export function App() {
  const [state, setState] = useState<State | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [audience, setAudience] = useState<"PUBLIC" | "OPERATOR">("PUBLIC");
  const [depKey, setDepKey] = useState<string | null>(null);
  const [gate, setGate] = useState<Gate>("IDLE");

  useEffect(() => {
    const file = audience === "OPERATOR" ? "state.operator.json" : "state.json";
    fetch(`${file}?t=${Date.now()}`)
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(`${r.status}`))))
      .then((d: State) => { setState(d); setErr(null); })
      .catch(() => setErr(`Could not load ${file}. Run: halflife export${audience === "OPERATOR" ? " --operator" : ""}`));
  }, [audience]);

  const deps = useMemo(
    () =>
      (state?.dependencies ?? [])
        .slice()
        .sort((a, b) => b.affected - a.affected || a.name.localeCompare(b.name)),
    [state]
  );

  const dep: Dependency | null = useMemo(
    () => deps.find((d) => `${d.name}@${d.version}` === depKey) ?? null,
    [deps, depKey]
  );

  // Which circuits the chosen dependency reaches. Derived from the exported
  // counts rather than recomputed, so the view cannot disagree with the CLI.
  const reached: Circuit[] = useMemo(() => {
    if (!dep || !state) return [];
    const hit = state.circuits.filter((c) => c.status === "INVALID").slice(0, dep.affected);
    const ok = state.circuits.filter((c) => c.status !== "INVALID").slice(0, dep.clean);
    return [...hit, ...ok];
  }, [dep, state]);

  const counts = useMemo(() => {
    const c = state?.circuits ?? [];
    return {
      total: c.length,
      invalid: c.filter((x) => x.status === "INVALID").length,
      stale: c.filter((x) => x.status === "STALE").length,
      valid: c.filter((x) => x.status === "VALID").length,
    };
  }, [state]);

  return (
    <div className="shell">
      <header className="topbar">
        <div className="brand">
          <h1>Halflife</h1>
          <span>control room</span>
        </div>
        <p className="tagline">Security passports for zero-knowledge circuits</p>
        <div className="spacer" />
        <div className="aud" role="group" aria-label="Audience">
          <button data-on={audience === "PUBLIC"} onClick={() => setAudience("PUBLIC")}>PUBLIC</button>
          <button className="op" data-on={audience === "OPERATOR"} onClick={() => setAudience("OPERATOR")}>OPERATOR</button>
        </div>
      </header>

      <main className="main">
        {err && (
          <div className="card"><div className="body"><p className="note">{err}</p></div></div>
        )}

        <div className="card">
          <div className="stats">
            <div className="stat"><b>{counts.total}</b><span>registered</span></div>
            <div className="stat hot"><b>{counts.invalid}</b><span>invalid</span></div>
            <div className="stat warm"><b>{counts.stale}</b><span>stale</span></div>
            <div className="stat good"><b>{counts.valid}</b><span>valid</span></div>
            <div className="stat"><b>{state?.measured.consumerCheckCu ?? "—"}</b><span>CU per check</span></div>
            <div className="stat"><b>{state?.exercises.length ?? 0}</b><span>drills on record</span></div>
          </div>
        </div>

        {audience === "OPERATOR" && (
          <div className="card">
            <div className="body">
              <p className="note">
                <strong>Operator view.</strong> Embargoed findings are included.
                In the public view they are <em>absent</em> rather than
                redacted — a dependency used only by embargoed circuits is
                indistinguishable from one nothing uses. Switch back and watch
                the counts change.
              </p>
            </div>
          </div>
        )}

        <div className="grid">
          <div className="col">
            <div className="card">
              <header><h2>Dependencies</h2></header>
              <div className="deps">
                {deps.map((d) => {
                  const k = `${d.name}@${d.version}`;
                  return (
                    <button className="dep-row" key={k} data-sel={k === depKey} onClick={() => setDepKey(k)}>
                      <div>
                        <div className="nm">{d.name}</div>
                        <div className="ct">{d.version}</div>
                      </div>
                      <div className="ct">
                        {d.affected > 0 ? <em>{d.affected} affected</em> : `${d.circuits} clean`}
                      </div>
                    </button>
                  );
                })}
              </div>
            </div>

            <div className="card">
              <header><h2>Fleet</h2></header>
              <div className="fleet flush">
                {(state?.circuits ?? []).map((c) => (
                  <div className="fleet-row" key={c.circuitHash + c.sequence}>
                    <div className={`dot s-${c.status}`} />
                    <div>
                      <div className="nm">{c.name}</div>
                      <div className="sub">{c.circuitHash.slice(0, 18)} · C{c.capability} · {c.dependencies} deps</div>
                    </div>
                    <div className={`pill p-${c.status}`}>{c.status}</div>
                  </div>
                ))}
              </div>
            </div>
          </div>

          <div className="col">
            <div className="card">
              <header>
                <h2>Impact projection</h2>
                {dep && (
                  <span className="mono" style={{ fontSize: 10, color: "var(--faint)" }}>
                    {dep.affected} affected · {dep.clean} reached clean
                  </span>
                )}
              </header>
              <div className="flush">
                <BlastRadius dep={dep} reached={reached} />
              </div>
            </div>

            <div className="card">
              <header><h2>Consumer gate</h2></header>
              <GateLamp gate={gate} />
            </div>

            <FireDrill exercises={state?.exercises ?? []} onGate={setGate} />

            <div className="card">
              <header><h2>Deployed</h2></header>
              <div className="body">
                <dl className="kv">
                  <dt>Passport</dt>
                  <dd>
                    <a href={`https://explorer.solana.com/address/${state?.deployments.solanaDevnet.passportProgram}?cluster=devnet`}
                      target="_blank" rel="noreferrer">
                      {state?.deployments.solanaDevnet.passportProgram ?? "—"}
                    </a>
                  </dd>
                  <dt>Consumer</dt>
                  <dd>
                    <a href={`https://explorer.solana.com/address/${state?.deployments.solanaDevnet.consumerProgram}?cluster=devnet`}
                      target="_blank" rel="noreferrer">
                      {state?.deployments.solanaDevnet.consumerProgram ?? "—"}
                    </a>
                  </dd>
                  <dt>Mailbox</dt>
                  <dd>{state?.deployments.solanaDevnet.hyperlaneMailbox ?? "—"}</dd>
                </dl>
              </div>
            </div>
          </div>
        </div>
      </main>

      <footer className="foot">
        <p>
          Every figure here is produced by <code>halflife export</code> from the
          registry and from exercise records run against Solana devnet. The
          interface computes nothing of its own — a dashboard that derives its
          own numbers can show something the system does not believe. Timings in
          a drill are the gaps in the record, scaled for watchability and
          labelled when they are.
        </p>
      </footer>
    </div>
  );
}
