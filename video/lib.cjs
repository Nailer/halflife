const fs = require('fs'), path = require('path');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const FONTS = path.join(__dirname, 'fonts');
// Live external calls (OSV, Solana devnet) are relayed through curl, which
// trusts the sandbox proxy. The responses are real and fetched at capture time.
const { execFileSync } = require('child_process');
const LIVE = /^https:\/\/(api\.osv\.dev|api\.devnet\.solana\.com)\//;
function relay(route) {
  const req = route.request();
  const args = ['-sS', '-m', '30', '-X', req.method(), '-H', 'Content-Type: application/json', req.url()];
  if (req.postData()) args.push('--data-binary', '@-');
  const out = execFileSync('curl', args, { input: req.postData() || '' });
  if (process.env.LIVE_LOG) console.log('live', req.url(), out.length, 'bytes');
  return route.fulfill({ status: 200, contentType: 'application/json', headers: { 'access-control-allow-origin': '*' }, body: out });
}
async function setup(page) {
  await page.route(LIVE, relay);
  await page.route(/fonts\.googleapis\.com\/css2/, r => r.fulfill({ contentType: 'text/css', body: fs.readFileSync(FONTS + '/fonts.css', 'utf8') }));
  await page.route(/\/__fonts\//, r => r.fulfill({ contentType: 'font/woff2', body: fs.readFileSync(FONTS + '/' + path.basename(new URL(r.request().url()).pathname)) }));
  await page.addInitScript(() => { try { localStorage.setItem('halflife.tour.v1', '1'); localStorage.setItem('halflife.theme', 'dark'); } catch (e) {} });
}
async function launch() {
  const b = await chromium.launch({ args: ['--font-render-hinting=none', '--disable-lcd-text'] });
  const ctx = await b.newContext({ viewport: { width: 1440, height: 810 }, deviceScaleFactor: 4 / 3, colorScheme: 'dark' });
  return { b, ctx };
}
module.exports = { setup, launch };
