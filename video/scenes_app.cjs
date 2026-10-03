// The six control-room scenes: the real app, driven on cue, annotated.
const fs = require('fs');
const { open, record } = require('./rec.cjs');
const TL = Object.fromEntries(JSON.parse(fs.readFileSync('timeline.json')).map((s) => [s.id, s]));
const CSS = fs.readFileSync('common.css', 'utf8');
const OVJS = fs.readFileSync('overlay.js', 'utf8');
const URL = 'http://127.0.0.1:5174/';

const ev = (fn, arg) => (p) => p.evaluate(fn, arg);
const at = (t, fn, arg) => ({ at: t, fn: ev(fn, arg) });

// captions from the narration cues
function captions(id) {
  const cues = TL[id].cues, out = [];
  cues.forEach((c, i) => {
    out.push(at(c.start, (t) => OV.caption(t), c.cap));
    const next = cues[i + 1];
    if (!next || next.start - c.end > 0.7) out.push(at(c.end + 0.35, () => OV.caption(null)));
  });
  return out;
}
const cue = (id, i) => TL[id].cues[i].start;

// in-page helpers (string-free so they serialise)
const DEP = (name, ver) => `[...document.querySelectorAll('.row-dep')].find(r => r.querySelector('.nm').textContent === '${name}' && r.textContent.includes('${ver}'))`;
const NAV = (label) => `[...document.querySelectorAll('.nav button')].find(b => b.textContent === '${label}')`;
const js = (code) => (p) => p.evaluate(code);
const A = (t, code) => ({ at: t, fn: js(code) });

