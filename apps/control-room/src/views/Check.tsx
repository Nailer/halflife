import { useState } from "react";

// A visitor's own lockfile, checked in their browser against OSV. Nothing here
// is signed and nothing touches our servers: the page sends crate names and
// versions to api.osv.dev, exactly as `halflife scan` does, and shows the
// verdict the C1 rule would give. It is a preview of the CLI, not a passport.

type Pkg = { name: string; version: string };
type Hit = { pkg: Pkg; ids: string[] };
type Result = { total: number; registry: number; hits: Hit[]; checkedAt: string };

const OSV = "https://api.osv.dev/v1/querybatch";

export function parseLock(text: string): Pkg[] {
  const out: Pkg[] = [];
  const seen = new Set<string>();
  for (const block of text.split(/\[\[package\]\]/).slice(1)) {
    const name = /^name\s*=\s*"([^"]+)"/m.exec(block)?.[1];
    const version = /^version\s*=\s*"([^"]+)"/m.exec(block)?.[1];
    const source = /^source\s*=\s*"([^"]+)"/m.exec(block)?.[1] ?? "";
    // Registry crates only: a path or git dependency has no OSV crates.io entry.
    if (!name || !version || !/^(registry|sparse)\+/.test(source)) continue;
    const key = `${name}@${version}`;
    if (!seen.has(key)) {
      seen.add(key);
      out.push({ name, version });
    }
  }
  return out;
}

async function query(pkgs: Pkg[]): Promise<Hit[]> {
  const hits: Hit[] = [];
  for (let i = 0; i < pkgs.length; i += 500) {
    const chunk = pkgs.slice(i, i + 500);
    const res = await fetch(OSV, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        queries: chunk.map((p) => ({ package: { name: p.name, ecosystem: "crates.io" }, version: p.version })),
      }),
    });
    if (!res.ok) throw new Error(`OSV answered ${res.status}`);
    const body = (await res.json()) as { results: { vulns?: { id: string }[] }[] };
    body.results.forEach((r, j) => {
      const ids = (r.vulns ?? []).map((v) => v.id);
      if (ids.length) hits.push({ pkg: chunk[j], ids });
    });
  }
  return hits.sort((a, b) => a.pkg.name.localeCompare(b.pkg.name));
}

const EXAMPLES = [
  { id: "orchard-vulnerable", label: "Example: pinned to the vulnerable halo2_gadgets" },
  { id: "orchard-patched", label: "Example: patched to 0.5.0" },
];

export function Check() {
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [res, setRes] = useState<Result | null>(null);

  async function run(source = text) {
    setErr(null);
    setRes(null);
    const pkgs = parseLock(source);
    if (pkgs.length === 0) {
      setErr("No registry packages found. Paste the contents of a Cargo.lock (it should contain [[package]] blocks).");
      return;
    }
    setBusy(true);
    try {
      const hits = await query(pkgs);
      setRes({ total: source.split(/\[\[package\]\]/).length - 1, registry: pkgs.length, hits, checkedAt: new Date().toISOString() });
    } catch (e) {
      setErr(`Could not reach OSV from your browser (${(e as Error).message}). Nothing was checked, which is not the same as clean.`);
    } finally {
      setBusy(false);
    }
  }

  async function load(id: string) {
    const r = await fetch(`examples/${id}.lock`);
    const t = await r.text();
    setText(t);
    run(t);
  }

  const bad = res && res.hits.length > 0;

  return (
    <div className="view">
      <div className="grid g-2">
        <div className="card">
          <header><h2>Check your own project</h2></header>
          <div className="body">
            <p className="note">
              Paste a <code>Cargo.lock</code>. Your browser sends each crate's <strong>name and version</strong> to
              OSV (api.osv.dev) and nothing else — no source code, and nothing to a server of ours.
            </p>
            <textarea
              className="paste"
              value={text}
              onChange={(e) => setText(e.target.value)}
              placeholder={'[[package]]\nname = "halo2_gadgets"\nversion = "0.4.0"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\n…'}
              spellCheck={false}
              aria-label="Cargo.lock contents"
            />
            <div className="cta-row">
              <button className="btn primary" disabled={busy || !text.trim()} onClick={() => run()}>
                {busy ? "Checking…" : "Check dependencies"}
              </button>
              {EXAMPLES.map((e) => (
                <button key={e.id} className="btn" disabled={busy} onClick={() => load(e.id)}>{e.label}</button>
              ))}
            </div>
            {err && <p className="note" style={{ marginTop: 12, color: "var(--invalid)" }}>{err}</p>}
          </div>
        </div>

        <div className="col">
          <div className="card">
            <header><h2>Verdict</h2></header>
            <div className="body">
              {!res && !busy && <p className="note">Run a check to see the result. Try one of the examples.</p>}
              {busy && <p className="note">Asking OSV about your dependencies…</p>}
              {res && (
                <>
                  <div style={{ display: "flex", alignItems: "center", gap: 11, marginBottom: 14 }}>
                    <div className={`dot s-${bad ? "INVALID" : "VALID"}`} style={{ width: 11, height: 11 }} />
                    <div style={{ fontSize: 17, fontWeight: 700, letterSpacing: "-0.02em" }}>
                      {bad ? `${res.hits.length} package${res.hits.length === 1 ? "" : "s"} carry advisories` : "No published advisories found"}
                    </div>
                    <div className="spacer" />
                    <div className={`pill p-${bad ? "INVALID" : "VALID"}`}>{bad ? "WOULD FAIL" : "WOULD PASS"}</div>
                  </div>
                  <p className="note">
                    Checked <strong>{res.registry}</strong> registry packages against OSV at {res.checkedAt.slice(0, 19).replace("T", " ")} UTC.
                    {bad
                      ? " Under the C1 rule the closure would be INVALID and the CI gate would fail the build."
                      : " That means no advisory is published for these exact versions today — not that the code is sound."}
                  </p>
                  {bad && (
                    <div style={{ marginTop: 12 }}>
                      {res.hits.map((h) => (
                        <div key={h.pkg.name + h.pkg.version} className="hit">
                          <div className="nm">{h.pkg.name} <span className="mono dim">{h.pkg.version}</span></div>
                          <div className="ids">
                            {h.ids.map((id) => (
                              <a key={id} href={`https://osv.dev/vulnerability/${id}`} target="_blank" rel="noreferrer">{id}</a>
                            ))}
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </>
              )}
            </div>
          </div>

          <div className="card">
            <header><h2>What this is, and isn't</h2></header>
            <div className="body">
              <p className="note"><strong>A preview of the scanner.</strong> This is the same lookup <code>halflife scan</code> performs at capability C1: the dependency closure matched against published advisories.</p>
              <p className="note"><strong>Not signed, not on-chain.</strong> A passport is signed, expires, and is what a Solana program refuses to proceed without. Create one with the CLI:</p>
              <pre className="snip">{`halflife scan . --deny   # fails the build on any advisory`}</pre>
              <p className="note"><strong>Blunt on purpose.</strong> Any advisory counts, with no severity or reachability filter, so test-only and “unmaintained” notices show up too. Expect noise on large workspaces.</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
