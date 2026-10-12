// Real-product scenes: the landing page and the control room, driven on cue
// and annotated. Live calls (OSV, Solana devnet) are real, relayed at capture.
const fs = require('fs');
const { open, record } = require('./rec.cjs');
const TL = Object.fromEntries(JSON.parse(fs.readFileSync('timeline.json')).map((s) => [s.id, s]));
const CSS = fs.readFileSync('common.css', 'utf8');
const OVJS = fs.readFileSync('overlay.js', 'utf8');
const BASE = 'http://127.0.0.1:5174/';

const A = (t, code) => ({ at: t, fn: (p) => p.evaluate(code) });
const cue = (id, i) => TL[id].cues[i].start;
const cueEnd = (id, i) => TL[id].cues[i].end;

function captions(id) {
  const cues = TL[id].cues, out = [];
  cues.forEach((c, i) => {
    out.push({ at: c.start, fn: (p) => p.evaluate((t) => OV.caption(t), c.cap) });
    const next = cues[i + 1];
    if (!next || next.start - c.end > 0.7) out.push(A(c.end + 0.35, `OV.caption(null)`));
  });
  return out;
}

// eased scroll so `sel` sits `off` px below the top of the viewport
const scroll = (from, to, sel, off = 90) => [
  A(from - 0.01, `window.__sy0 = window.scrollY`),
  { from, to, fn: (p, u) => p.evaluate(([sel, off, u]) => {
      const el = typeof sel === 'string' ? document.querySelector(sel) : null;
      const y1 = el ? el.getBoundingClientRect().top + window.scrollY - off : sel;
      window.scrollTo(0, window.__sy0 + (y1 - window.__sy0) * u);
    }, [sel, off, u]) },
];

const DEP = (name, ver) => `[...document.querySelectorAll('.row-dep')].find(r => r.querySelector('.nm').textContent === '${name}' && r.textContent.includes('${ver}'))`;
const CARD = (txt) => `[...document.querySelectorAll('.card')].find(c => c.querySelector('header') && c.querySelector('header').textContent.toLowerCase().includes('${txt}'))`;
const BTN = (txt) => `[...document.querySelectorAll('button, a')].find(b => b.textContent.trim().startsWith('${txt}'))`;

