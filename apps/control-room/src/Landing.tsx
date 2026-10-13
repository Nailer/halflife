import { useEffect, useRef, useState } from "react";
import { Sun, Moon, Arrow, NavDocs } from "./icons";
import type { Theme } from "./theme";
import "./landing.css";

const GH = "https://github.com/Nailer/halflife";
// One curve is used everywhere: the logo, the intro stroke, the hero path.
// It is the shape of decay — evidence as issued, then aged, then refused.
const CURVE = "M60 360 C 160 130, 330 90, 580 90";

const reduced = () =>
  typeof window !== "undefined" && !!window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;

function useReveal() {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const root = ref.current;
    if (!root) return;
    const els = root.querySelectorAll<HTMLElement>(".lp-reveal");
    if (reduced() || !("IntersectionObserver" in window)) {
      els.forEach((e) => e.classList.add("in"));
      return;
    }
    const io = new IntersectionObserver(
      (entries) =>
        entries.forEach((en) => {
          if (en.isIntersecting) {
            en.target.classList.add("in");
            io.unobserve(en.target);
          }
        }),
      { threshold: 0.14 }
    );
    els.forEach((e) => io.observe(e));
    return () => io.disconnect();
  }, []);
  return ref;
}

function Intro({ onDone }: { onDone: () => void }) {
  const [leaving, setLeaving] = useState(false);
  useEffect(() => {
    const t1 = setTimeout(() => setLeaving(true), 2900);
    const t2 = setTimeout(onDone, 3600);
    const key = () => {
      setLeaving(true);
      setTimeout(onDone, 450);
    };
    window.addEventListener("keydown", key, { once: true });
    return () => {
      clearTimeout(t1);
      clearTimeout(t2);
      window.removeEventListener("keydown", key);
    };
  }, [onDone]);

  return (
    <div className="lp-intro" data-leaving={leaving} role="presentation">
      <div className="lp-intro-inner">
        <svg viewBox="0 0 640 440" aria-hidden="true">
          <defs>
            <linearGradient id="ig" gradientUnits="userSpaceOnUse" x1="60" y1="0" x2="580" y2="0">
              <stop offset="0" stopColor="#3fb950" />
              <stop offset="0.5" stopColor="#d29922" />
              <stop offset="1" stopColor="#f85149" />
            </linearGradient>
          </defs>
          <path d={CURVE} pathLength={1000} className="lp-intro-curve" stroke="url(#ig)" />
          <circle cx="580" cy="90" r="13" className="lp-intro-dot" fill="#f85149" />
        </svg>
        <div className="lp-intro-word">Halflife</div>
        <div className="lp-intro-tag">a use-by date for cryptographic security</div>
      </div>
      <button
        className="lp-skip"
        onClick={() => {
          setLeaving(true);
          setTimeout(onDone, 450);
        }}
      >
        Skip intro
      </button>
    </div>
  );
}

