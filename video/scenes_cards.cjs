const fs = require('fs');
const { open, record } = require('./rec.cjs');
const TL = JSON.parse(fs.readFileSync('timeline.json'));
const CSS = fs.readFileSync('common.css', 'utf8');
const OVJS = fs.readFileSync('overlay.js', 'utf8');
const CARDS = ['s01_title', 's02_problem', 's03_insight', 's04_solution', 's05_cli', 's12_why', 's13_self', 's14_end'];
const at = (t, code) => ({ at: t, fn: (p) => p.evaluate(code) });
(async () => {
  const only = process.argv.slice(2);
  for (const sc of TL) {
    if (!CARDS.includes(sc.id) || (only.length && !only.includes(sc.id))) continue;
    const t0 = Date.now();
    const url = 'file://' + require('path').join(__dirname, 'cards.html') + '?s=' + sc.id + '&c=' + encodeURIComponent(JSON.stringify(sc.cues));
    const { browser, page } = await open({ url, init: async (p) => {
      await p.addStyleTag({ content: CSS }); await p.evaluate(OVJS);
      await p.evaluate(() => document.querySelector('.v-vig').remove());
    }});
    const acts = [];
    sc.cues.forEach((c, i) => {
      acts.push(at(c.start, `OV.caption(${JSON.stringify(c.cap)})`));
      const nx = sc.cues[i + 1];
      if (!nx || nx.start - c.end > 0.7) acts.push(at(c.end + 0.35, 'OV.caption(null)'));
    });
    await record(page, { dur: sc.dur, out: `clips/${sc.id}.mp4`, actions: acts });
    await browser.close();
    console.log(sc.id, ((Date.now() - t0) / 1000).toFixed(1) + 's');
  }
})();