const SCENES = {
  // The real landing page: intro stroke, then the hero and its decay pipeline.
  s01_title: {
    url: BASE, vignette: false, warm: false,
    actions: (id) => [],
  },

  s05_room: {
    url: BASE + '#app/overview',
    actions: (id) => [
      A(0, `OV.fadeIn(); OV.chapter('03', 'Control room', 'live · solana devnet')`),
      A(1.6, `OV.ring('[data-tour=stats]', {pad: 6})`),
      A(2.6, `OV.callout({k: 'Measured on devnet', v: '1,520 CU', d: 'per passport check — 0.76% of a transaction budget', anchor: [...document.querySelectorAll('.stat')][4], side: 'above', gap: 30, color: 'var(--v-live)'})`),
      A(cue(id, 1) - 1.4, `OV.unring(); OV.uncall()`),
      ...scroll(cue(id, 1) - 1.3, cue(id, 1) + 0.2, CARD_SEL('hyperlane'), 70),
      A(cue(id, 1) + 0.35, `OV.ring(${CARD('hyperlane')}, {pad: 5, color: '#58a6ff'})`),
    ],
  },

  s06_impact: {
    url: BASE + '#app/impact',
    actions: (id) => [
      A(0, `OV.fadeIn(); OV.chapter('04', 'Impact', 'blast radius of an advisory')`),
      A(0.9, `OV.ring(${CARD('what this is answering')}, {pad: 4, color: '#d29922'})`),
      A(cue(id, 1) - 0.2, `OV.unring(); OV.cursor(1150, 560); OV.move(${DEP('halo2_gadgets', '0.4.0')}, 1.1, -60)`),
      A(cue(id, 2) - 0.1, `OV.click(${DEP('halo2_gadgets', '0.4.0')})`),
      A(cue(id, 2) + 0.7, `OV.ring('[data-tour=radius]', {pad: 4, color: '#f85149'})`),
      A(cue(id, 2) + 1.3, `OV.callout({k: 'GHSA-ww9q-8r59-xv46', v: '1 circuit affected', d: 'halo2_gadgets 0.4.0 — a missing constraint in Orchard\\'s circuit', anchor: '[data-tour=radius] .radius', side: 'below', gap: -150, color: '#f85149'})`),
    ],
  },

  s07_patched: {
    url: BASE + '#app/impact',
    init: `(${DEP('halo2_gadgets', '0.4.0')}).click()`,
    actions: (id) => [
      A(0, `OV.fadeIn(); OV.chapter('04', 'Impact', 'the patched version'); OV.cursor(300, 300); OV.move(${DEP('halo2_gadgets', '0.5.0')}, 0.8, -60)`),
      A(0.95, `OV.click(${DEP('halo2_gadgets', '0.5.0')})`),
      A(2.0, `OV.ring('[data-tour=radius]', {pad: 4, color: '#3fb950'})`),
      A(2.8, `OV.callout({k: 'halo2_gadgets 0.5.0', v: '0 affected', d: '3 circuits reached, none flagged. Amber is evidence aging out — not a finding.', anchor: '[data-tour=radius] .radius', side: 'below', gap: -40, color: '#3fb950'})`),
    ],
  },

  s08_check: {
    url: BASE + '#app/check',
    actions: (id) => [
      A(0, `OV.fadeIn(); OV.chapter('05', 'Check yours', 'live OSV lookup · in your browser')`),
      A(1.2, `OV.ring(document.querySelector('textarea.paste'), {pad: 6})`),
      A(cue(id, 1) - 1.6, `OV.unring(); OV.cursor(700, 500); OV.move(${BTN('Example: pinned')}, 0.9)`),
      A(cue(id, 1) - 0.6, `OV.click(${BTN('Example: pinned')})`),
      A(cue(id, 1) + 0.5, `OV.ring(${CARD('verdict')}, {pad: 5, color: '#f85149', id: 'v'})`),
      A(cue(id, 2) - 1.2, `OV.unring('v'); OV.move(${BTN('Example: patched')}, 0.6)`),
      A(cue(id, 2) - 0.5, `OV.click(${BTN('Example: patched')})`),
      A(cue(id, 2) + 0.3, `OV.ring(${CARD('verdict')}, {pad: 5, color: '#3fb950', id: 'v2'})`),
    ],
  },

  // The Action, on the real landing page's "Try it" section.
  s09_action: {
    url: BASE,
    pre: async (p) => { for (let i = 0; i < 50; i++) { await p.clock.runFor(100); await p.waitForTimeout(10); } await p.evaluate(() => { window.__step(0); window.__step(5000); }); },
    actions: (id) => [
      A(0, `OV.fadeIn()`),
      ...scroll(0.1, 1.6, '#try', 60),
      A(2.4, `OV.ring([...document.querySelectorAll('.lp-card2')].find(c => c.textContent.includes('Add the GitHub Action')), {pad: 6, color: '#3fb950'})`),
    ],
  },

  s10_drill: {
    url: BASE + '#app/drills',
    actions: (id) => [
      A(0, `OV.fadeIn(); OV.chapter('06', 'Fire drill', 'dependency compromise · devnet')`),
      A(2.2, `OV.ring([...document.querySelectorAll('.tl-row .ev')].find(e => e.querySelector('a')), {pad: 6})`),
      A(3.4, `OV.callout({k: 'On-chain evidence', d: 'Signatures and slots link to the live Solana explorer. Local events are labelled local.', anchor: [...document.querySelectorAll('.tl-row .ev')].find(e => e.querySelector('a')), side: 'left', gap: 30})`),
      A(cue(id, 1) - 1.3, `OV.unring(); OV.uncall(); OV.cursor(1000, 520); OV.move('.play', 1.0)`),
      A(cue(id, 1) - 0.15, `OV.click('.play')`),
      A(cue(id, 1) + 4.6, `OV.ring('.gate', {pad: 6, color: '#f85149'})`),
      A(cue(id, 2) - 0.1, `OV.callout({k: 'Consumer check · measured', v: '1,520 CU', d: '0.76% of a default 200k transaction budget', anchor: '.containment', side: 'below', gap: 22, color: '#3fb950'})`),
    ],
  },

  s11_censor: {
    url: BASE + '#app/drills',
    actions: (id) => [
      A(0, `OV.fadeIn(); OV.chapter('07', 'Fire drill', 'relayer censorship · devnet'); OV.cursor(1100, 520); OV.move([...document.querySelectorAll('.seg button')][1], 0.6)`),
      A(0.75, `OV.click([...document.querySelectorAll('.seg button')][1])`),
      A(cue(id, 1) - 0.9, `OV.move('.play', 0.8)`),
      A(cue(id, 1) - 0.05, `OV.click('.play')`),
      A(cue(id, 1) + 2.2, `OV.callout({k: 'After the baseline', v: '0 writes', d: 'No invalidation. No second transaction. Delivery stopped.', anchor: '.containment', side: 'below', gap: 22, color: '#d29922'})`),
      A(cue(id, 2) - 0.9, `OV.uncall(); OV.ring('.gate', {pad: 6, color: '#d29922'})`),
      A(cue(id, 2) + 0.6, `OV.ring('.containment', {pad: 4, color: '#d29922', id: 'c2'})`),
      A(cue(id, 3) - 0.1, `OV.callout({k: 'STALE is never issued', d: 'It is derived by the reader against its own clock. A passport cannot assert its own freshness.', anchor: '.containment', side: 'below', gap: 22, color: '#d29922', id: 'c3'})`),
      A(cue(id, 4) - 0.15, `OV.unring(); OV.unring('c2'); OV.uncall('c3'); OV.banner('The property it rests on', 'Withholding a message produces<br>the safe outcome.', '#d29922')`),
    ],
  },

  s12_live: {
    url: BASE + '#app/fleet',
    actions: (id) => [
      A(0, `OV.fadeIn(); OV.chapter('08', 'Live read', 'your browser → Solana devnet')`),
      ...scroll(0.2, 1.4, '[data-tour=liveread]', 90),
      A(1.6, `OV.cursor(900, 700); OV.move(${BTN('Read from devnet')}, 0.9)`),
      A(cue(id, 0) + 2.7, `OV.click(${BTN('Read from devnet')})`),
      ...scroll(cue(id, 1) - 0.2, cue(id, 1) + 1.2, '[data-tour=liveread]', 20),
      A(cue(id, 1) + 1.5, `OV.ring(document.querySelector('[data-tour=liveread] .gate') || ${CARD('live read')}, {pad: 6, color: '#f85149'})`),
      A(cue(id, 0) + 3.3, `{ const c = document.querySelector('.v-cursor'); c.style.transition = 'opacity .5s ease'; c.style.opacity = '0'; }`),
    ],
  },
};
function CARD_SEL(txt) { return `[data-vcard*="${txt}"]`; }