// The hero art. Mounted only after the intro, so its SMIL timeline starts when
// the viewer can actually see it. One 9-second loop: a passport is issued
// (green), ages (amber), expires (red) and the program stops accepting it.
function Pipeline() {
  const D = "9s";
  const cyc = (values: string, keyTimes: string, attr: string) => (
    <animate attributeName={attr} values={values} keyTimes={keyTimes} dur={D} repeatCount="indefinite" />
  );
  return (
    <svg viewBox="0 0 640 440" className="lp-art" role="img"
      aria-label="A dependency lockfile is scanned, becomes a signed passport that ages from valid to stale to invalid, and a Solana program stops accepting it.">
      <defs>
        <linearGradient id="pg" gradientUnits="userSpaceOnUse" x1="60" y1="0" x2="580" y2="0">
          <stop offset="0" stopColor="#3fb950" />
          <stop offset="0.5" stopColor="#d29922" />
          <stop offset="1" stopColor="#f85149" />
        </linearGradient>
        <filter id="soft" x="-50%" y="-50%" width="200%" height="200%">
          <feGaussianBlur stdDeviation="6" />
        </filter>
      </defs>

      <path d={CURVE} className="lp-track" pathLength={1000} />
      <path d={CURVE} className="lp-draw" pathLength={1000} stroke="url(#pg)" />

      {/* nodes */}
      <g className="lp-node" style={{ ["--n" as string]: 0 }}>
        <circle cx="60" cy="360" r="8" className="lp-dot" />
        <rect x="24" y="378" width="172" height="52" rx="10" className="lp-card" />
        <text x="40" y="400" className="lp-t">Cargo.lock</text>
        <text x="40" y="418" className="lp-s">your dependencies</text>
      </g>
      <g className="lp-node" style={{ ["--n" as string]: 1 }}>
        <circle cx="169" cy="200" r="8" className="lp-dot" />
        <rect x="190" y="214" width="176" height="52" rx="10" className="lp-card" />
        <text x="206" y="236" className="lp-t">Scan</text>
        <text x="206" y="254" className="lp-s">matched to advisories</text>
      </g>
      <g className="lp-node" style={{ ["--n" as string]: 2 }}>
        <circle cx="318" cy="119" r="24" className="lp-ringbg" />
        <circle cx="318" cy="119" r="24" className="lp-ring" transform="rotate(-90 318 119)"
          strokeDasharray="150.8" fill="none">
          {cyc("0;0;150.8;150.8", "0;0.05;0.78;1", "stroke-dashoffset")}
          {cyc("#3fb950;#3fb950;#d29922;#f85149;#f85149", "0;0.2;0.45;0.78;1", "stroke")}
        </circle>
        <circle cx="318" cy="119" r="6" className="lp-dot" />
        <rect x="236" y="152" width="190" height="52" rx="10" className="lp-card" />
        <text x="252" y="174" className="lp-t">Passport</text>
        <text x="252" y="192" className="lp-s">125 bytes · signed · expires</text>
      </g>
      <g className="lp-node" style={{ ["--n" as string]: 3 }}>
        {/* the gate: open (short) while valid, closed (full) once expired */}
        <rect x="574" y="52" width="12" height="76" rx="6" className="lp-gatebg" />
        <rect x="574" y="52" width="12" height="18" rx="6" fill="#3fb950">
          {cyc("18;18;18;76;76", "0;0.2;0.62;0.8;1", "height")}
          {cyc("#3fb950;#3fb950;#d29922;#f85149;#f85149", "0;0.2;0.45;0.78;1", "fill")}
        </rect>
        <rect x="462" y="146" width="160" height="52" rx="10" className="lp-card" />
        <text x="478" y="168" className="lp-t">Your program</text>
        <text x="478" y="186" className="lp-s">checks it · 1,520 CU</text>
        <g className="lp-status" textAnchor="end">
          <text x="560" y="36" fill="#3fb950">ACCEPTS
            {cyc("1;1;0;0;0", "0;0.2;0.3;0.99;1", "opacity")}
          </text>
          <text x="560" y="36" fill="#d29922" opacity="0">STALE · NEEDS REFRESH
            {cyc("0;0;1;1;0;0", "0;0.28;0.36;0.62;0.72;1", "opacity")}
          </text>
          <text x="560" y="36" fill="#f85149" opacity="0">REFUSES
            {cyc("0;0;1;1", "0;0.7;0.8;1", "opacity")}
          </text>
        </g>
      </g>

      {/* the evidence, travelling and ageing */}
      <g className="lp-packet">
        <circle r="13" fill="#3fb950" opacity="0.35" filter="url(#soft)">
          <animateMotion dur={D} repeatCount="indefinite" path={CURVE} keyPoints="0;1;1" keyTimes="0;0.78;1" calcMode="linear" />
          {cyc("#3fb950;#3fb950;#d29922;#f85149;#f85149", "0;0.2;0.45;0.78;1", "fill")}
        </circle>
        <circle r="7" fill="#3fb950">
          <animateMotion dur={D} repeatCount="indefinite" path={CURVE} keyPoints="0;1;1" keyTimes="0;0.78;1" calcMode="linear" />
          {cyc("#3fb950;#3fb950;#d29922;#f85149;#f85149", "0;0.2;0.45;0.78;1", "fill")}
          {cyc("0;1;1;0", "0;0.03;0.93;1", "opacity")}
        </circle>
      </g>
    </svg>
  );
}

const STEPS = [
  { n: "01", t: "Scan", d: "Halflife reads your Cargo.lock, resolves every dependency (including your dependencies' dependencies), and matches the whole closure against published advisories." },
  { n: "02", t: "Sign a passport", d: "The result becomes a 125-byte signed passport: which circuit, which exact build, what was checked, at what depth, and until when." },
  { n: "03", t: "Enforce on Solana", d: "Your program reads the passport before it accepts a proof. The check costs 1,520 compute units, under 1% of a transaction." },
  { n: "04", t: "Expire by default", d: "A passport can't claim its own freshness. If nobody renews it, or a message is censored, it lapses and the program stops. Absence is what causes the block." },
];

