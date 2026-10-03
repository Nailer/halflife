import { useEffect, useMemo, useState } from "react";
import type { State } from "./types";
import { applyTheme, initialTheme, type Theme } from "./theme";
import { Tour, hasSeenTour, type TourStep } from "./Tour";
import { Sun, Moon, Help } from "./icons";
import { Overview } from "./views/Overview";
import { Impact } from "./views/Impact";
import { Drills } from "./views/Drills";
import { Fleet } from "./views/Fleet";
import { About } from "./views/About";

const VIEWS = [
  { id: "overview", label: "Overview" },
  { id: "impact", label: "Impact" },
  { id: "drills", label: "Fire drills" },
  { id: "fleet", label: "Fleet" },
  { id: "about", label: "How it works" },
] as const;

const STEPS: TourStep[] = [
  {
    target: "hero",
    view: "overview",
    title: "What this is",
    body: "Halflife gives a zero-knowledge circuit a passport — a signed claim about what was actually verified, and when it stops being current. Programs read it and refuse to proceed without one.",
  },
  {
    target: "stats",
    view: "overview",
    title: "The fleet at a glance",
    body: "Registered circuits and their current state. Amber means STALE — the evidence aged out, which is not the same as a vulnerability being found. Every tile is clickable.",
  },
  {
    target: "deps",
    view: "impact",
    title: "Pick a dependency",
    body: "An advisory names its affected crates and then says “and any dependents thereof.” Nothing enumerates them. This does. Try 0.4.0, then 0.5.0 — both reach circuits, only one affects them.",
  },
  {
    target: "drill",
    view: "drills",
    title: "Run a fire drill",
    body: "A real exercise recorded against Solana devnet, replayed at the gaps in its own record. The signatures link to the live explorer — these transactions actually happened.",
  },
  {
    target: "liveread",
    view: "fleet",
    title: "Is any of this real?",
    body: "Fair question. Everything else on the page comes from a file the CLI exported. Press Read from devnet and your browser queries Solana directly, pulls the actual passport account and decodes it here — no file, no server of ours in between.",
  },
  {
    target: "audience",
    view: "overview",
    title: "Public vs operator",
    body: "The important one. Embargoed findings are absent from the public view, not redacted — a dependency used only by embargoed circuits looks exactly like one nothing uses. Flip it and watch the counts change.",
  },
];

export function App() {
  const [theme, setTheme] = useState<Theme>(initialTheme);
  const [state, setState] = useState<State | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [audience, setAudience] = useState<"PUBLIC" | "OPERATOR">("PUBLIC");
  const [view, setView] = useState<string>("overview");
  const [tour, setTour] = useState(false);

  useEffect(() => applyTheme(theme), [theme]);

  // Hold the tour until data has landed, so step one does not spotlight a
  // skeleton.
  useEffect(() => {
    if (state && !hasSeenTour()) {
      const t = setTimeout(() => setTour(true), 650);
      return () => clearTimeout(t);
    }
  }, [state]);

  useEffect(() => {
    const file = audience === "OPERATOR" ? "state.operator.json" : "state.json";
    fetch(`${file}?t=${Date.now()}`)
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(String(r.status)))))
      .then((d: State) => {
        setState(d);
        setErr(null);
      })
      .catch(() =>
        setErr(`Could not load ${file}. Run: halflife export${audience === "OPERATOR" ? " --operator" : ""}`)
      );
  }, [audience]);

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

        <nav className="nav" aria-label="Views">
          {VIEWS.map((v) => (
            <button key={v.id} data-on={view === v.id} onClick={() => setView(v.id)}>
              {v.label}
            </button>
          ))}
        </nav>

        <div className="spacer" />

        <div className="aud" role="group" aria-label="Audience" data-tour="audience">
          <button data-on={audience === "PUBLIC"} onClick={() => setAudience("PUBLIC")}>PUBLIC</button>
          <button className="op" data-on={audience === "OPERATOR"} onClick={() => setAudience("OPERATOR")}>OPERATOR</button>
        </div>

        <button className="icon-btn" onClick={() => setTour(true)} aria-label="Take the tour" title="Take the tour">
          <Help />
        </button>
        <button
          className="icon-btn"
          onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
          aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} theme`}
          title={`Switch to ${theme === "dark" ? "light" : "dark"} theme`}
        >
          {theme === "dark" ? <Sun /> : <Moon />}
        </button>
      </header>

      <main className="main">
        {err && (
          <div className="card">
            <div className="body"><p className="note">{err}</p></div>
          </div>
        )}

        {audience === "OPERATOR" && (
          <div className="card">
            <div className="body">
              <p className="note">
                <strong>Operator view.</strong> Embargoed findings are included.
                In the public view they are <em>absent</em> rather than redacted —
                a dependency used only by embargoed circuits is indistinguishable
                from one nothing uses.
              </p>
            </div>
          </div>
        )}

        {view === "overview" && <Overview state={state} counts={counts} go={setView} />}
        {view === "impact" && <Impact state={state} audience={audience} />}
        {view === "drills" && <Drills exercises={state?.exercises ?? []} />}
        {view === "fleet" && <Fleet state={state} />}
        {view === "about" && <About />}
      </main>

      <footer className="foot">
        <p>
          Every figure here is produced by <code>halflife export</code> from the
          registry and from exercises run against Solana devnet. The interface
          computes nothing of its own — a dashboard that derives its own numbers
          can show something the system does not believe. Drill timings are the
          gaps in the record, scaled for watchability and labelled when they are.
        </p>
      </footer>

      {tour && <Tour steps={STEPS} onView={setView} onDone={() => setTour(false)} />}
    </div>
  );
}
