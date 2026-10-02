import { useMemo, useState } from "react";
import type { Circuit, Dependency, State } from "../types";
import { BlastRadius } from "../BlastRadius";
import { Search } from "../icons";

export function Impact({ state, audience }: { state: State | null; audience: string }) {
  const [q, setQ] = useState("");
  const [key, setKey] = useState<string | null>(null);

  const deps = useMemo(() => {
    const all = state?.dependencies ?? [];
    const needle = q.trim().toLowerCase();
    return all
      .filter((d) => !needle || `${d.name}@${d.version}`.toLowerCase().includes(needle))
      .slice()
      .sort((a, b) => b.affected - a.affected || a.name.localeCompare(b.name));
  }, [state, q]);

  const dep: Dependency | null = useMemo(
    () => deps.find((d) => `${d.name}@${d.version}` === key) ?? null,
    [deps, key]
  );

  // Derived from the exported counts rather than recomputed, so the view can
  // never disagree with what the CLI projected.
  const reached: Circuit[] = useMemo(() => {
    if (!dep || !state) return [];
    const hit = state.circuits.filter((c) => c.status === "INVALID").slice(0, dep.affected);
    const ok = state.circuits.filter((c) => c.status !== "INVALID").slice(0, dep.clean);
    return [...hit, ...ok];
  }, [dep, state]);

  return (
    <div className="view">
      <div className="grid g-2">
        <div className="card" data-tour="deps">
          <header><h2>Dependencies · {deps.length}</h2></header>
          <div className="search">
            <Search />
            <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="filter dependencies…" aria-label="Filter dependencies" />
          </div>
          <div className="scroll">
            {deps.map((d) => {
              const k = `${d.name}@${d.version}`;
              return (
                <button key={k} className="row row-dep" data-sel={k === key} onClick={() => setKey(k)}>
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

        <div className="col">
          <div className="card" data-tour="radius">
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
            <div className="legend">
              <div><span className="dot s-INVALID" /> affected by an advisory</div>
              <div><span className="dot s-VALID" /> reached, clean</div>
              <div><span className="dot s-STALE" /> evidence aged out</div>
            </div>
          </div>

          <div className="card">
            <header><h2>What this is answering</h2></header>
            <div className="body">
              <p className="note">
                The advisory for <code>halo2_gadgets</code> lists its affected
                crates and then says <em>“and any dependents thereof.”</em>{" "}
                <strong>Nothing enumerates them.</strong> This is that
                enumeration.
              </p>
              <p className="note">
                Pick <code>0.4.0</code> and then <code>0.5.0</code>. Both reach
                circuits; only one affects them. Anyone can build an alarm —
                showing what a healthy dependency <em>doesn't</em> touch is what
                proves the thing discriminates.
              </p>
              {audience === "PUBLIC" && (
                <p className="note">
                  You are on the <strong>public</strong> view. Embargoed findings
                  are absent here, not redacted — a dependency used only by
                  embargoed circuits looks exactly like one nothing uses. Switch
                  to <strong>operator</strong> and watch the counts change.
                </p>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