(async () => {
  const only = process.argv.slice(2);
  for (const [id, sc] of Object.entries(SCENES)) {
    if (only.length && !only.includes(id)) continue;
    const t0 = Date.now();
    const { browser, page } = await open({
      url: sc.url, warm: sc.warm,
      init: async (p) => {
        if (sc.warm !== false) for (let i = 0; i < 6; i++) { await p.clock.runFor(100); await p.waitForTimeout(40); }
        if (sc.pre) await sc.pre(p);
        if (sc.init) { await p.evaluate(sc.init); for (let i = 0; i < 10; i++) { await p.clock.runFor(100); await p.waitForTimeout(30); } }
        await p.addStyleTag({ content: CSS });
        await p.evaluate(OVJS);
        if (sc.vignette === false) await p.evaluate(() => document.querySelector('.v-vig').remove());
        // tag cards by header text so scroll() can target them with a selector
        await p.evaluate(() => document.querySelectorAll('.card').forEach((c) => {
          const h = c.querySelector('header'); if (h) c.setAttribute('data-vcard', h.textContent.toLowerCase());
        }));
        if (sc.url !== BASE) await p.evaluate(() => { window.__step(0); window.__step(2000); });
      },
    });
    const acts = sc.actions(id).map((a) => a.from !== undefined ? null : a).filter(Boolean);
    const ticks = sc.actions(id).filter((a) => a.from !== undefined);
    await record(page, { dur: TL[id].dur, out: `clips/${id}.mp4`, actions: [...acts, ...captions(id)], ticks });
    await browser.close();
    console.log(id, ((Date.now() - t0) / 1000).toFixed(1) + 's');
  }
})();
