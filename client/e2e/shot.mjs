// Close-up captures for the QC loop: idle + walking + carrying per caste.
// Reuses the live dev server on :5173. Usage: node e2e/shot.mjs [outdir]
import { chromium } from 'playwright';
import { mkdirSync } from 'node:fs';

const OUT = process.argv[2] || '/tmp/woa-ants';
mkdirSync(OUT, { recursive: true });
const browser = await chromium.launch({
  executablePath: '/usr/bin/chromium', headless: true,
  args: ['--no-sandbox', '--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader',
    '--disable-background-timer-throttling', '--disable-renderer-backgrounding', '--disable-backgrounding-occluded-windows'],
});
const zoomIn = async (page) => {
  await page.mouse.move(640, 400);
  for (let i = 0; i < 9; i++) await page.mouse.wheel(0, -400);
  await page.waitForTimeout(250);
};
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => { if (m.type() === 'error') errors.push('console: ' + m.text()); });
  await page.goto('http://localhost:5173/?seed=11&e2e=1', { waitUntil: 'load' });
  await page.waitForSelector('#menu:not(.hidden)', { timeout: 30000 });
  await page.click('#btn-play');
  await page.click('#btn-team-red');
  await page.waitForFunction(() => window.__woa !== undefined, null, { timeout: 30000 });
  await page.waitForTimeout(700);
  await zoomIn(page);
  await page.screenshot({ path: `${OUT}/queen-flying.png` });

  // land → found → dig a wall so the queen hauls dirt (carrying shot)
  const q = await page.evaluate(() => { const s = window.__woa.state(); return { x: s.queen.x, y: s.queen.y }; });
  await page.evaluate((p) => window.__woa.click(p.x, p.y, 2), q);
  for (let i = 0; i < 60; i++) {
    await page.evaluate(() => window.__woa.step(5));
    if ((await page.evaluate(() => window.__woa.state().phase)) === 1) break;
  }
  const q2 = await page.evaluate(() => { const s = window.__woa.state(); return { x: s.queen.x, y: s.queen.y }; });
  await page.evaluate((p) => window.__woa.click(p.x, p.y, 2), q2);
  for (let i = 0; i < 60; i++) {
    await page.evaluate(() => window.__woa.step(5));
    if ((await page.evaluate(() => window.__woa.state().phase)) === 2) break;
  }
  await page.evaluate(() => window.__woa.step(30));
  const s2 = await page.evaluate(() => window.__woa.state());
  const [ex, ey] = s2.entrance;
  // a fully-soft 2×2 block NOT adjacent to the entrance hole (clicking the
  // hole area while hauling means "carry it out" — she'd surface)
  const wall = await page.evaluate((e) => {
    const cands = [[e[0] + 2, e[1] + 6], [e[0] - 2, e[1] + 4], [e[0] + 2, e[1] + 4], [e[0], e[1] + 8]];
    for (const [bx, by] of cands) {
      if (Math.max(Math.abs(bx - e[0]), Math.abs(by - e[1])) <= 2) continue;
      const soft = [[0, 0], [1, 0], [0, 1], [1, 1]].every(([dx, dy]) => {
        const k = window.__woa.tile(1, bx + dx, by + dy);
        return k >= 1 && k <= 3;
      });
      if (soft) return { x: bx + 0.5, y: by + 0.5 };
    }
    return null;
  }, [ex, ey]);
  if (wall) {
    await page.evaluate((w) => window.__woa.click(w.x, w.y, 2), wall);
    for (let i = 0; i < 60; i++) {
      await page.evaluate(() => window.__woa.step(5));
      const st = await page.evaluate(() => window.__woa.state());
      if (st.queen.carry.startsWith('dirt') && st.queen.layer === 'U') break;
    }
  }
  await page.waitForTimeout(600);
  await zoomIn(page);
  await page.screenshot({ path: `${OUT}/queen-carrying.png` });

  // keep the queen home: if she surfaced while hauling, send her back in
  for (let i = 0; i < 3; i++) {
    const st = await page.evaluate(() => window.__woa.state());
    if (st.queen.layer !== 'S') break;
    await page.evaluate((e) => window.__woa.click(e[0] + 0.5, e[1] + 0.5, 2), [ex, ey]);
    await page.evaluate(() => window.__woa.step(120));
  }

  // caste lineup: spawned worker + soldier clear of the bigger queen; PAUSE
  // before the shot so the free-roaming castes can't overlap into a chimera
  const q3 = await page.evaluate(() => { const s = window.__woa.state(); return { x: s.queen.x, y: s.queen.y }; });
  await page.evaluate((p) => {
    window.__woa.spawn('worker', p.x + 4.2, p.y);
    window.__woa.spawn('soldier', p.x - 4.4, p.y);
  }, q3);
  await page.evaluate(() => window.__woa.step(8));
  await page.evaluate(() => window.__woa.pause());
  await page.waitForTimeout(1100);
  await page.screenshot({ path: `${OUT}/castes.png` });
  await page.evaluate(() => window.__woa.pause());

  // walking worker: select it (click+verify in ONE evaluate — split calls
  // race the live rAF loop), then send it across the chamber
  const st3 = await page.evaluate(() => window.__woa.state());
  const worker = st3.ants.find((a) => a.layer === 'U');
  let selected = false;
  if (worker) {
    await page.evaluate(() => window.__woa.pause());
    selected = await page.evaluate((id) => {
      const w = window.__woa.state().ants.find((a) => a.id === id);
      if (!w) return { ok: false, why: 'worker missing' };
      window.__woa.click(w.x, w.y, 0);
      const s = window.__woa.state();
      return {
        ok: s.playerAnt === id,
        why: `playerAnt=${s.playerAnt} worker=${w.id}@${w.x.toFixed(1)},${w.y.toFixed(1)} queen=${s.queen ? `${s.queen.x.toFixed(1)},${s.queen.y.toFixed(1)} L${s.queen.layer}` : 'gone'} view=${s.layer} paused=${s.paused}`,
      };
    }, worker.id);
    console.log('select:', JSON.stringify(selected));
    selected = selected.ok;
    if (selected) {
      await page.evaluate(() => window.__woa.pause());
      const me = await page.evaluate((id) => window.__woa.state().ants.find((a) => a.id === id), worker.id);
      const tgt = await page.evaluate((m) => {
        let best = null;
        let bd = -1;
        for (let dy = -7; dy <= 7; dy++) for (let dx = -7; dx <= 7; dx++) {
          const tx = Math.floor(m.x) + dx, ty = Math.floor(m.y) + dy;
          if (window.__woa.tile(1, tx, ty) === 0) {
            const d = Math.hypot(dx, dy);
            if (d > bd) { bd = d; best = { x: tx + 0.5, y: ty + 0.5 }; }
          }
        }
        return best;
      }, me);
      if (tgt) {
        await page.evaluate((t) => window.__woa.click(t.x, t.y, 0), tgt);
        await page.waitForTimeout(800);
        await page.screenshot({ path: `${OUT}/worker-walking.png` });
        await page.waitForTimeout(600);
        await page.screenshot({ path: `${OUT}/worker-walking2.png` });
      }
    } else {
      console.log('worker selection failed — skipping walking shots');
    }
  }
  console.log('errors:', errors.length ? errors.slice(0, 6) : 'none');
  console.log('shots in', OUT);
} finally { await browser.close(); }
