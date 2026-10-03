import { useMemo, useState } from "react";
import type { Circuit, State } from "../types";
import { Search } from "../icons";
import { LiveRead } from "../LiveRead";

export function Fleet({ state }: { state: State | null }) {
  const [q, setQ] = useState("");
  const [sel, setSel] = useState<Circuit | null>(null);

  const rows = useMemo(() => {
    const all = state?.circuits ?? [];
    const needle = q.trim().toLowerCase();
    const order = { INVALID: 0, STALE: 1, NONE: 2, VALID: 3 } as const;
    return all
      .filter((c) => !needle || c.name.toLowerCase().includes(needle) || c.circuitHash.includes(needle))
      .slice()
      .sort((a, b) => order[a.status] - order[b.status] || a.name.localeCompare(b.name));
  }, [state, q]);

  const current = sel ?? rows[0] ?? null;

  return (
    <div className="view">
      <div className="grid g-2">
        <div className="card" data-tour="fleet">
          <header><h2>Fleet · {rows.length}</h2></header>
          <div className="search">
            <Search />
            <input
              value={q}
              onChange={(e) => setQ(e.target.value)}
              placeholder="filter by name or hash…"
              aria-label="Filter circuits"
            />
          </div>
          <div className="scroll">
            {rows.map((c) => (
              <button
                key={c.circuitHash + c.sequence}
                className="row row-fleet"
                data-sel={current?.circuitHash === c.circuitHash}
                onClick={() => setSel(c)}
              >
                <div className={`dot s-${c.status}`} />
                <div>
                  <div className="nm">{c.name}</div>
                  <div className="sub">{c.circuitHash.slice(0, 18)} · C{c.capability} · {c.dependencies} deps</div>
                </div>
                <div className={`pill p-${c.status}`}>{c.status}</div>
              </button>
            ))}
            {rows.length === 0 && (
              <div style={{ padding: 20 }}>
                <p className="note">Nothing matches “{q}”.</p>
              </div>
            )}
          </div>
        </div>

        <div className="col">
        <div className="card">
          <header><h2>Circuit detail</h2></header>
          {current ? (
            <div className="body">
              <div style={{ display: "flex", alignItems: "center", gap: 11, marginBottom: 16 }}>
                <div className={`dot s-${current.status}`} style={{ width: 11, height: 11 }} />
                <div style={{ fontSize: 17, fontWeight: 700, letterSpacing: "-0.02em" }}>{current.name}</div>
                <div className="spacer" />
                <div className={`pill p-${current.status}`}>{current.status}</div>
              </div>
              <dl className="kv">
                <dt>Circuit</dt><dd>{current.circuitHash}</dd>
                <dt>Repository</dt><dd>{current.repository}</dd>
                <dt>Commit</dt><dd>{current.commit}</dd>
                <dt>Proof system</dt><dd>{current.proofSystem}</dd>
                <dt>Capability</dt><dd>C{current.capability} — dependency and advisory analysis</dd>
                <dt>Dependencies</dt><dd>{current.dependencies}</dd>
                <dt>Sequence</dt><dd>{current.sequence}</dd>
                <dt>Issuer</dt><dd>{current.issuer || "—"}</dd>
              </dl>
              <p className="note" style={{ marginTop: 16 }}>
                <strong>Capability is a claim about method, not safety.</strong>{" "}
                C{current.capability} says a dependency closure was resolved and
                matched against published advisories. It does not say the circuit
                is sound — no tier is claimed here without a published
                measurement behind it.
              </p>
            </div>
          ) : (
            <div className="body"><p className="note">No circuits registered.</p></div>
          )}
        </div>

        <LiveRead circuitHash={current?.circuitHash} issuerHex={current?.issuer} />
        </div>
      </div>
    </div>
  );
}