const PROBLEMS = [
  { t: "An audit is a snapshot", d: "An audit records what could be found with the tools of its day. Those tools keep improving, and nobody tracks how stale an audit has become. In July 2026 a critical flaw in the code under Zcash's Orchard circuit was published (CVE-2026-54496)." },
  { t: "Zero-knowledge bugs are silent", d: "A broken contract reverts. A broken circuit produces a valid-looking proof of a false statement. Nothing fails, so nobody notices." },
  { t: "Stopping takes people", d: "When a flaw is published, halting a protocol takes a meeting, a vote, signers and a redeploy. Money keeps moving the whole time." },
];

export function Landing({ theme, onTheme }: { theme: Theme; onTheme: () => void }) {
  const [ready, setReady] = useState(reduced());
  const root = useReveal();
  useEffect(() => {
    document.body.style.overflow = ready ? "" : "hidden";
    return () => {
      document.body.style.overflow = "";
    };
  }, [ready]);

  return (
    <div className="lp" ref={root} data-ready={ready}>
      {!ready && <Intro onDone={() => setReady(true)} />}

      <div className="lp-bg" aria-hidden="true">
        <i className="b1" /><i className="b2" /><i className="b3" />
      </div>

      <header className="lp-nav">
        <a className="lp-brand" href="#" aria-label="Halflife home">
          <svg viewBox="0 0 64 64" aria-hidden="true"><path d="M12 44 C 20 18, 32 14, 52 14" fill="none" stroke="#3fb950" strokeWidth="6" strokeLinecap="round" /><path d="M12 44 C 20 18, 32 14, 52 14" fill="none" stroke="#d29922" strokeWidth="6" strokeLinecap="round" strokeDasharray="26 100" /><circle cx="52" cy="14" r="6" fill="#f85149" /></svg>
          Halflife
        </a>
        <nav aria-label="Sections">
          <a href="#how" onClick={(e) => { e.preventDefault(); document.getElementById("how")?.scrollIntoView({ behavior: "smooth" }); }}>How it works</a>
          <a href="#try" onClick={(e) => { e.preventDefault(); document.getElementById("try")?.scrollIntoView({ behavior: "smooth" }); }}>Try it</a>
          <a href="https://halflife.mintlify.site/" target="_blank" rel="noopener noreferrer">Docs</a>
          <a href={GH} target="_blank" rel="noreferrer">GitHub</a>
        </nav>
        <a className="lp-icon lp-docs" href="https://halflife.mintlify.site/" target="_blank" rel="noopener noreferrer" aria-label="Documentation" title="Documentation"><NavDocs /></a>
        <button className="lp-icon" onClick={onTheme} aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} theme`}>
          {theme === "dark" ? <Sun /> : <Moon />}
        </button>
        <a className="lp-btn primary sm" href="#app/overview">Enter the control room <Arrow /></a>
      </header>

      <main>
        <section className="lp-hero">
          <div className="lp-copy">
            <p className="lp-kicker lp-rise" style={{ ["--d" as string]: "0ms" }}>Security infrastructure for zero-knowledge protocols on Solana</p>
            <h1 className="lp-rise" style={{ ["--d" as string]: "90ms" }}>
              A use-by date for<br /><span className="lp-grad">cryptographic security.</span>
            </h1>
            <p className="lp-lede lp-rise" style={{ ["--d" as string]: "200ms" }}>
              Zero-knowledge systems depend on cryptography that gets old without anyone
              noticing. Halflife gives each circuit a signed, <strong>expiring</strong> passport, and
              your Solana program refuses to trust one that has lapsed or been flagged,
              automatically, before an attack, not after it.
            </p>
            <div className="lp-cta lp-rise" style={{ ["--d" as string]: "310ms" }}>
              <a className="lp-btn primary" href="#app/overview">Enter the control room <Arrow /></a>
              <a className="lp-btn" href="#app/check">Check your own project</a>
            </div>
            <p className="lp-note lp-rise" style={{ ["--d" as string]: "400ms" }}>
              Open source · Apache-2.0 · live on Solana devnet · no sign-up
            </p>
          </div>
          <div className="lp-stage lp-rise" style={{ ["--d" as string]: "150ms" }}>
            {ready && <Pipeline />}
            <p className="lp-cap">Watch one passport age. The program that reads it stops trusting it on its own.</p>
          </div>
        </section>

        <section className="lp-sec">
          <p className="lp-eyebrow lp-reveal">Why this exists</p>
          <h2 className="lp-reveal">The cryptography under your protocol has a shelf life. Nobody is tracking it.</h2>
          <div className="lp-cards">
            {PROBLEMS.map((p, i) => (
              <article key={p.t} className="lp-card2 lp-reveal" style={{ ["--d" as string]: `${i * 90}ms` }}>
                <h3>{p.t}</h3>
                <p>{p.d}</p>
              </article>
            ))}
          </div>
        </section>

        <section className="lp-sec" id="how">
          <p className="lp-eyebrow lp-reveal">What it does</p>
          <h2 className="lp-reveal">Four steps from a lockfile to a program that stops itself.</h2>
          <ol className="lp-steps">
            {STEPS.map((s, i) => (
              <li key={s.n} className="lp-reveal" style={{ ["--d" as string]: `${i * 90}ms` }}>
                <span className="lp-num">{s.n}</span>
                <div><h3>{s.t}</h3><p>{s.d}</p></div>
              </li>
            ))}
          </ol>
        </section>

        <section className="lp-sec" id="try">
          <p className="lp-eyebrow lp-reveal">Try it, nothing to install</p>
          <h2 className="lp-reveal">Three ways to see it work for yourself.</h2>
          <div className="lp-cards">
            <a className="lp-card2 link lp-reveal" href="#app/check">
              <h3>Check your project</h3>
              <p>Paste a <code>Cargo.lock</code>. Your browser asks OSV about each crate and returns a would-pass / would-fail verdict. Names and versions only; nothing goes to our servers.</p>
              <span className="lp-go">Open the checker <Arrow /></span>
            </a>
            <div className="lp-card2 lp-reveal" style={{ ["--d" as string]: "90ms" }}>
              <h3>Add the GitHub Action</h3>
              <p>Fail a pull request that pulls a vulnerable dependency in, before anyone merges it.</p>
              <pre className="lp-code">{`- uses: Nailer/halflife@main
  with:
    path: .`}</pre>
              <a className="lp-go" href={GH} target="_blank" rel="noreferrer">See it on GitHub <Arrow /></a>
            </div>
            <a className="lp-card2 link lp-reveal" style={{ ["--d" as string]: "180ms" }} href="#app/fleet">
              <h3>Read the chain yourself</h3>
              <p>Query Solana devnet straight from your browser and decode a real passport account. No file, no server of ours in between.</p>
              <span className="lp-go">Read from devnet <Arrow /></span>
            </a>
          </div>
        </section>

        <section className="lp-sec">
          <p className="lp-eyebrow lp-reveal">Measured, not claimed</p>
          <div className="lp-stats lp-reveal">
            <div><b>1,520</b><span>compute units per check</span></div>
            <div><b>125</b><span>bytes per passport</span></div>
            <div><b>39</b><span>automated checks pass</span></div>
            <div><b>2</b><span>programs live on devnet</span></div>
          </div>
          <div className="lp-honest lp-reveal">
            <h3>What it isn't, yet</h3>
            <p>
              Halflife matches <strong>published</strong> advisories (capability C1). It does not discover new
              bugs, and its check is deliberately blunt: any advisory counts. It has no customers yet, and its own
              on-chain program is unaudited. We found and fixed two access-control holes in it ourselves, and
              wrote them up in the open.
            </p>
          </div>
        </section>

        <section className="lp-final lp-reveal">
          <h2>See what a use-by date looks like.</h2>
          <div className="lp-cta" style={{ justifyContent: "center" }}>
            <a className="lp-btn primary" href="#app/overview">Enter the control room <Arrow /></a>
            <a className="lp-btn" href={GH} target="_blank" rel="noreferrer">View the source</a>
          </div>
        </section>
      </main>

      <footer className="lp-foot">
        <span>Halflife · Apache-2.0 · built solo for Colosseum's Crypto World's Fair</span>
        <a href={GH} target="_blank" rel="noreferrer">github.com/Nailer/halflife</a>
      </footer>
    </div>
  );
}
