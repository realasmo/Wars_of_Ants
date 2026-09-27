// Perf snapshot: reuse the live dev server on :5173 (never spawn a second
// vite). Loads the game with ?perf=1, spawns a 100-ant crowd near the
// queen, and reports the perf overlay's moving + paused numbers.
// Usage: node e2e/perf.mjs [label]
import { chromium } from 'playwright';

const LABEL = process.argv[2] || 'run';
const BASE = process.env.WOA_BASE || 'http://127.0.0.1:5173';

const browser = await chromium.launch({
  executablePath: '/usr/bin/chromium',
  headless: true,
  args: ['--no-sandbox', '--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  await page.goto(`${BASE}/?seed=11&e2e=1&perf=1`, { waitUntil: 'load' });
  await page.waitForSelector('#menu:not(.hidden)', { timeout: 30000 });
  await page.click('#btn-play');
  await page.click('#btn-team-red');
  await page.waitForFunction(() => window.__woa !== undefined, null, { timeout: 30000 });
  await page.waitForTimeout(600);

  // crowd: 95 workers + 5 soldiers on the queen's layer, around her
  const q = await page.evaluate(() => {
    const s = window.__woa.state();
    return { x: s.queen.x, y: s.queen.y, layer: s.queen.layer };
  });
  const spawned = await page.evaluate((base) => {
    let n = 0;
    for (let i = 0; i < 95; i++) {
      const a = (i * 2.399963) % (Math.PI * 2); // golden-angle ring
      const r = 1 + (i % 7) * 0.9;
      if (window.__woa.spawn('worker', base.x + Math.cos(a) * r, base.y + Math.sin(a) * r)) n++;
    }
    for (let i = 0; i < 5; i++)
      if (window.__woa.spawn('soldier', base.x + Math.cos(i * 1.3) * 4, base.y + Math.sin(i * 1.3) * 4)) n++;
    return n;
  }, q);
  // park the camera inside the crowd: land the flying queen (wait grounded),
  // found the nest where she stands (wait founding) — the camera follows her
  // underground, where all dev-spawned ants live, so the crowd is on-screen
  await page.evaluate((p) => window.__woa.click(p.x, p.y, 2), q);
  for (let i = 0; i < 60; i++) {
    await page.evaluate(() => window.__woa.step(5));
    if ((await page.evaluate(() => window.__woa.state().phase)) === 1) break;
  }
  const q2 = await page.evaluate(() => {
    const s = window.__woa.state();
    return { x: s.queen.x, y: s.queen.y };
  });
  await page.evaluate((p) => window.__woa.click(p.x, p.y, 2), q2);
  for (let i = 0; i < 60; i++) {
    await page.evaluate(() => window.__woa.step(5));
    if ((await page.evaluate(() => window.__woa.state().phase)) === 2) break;
  }
  await page.evaluate(() => window.__woa.step(30));
  await page.evaluate(() => window.__woa.setfood(500));
  await page.waitForTimeout(8000); // the crowd migrates out to forage
  // follow a surface worker: the camera sits inside the walking crowd —
  // that is the real rig stress, not an empty nest corridor
  await page.evaluate(() => window.__woa.pause());
  await page.evaluate(() => window.__woa.key('KeyC'));
  await page.evaluate(() => window.__woa.pause());
  await page.waitForTimeout(4000); // moving crowd: sim + render
  const moving = await page.evaluate(() => document.getElementById('perf').textContent);

  await page.evaluate(() => window.__woa.pause());
  await page.waitForTimeout(5000); // paused: render-only
  const paused = await page.evaluate(() => document.getElementById('perf').textContent);

  console.log(`[perf:${LABEL}] spawned=${spawned}`);
  console.log(`[perf:${LABEL}:moving]\n${moving}`);
  console.log(`[perf:${LABEL}:paused]\n${paused}`);
} finally {
  await browser.close();
}
