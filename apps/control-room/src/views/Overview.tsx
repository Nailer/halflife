import type { State } from "../types";
import { Arrow, Play } from "../icons";

export function Overview({
  state,
  counts,
  go,
}: {
  state: State | null;
  counts: { total: number; invalid: number; stale: number; valid: number };
  go: (v: string) => void;
}) {
  return (
    <div className="view">
      <div className="card hero" data-tour="hero">
        <h3>A use-by date for cryptographic security.</h3>
        <p>
          A circuit's audit tells you what someone knew then. A passport tells
          your program what has actually been verified, at what capability,
          about the exact build it is trusting — and blocks it when that
          evidence stops being current.
        </p>
        <div className="cta">
          <button className="btn primary" onClick={() => go("impact")}>
            Project a blast radius <Arrow />
          </button>
          <button className="btn" onClick={() => go("drills")}>
            <Play /> Run a fire drill
          </button>
        </div>
      </div>

      <div className="card" data-tour="stats">
        <div className="stats">
          <button className="stat" onClick={() => go("fleet")}>
            <b>{counts.total}</b>
            <span>registered</span>
          </button>
          <button className="stat hot" onClick={() => go("fleet")}>
            <b>{counts.invalid}</b>
            <span>invalid</span>
          </button>
          <button className="stat warm" onClick={() => go("fleet")}>
            <b>{counts.stale}</b>
            <span>stale</span>
          </button>
          <button className="stat good" onClick={() => go("fleet")}>
            <b>{counts.valid}</b>
            <span>valid</span>
          </button>
          <div className="stat">
            <b>{state?.measured.consumerCheckCu ?? "—"}</b>
            <span>CU per check</span>
          </div>
          <button className="stat" onClick={() => go("drills")}>
            <b>{state?.exercises.length ?? 0}</b>
            <span>drills on record</span>
          </button>
        </div>
      </div>

      <div className="grid g-half">
        <div className="card">
          <header><h2>What a passport blocks on</h2></header>
          <div className="body">
            <div style={{ display: "grid", gap: 13 }}>
              {[
                ["VALID", "Evidence is current. The program proceeds."],
                ["STALE", "Evidence aged out. Not a finding — nothing was discovered, the claim simply is not fresh."],
                ["INVALID", "An accepted issuer reported the circuit should no longer be trusted."],
                ["NONE", "No passport at all. Absence is not permission; this blocks too."],
              ].map(([k, d]) => (
                <div key={k} style={{ display: "grid", gridTemplateColumns: "76px 1fr", gap: 12, alignItems: "start" }}>
                  <span className={`pill p-${k}`}>{k}</span>
                  <p className="note" style={{ margin: 0 }}>{d}</p>
                </div>
              ))}
            </div>
          </div>
        </div>

        <div className="card">
          <header><h2>Deployed</h2></header>
          <div className="body">
            <dl className="kv">
              <dt>Passport</dt>
              <dd>
                <a href={`https://explorer.solana.com/address/${state?.deployments.solanaDevnet.passportProgram}?cluster=devnet`} target="_blank" rel="noreferrer">
                  {state?.deployments.solanaDevnet.passportProgram.slice(0, 22) ?? "—"}…
                </a>
              </dd>
              <dt>Consumer</dt>
              <dd>
                <a href={`https://explorer.solana.com/address/${state?.deployments.solanaDevnet.consumerProgram}?cluster=devnet`} target="_blank" rel="noreferrer">
                  {state?.deployments.solanaDevnet.consumerProgram.slice(0, 22) ?? "—"}…
                </a>
              </dd>
              <dt>Mailbox</dt>
              <dd>{state?.deployments.solanaDevnet.hyperlaneMailbox.slice(0, 22) ?? "—"}…</dd>
              <dt>Cluster</dt>
              <dd>solana devnet</dd>
            </dl>
            <p className="note" style={{ marginTop: 14 }}>
              Both programs are live. A check costs{" "}
              <strong>{state?.measured.consumerCheckCu} CU</strong> — 0.76% of a
              default transaction budget, which is why it can sit inside a hot
              path rather than beside one.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