const SCENES = {
  s06_room: {
    view: 'Overview',
    actions: (id) => [
      A(0, `OV.fadeIn(); OV.chapter('03', 'Control room', 'live · solana devnet'); OV.camera('main', 1.0, 0)`),
      A(0.2, `OV.camera('main', 1.04, 6.8)`),
      A(2.4, `OV.ring('[data-tour=stats]', {pad: 6})`),
      A(3.6, `OV.callout({k: 'Measured on devnet', v: '1,520 CU', d: 'per passport check — 0.76% of a default transaction budget', anchor: [...document.querySelectorAll('.stat')][4], side: 'above', gap: 34, color: 'var(--v-live)'})`),
    ],
  },
  s07_impact: {
    view: 'Impact',
    actions: (id) => [
      A(0, `OV.fadeIn(); OV.chapter('04', 'Impact', 'blast radius of an advisory')`),
      A(0.9, `OV.ring([...document.querySelectorAll('.card')].find(c => c.textContent.includes('What this is answering')), {pad: 4, color: '#d29922'})`),
      A(cue(id, 1) - 0.2, `OV.unring(); OV.cursor(1150, 560); OV.move(${DEP('halo2_gadgets', '0.4.0')}, 1.1, -60)`),
      A(cue(id, 2) - 0.1, `OV.click(${DEP('halo2_gadgets', '0.4.0')})`),
      A(cue(id, 2) + 0.7, `OV.ring('[data-tour=radius]', {pad: 4, color: '#f85149'})`),
      A(cue(id, 2) + 1.4, `OV.callout({k: 'GHSA-ww9q-8r59-xv46', v: '1 circuit affected', d: 'halo2_gadgets 0.4.0 — missing copy constraint in variable-base scalar mul', anchor: '[data-tour=radius] .radius', side: 'below', gap: -150, color: '#f85149'})`),
    ],
  },
  s08_patched: {
    view: 'Impact',
    init: `(${DEP('halo2_gadgets', '0.4.0')}).click()`,
    actions: (id) => [
      A(0, `OV.fadeIn(); OV.chapter('04', 'Impact', 'the patched version'); OV.cursor(300, 300); OV.move(${DEP('halo2_gadgets', '0.5.0')}, 0.9, -60)`),
      A(1.15, `OV.click(${DEP('halo2_gadgets', '0.5.0')})`),
      A(2.2, `OV.ring('[data-tour=radius]', {pad: 4, color: '#3fb950'})`),
      A(3.0, `OV.callout({k: 'halo2_gadgets 0.5.0', v: '0 affected', d: '3 circuits reached — every one clean.', anchor: '[data-tour=radius] .radius', side: 'below', gap: -40, color: '#3fb950'})`),
    ],
  },
  s09_drill: {
    view: 'Fire drills',
    actions: (id) => [
      A(0, `OV.fadeIn(); OV.chapter('05', 'Fire drill', 'dependency compromise · devnet')`),
      A(2.4, `OV.ring([...document.querySelectorAll('.tl-row .ev')].find(e => e.querySelector('a')).closest('.tl-row').querySelector('.ev'), {pad: 6})`),
      A(4.0, `OV.callout({k: 'On-chain evidence', d: 'Signatures and slots link to the live Solana explorer. Local events are labelled local.', anchor: [...document.querySelectorAll('.tl-row .ev')].find(e => e.querySelector('a')), side: 'left', gap: 30})`),
      A(cue(id, 1) - 1.3, `OV.unring(); OV.uncall(); OV.cursor(1000, 520); OV.move('.play', 1.0)`),
      A(cue(id, 1) - 0.15, `OV.click('.play')`),
      A(cue(id, 1) + 4.6, `OV.ring('.gate', {pad: 6, color: '#f85149'})`),
      A(cue(id, 2) - 0.1, `OV.callout({k: 'Consumer check · measured', v: '1,520 CU', d: '0.76% of a default 200k transaction budget', anchor: '.containment', side: 'below', gap: 22, color: '#3fb950'})`),
    ],
  },
  s10_censor: {
    view: 'Fire drills',
    actions: (id) => [
      A(0, `OV.fadeIn(); OV.chapter('06', 'Fire drill', 'relayer censorship · devnet'); OV.cursor(1100, 520); OV.move([...document.querySelectorAll('.seg button')][1], 0.6)`),
      A(0.75, `OV.click([...document.querySelectorAll('.seg button')][1])`),
      A(cue(id, 1) - 0.9, `OV.move('.play', 0.8)`),
      A(cue(id, 1) - 0.05, `OV.click('.play')`),
      A(cue(id, 1) + 2.2, `OV.callout({k: 'After the baseline', v: '0 writes', d: 'No invalidation. No second transaction. Delivery stopped.', anchor: '.gate', side: 'below', gap: 360, color: '#d29922'})`),
      A(cue(id, 2) - 0.9, `OV.uncall(); OV.ring('.gate', {pad: 6, color: '#d29922'})`),
      A(cue(id, 2) + 0.6, `OV.ring('.containment', {pad: 4, color: '#d29922', id: 'c2'})`),
      A(cue(id, 3) - 0.1, `OV.callout({k: 'STALE is never issued', d: 'It is derived by the reader against its own clock. A passport cannot assert its own freshness.', anchor: '.containment', side: 'below', gap: 22, color: '#d29922', id: 'c3'})`),
      A(cue(id, 4) - 0.15, `OV.unring(); OV.unring('c2'); OV.uncall('c3'); OV.banner('The property it rests on', 'Withholding a message produces<br>the safe outcome.', '#d29922')`),
    ],
  },
  s11_embargo: {
    view: 'Impact',
    init: `(${DEP('halo2_gadgets', '0.4.0')}).click()`,
    actions: (id) => [
      A(0, `OV.fadeIn(); OV.chapter('07', 'Disclosure', 'public vs operator'); OV.ring(${DEP('halo2_gadgets', '0.4.0')}, {pad: 2, color: '#f85149', id: 'd'}); OV.cursor(1180, 300); OV.move('.aud .op', 0.9)`),
      A(1.05, `OV.unring('d'); OV.click('.aud .op')`),
      A(cue(id, 1) - 0.1, `OV.ring(${DEP('halo2_gadgets', '0.4.0')}, {pad: 2, color: '#f85149', id: 'a'}); OV.ring('[data-tour=radius]', {pad: 4, color: '#f85149', id: 'b'})`),
      A(cue(id, 1) + 0.5, `OV.callout({k: 'halo2_gadgets 0.4.0 · public → operator', v: '1 → 3 affected', d: 'Two more circuits appear. Nothing in the public view hinted they existed.', anchor: '[data-tour=radius] .radius', side: 'below', gap: -150, color: '#f85149'})`),
      A(cue(id, 2) + 0.2, `OV.unring('a'); OV.unring('b'); OV.uncall(); OV.ring(document.querySelector('.main > .card'), {pad: 4, color: '#58a6ff', id: 'n'})`),
    ],
  },
};

(async () => {
  const only = process.argv.slice(2);
  for (const [id, sc] of Object.entries(SCENES)) {
    if (only.length && !only.includes(id)) continue;
    const t0 = Date.now();
    const { browser, page } = await open({
      url: URL,
      init: async (p) => {
        await p.evaluate(`(${NAV(sc.view)}).click()`);
        for (let i = 0; i < 6; i++) { await p.clock.runFor(100); await p.waitForTimeout(40); }
        if (sc.init) { await p.evaluate(sc.init); for (let i = 0; i < 10; i++) { await p.clock.runFor(100); await p.waitForTimeout(30); } }
        await p.addStyleTag({ content: CSS });
        await p.evaluate(OVJS);
        // settle the initial view animations before frame 0
        await p.evaluate(() => { window.__step(0); window.__step(2000); });
      },
    });
    await record(page, { dur: TL[id].dur, out: `clips/${id}.mp4`, actions: [...sc.actions(id), ...captions(id)] });
    await browser.close();
    console.log(id, ((Date.now() - t0) / 1000).toFixed(1) + 's');
  }
})();
