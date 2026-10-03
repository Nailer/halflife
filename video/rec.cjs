// Deterministic frame-stepped recorder.
// Time is virtual: Playwright's fake clock drives timers, and every Web
// Animation (CSS animations + transitions) is paused and seeked by hand, so a
// 30fps capture is perfectly smooth regardless of how long a screenshot takes.
const { spawn } = require('child_process');
const { setup } = require('./lib.cjs');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');

const FPS = 30, DT = 1000 / FPS;

const STEPPER = () => {
  window.__step = (dt) => {
    // pause() detaches an animation from CSS, so retire the ones CSS has
    // replaced: animations whose name left the element's animation-name, and
    // transitions superseded by a newer one on the same property.
    const latest = new Map();
    for (const a of document.getAnimations()) {
      const tg = a.effect && a.effect.target;
      if (!tg) continue;
      if (a instanceof CSSAnimation) {
        const names = getComputedStyle(tg, a.effect.pseudoElement).animationName.split(',').map((s) => s.trim());
        if (!names.includes(a.animationName)) { a.cancel(); continue; }
      } else if (a instanceof CSSTransition) {
        if (!tg.__tid) tg.__tid = Math.random();
        const k = tg.__tid + '|' + (a.effect.pseudoElement || '') + '|' + a.transitionProperty;
        const prev = latest.get(k); if (prev) prev.cancel();
        latest.set(k, a);
      }
    }
    for (const a of document.getAnimations()) {
      if (a.__t === undefined) { a.__t = 0; } else { a.__t += dt; }
      a.pause();
      try { a.currentTime = a.__t; } catch (e) {}
    }
  };
  window.__settle = () => new Promise((r) => {
    const ch = new MessageChannel(); let n = 0;
    ch.port1.onmessage = () => (++n < 3 ? ch.port2.postMessage(0) : r());
    ch.port2.postMessage(0);
  });
};

async function open({ url, viewport = { width: 1440, height: 810 }, scale = 4 / 3, init }) {
  const browser = await chromium.launch({ args: ['--font-render-hinting=none', '--disable-lcd-text', '--hide-scrollbars'] });
  const ctx = await browser.newContext({ viewport, deviceScaleFactor: scale, colorScheme: 'dark' });
  const page = await ctx.newPage();
  await setup(page);
  await page.addInitScript(STEPPER);
  await page.clock.install({ time: new Date('2026-10-03T12:00:00Z') });
  await page.clock.pauseAt(new Date('2026-10-03T12:00:01Z'));
  await page.goto(url);
  await page.evaluate(() => document.fonts.ready);
  // let data load: timers are frozen, but fetch is real
  for (let i = 0; i < 10; i++) { await page.clock.runFor(50); await page.waitForTimeout(60); }
  if (init) await init(page);
  return { browser, page };
}

// actions: [{at: seconds, fn: async (page) => {}}]
async function record(page, { dur, out, actions = [] }) {
  const ff = spawn('ffmpeg', ['-y', '-loglevel', 'error', '-f', 'image2pipe', '-framerate', String(FPS), '-i', '-',
    '-c:v', 'libx264', '-preset', 'medium', '-crf', '12', '-pix_fmt', 'yuv420p', '-r', String(FPS), out], { stdio: ['pipe', 'inherit', 'inherit'] });
  const queue = actions.slice().sort((a, b) => a.at - b.at);
  const n = Math.round(dur * FPS);
  await page.evaluate(() => window.__step(0));
  for (let f = 0; f < n; f++) {
    const t = f / FPS;
    while (queue.length && queue[0].at <= t + 1e-6) await queue.shift().fn(page);
    await page.clock.runFor(DT);
    await page.evaluate(async (dt) => { await window.__settle(); window.__step(dt); }, DT);
    const buf = await page.screenshot({ type: 'png' });
    if (!ff.stdin.write(buf)) await new Promise((r) => ff.stdin.once('drain', r));
  }
  ff.stdin.end();
  await new Promise((r) => ff.on('close', r));
}

module.exports = { open, record, FPS };
