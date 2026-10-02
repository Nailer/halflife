export function About() {
  const rows: [string, string][] = [
    ["Lineage", "Resolve a circuit's dependency closure and match every registry package against published advisories. No model involved — reproducible offline by anyone."],
    ["Passport", "A 125-byte signed claim: which circuit, which issuer, what capability was executed, when it expires. Three independent implementations agree on every byte."],
    ["Registry", "Published on Solana. Passport accounts are read-only in the consumer path, so any number of programs can check the same passport in one slot without contending."],
    ["Propagation", "Hyperlane carries the state to other chains. It authenticates transport — it establishes what Solana said, not that the claim is true. Solana did that part."],
    ["Enforcement", "The consumer owns the policy. Halflife publishes state and never decides what a program requires."],
  ];
  return (
    <div className="view">
      <div className="card hero">
        <h3>How it works</h3>
        <p>
          Five layers, each with one job. The boundaries between them are the
          design — collapsing any two would turn this from infrastructure into a
          trusted third party.
        </p>
      </div>

      <div className="card">
        <div className="flush">
          {rows.map(([k, v], i) => (
            <div key={k} style={{ display: "grid", gridTemplateColumns: "clamp(90px,16vw,140px) 1fr", gap: 16, padding: "15px", borderBottom: i < rows.length - 1 ? "1px solid var(--line)" : "none" }}>
              <div className="mono" style={{ fontSize: 11, fontWeight: 700, letterSpacing: "0.08em", textTransform: "uppercase", color: "var(--live)" }}>{k}</div>
              <p className="note" style={{ margin: 0 }}>{v}</p>
            </div>
          ))}
        </div>
      </div>

      <div className="grid g-half">
        <div className="card">
          <header><h2>The invariant</h2></header>
          <div className="body">
            <p className="note" style={{ fontSize: 15, color: "var(--text)", fontWeight: 600, lineHeight: 1.45 }}>
              No consumer may treat an unrefreshed security claim as valid
              indefinitely.
            </p>
            <p className="note" style={{ marginTop: 11 }}>
              Everything else is machinery for holding that true. A passport
              cannot assert its own freshness — <code>STALE</code> is derived by
              the reader against its own clock, never issued.
            </p>
            <p className="note">
              Which means <strong>suppressing delivery produces the safe
              outcome.</strong> Withhold a message and the consumer blocks; there
              is no path where losing the network results in continuing to trust
              old state.
            </p>
          </div>
        </div>

        <div className="card">
          <header><h2>What is not claimed</h2></header>
          <div className="body">
            <p className="note">
              Halflife does not prove a circuit is secure. It records what was
              tested, with which capability, against which exact build, and
              makes that machine-readable. Every stronger reading is wrong.
            </p>
            <p className="note">
              The current tier is <strong>C1</strong> — dependency and advisory
              analysis. Higher tiers exist in the specification and are not
              claimed, because no tier is claimed without a published
              measurement behind it.
            </p>
            <p className="note">
              Embargo via staleness is <strong>quieter than publishing an
              invalidation, not confidential.</strong> Passport state is public
              on-chain and correlatable. Anyone relying on it for secrecy should
              not.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
