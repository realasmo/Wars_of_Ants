// Gait e2e: planted-feet (no sliding), tripod alternation, in-group ripple
// stagger, and idle micro-motion — the acceptance core of the
// procedural-ants ticket. Reuses the live dev server on :5173.
import { chromium } from 'playwright';

const BASE = process.env.WOA_BASE || 'http://localhost:5173';
const failures = [];
const note = (ok, label, detail = '') => {
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${label}${detail ? ' — ' + detail : ''}`);
  if (!ok) failures.push(label + (detail ? ` (${detail})` : ''));
};

const browser = await chromium.launch({
  executablePath: '/usr/bin/chromium',
  headless: true,
  args: [
    '--no-sandbox',
    '--use-gl=angle',
    '--use-angle=swiftshader',
    '--enable-unsafe-swiftshader',
    '--disable-background-timer-throttling',
    '--disable-renderer-backgrounding',
    '--disable-backgrounding-occluded-windows',
  ],
});
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const pageErrors = [];
  page.on('pageerror', (e) => pageErrors.push(String(e)));
  await page.goto(`${BASE}/?seed=11&e2e=1`, { waitUntil: 'load' });
  await page.waitForSelector('#menu:not(.hidden)', { timeout: 30000 });
  await page.click('#btn-play');
  await page.click('#btn-team-red');
  await page.waitForFunction(() => window.__woa !== undefined, null, { timeout: 30000 });
  await page.waitForTimeout(500);

  // land → found: the queen walks underground; she is our test walker
  const q = await page.evaluate(() => { const s = window.__woa.state(); return { id: s.queen.id, x: s.queen.x, y: s.queen.y }; });
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

  // poll gait state from outside, paced at ~60 Hz with real waits between
  // polls — back-to-back evaluates starve the game's rAF loop in headless
  const sample = async (ms, cmds) => {
    const out = [];
    const t0 = Date.now();
    while (Date.now() - t0 < ms) {
      for (const c of cmds) {
        if (!c.done && Date.now() - t0 >= c.at) {
          c.done = true;
          await page.evaluate((cc) => window.__woa.click(cc.x, cc.y, 0), c);
        }
      }
      const g = await page.evaluate((id) => window.__woa.gait(id), q.id);
      out.push({ t: Date.now(), g });
      await page.waitForTimeout(16);
    }
    return out;
  };

  const q3 = await page.evaluate(() => { const s = window.__woa.state(); return { x: s.queen.x, y: s.queen.y }; });
  // walk targets must be dug-out floor — clicking solid rock makes move a no-op
  const floorTiles = await page.evaluate((q) => {
    const out = [];
    for (let dy = -5; dy <= 5; dy++) {
      for (let dx = -5; dx <= 5; dx++) {
        const tx = Math.floor(q.x) + dx;
        const ty = Math.floor(q.y) + dy;
        if (window.__woa.tile(1, tx, ty) === 0) out.push({ x: tx + 0.5, y: ty + 0.5, d: Math.hypot(dx, dy) });
      }
    }
    return out.sort((a, b) => a.d - b.d);
  }, q3);
  const far = floorTiles[floorTiles.length - 1];
  if (!far || floorTiles.length < 4) {
    failures.push(`not enough floor around the queen (${floorTiles.length} tiles)`);
  }
  const other = floorTiles[Math.floor(floorTiles.length / 2)];
  const walk = await sample(3400, [
    { at: 100, x: far.x, y: far.y, done: false },
    { at: 1600, x: other.x, y: other.y, done: false },
    { at: 3100, x: far.x, y: far.y, done: false },
  ]);
  const activity = await page.evaluate((id) => window.__woa.state().queen.activity, q.id);

  const frames = walk.filter((f) => f.g && f.g.legs && f.g.legs.length === 6);
  note(frames.length > 60, `sampled ${frames.length} gait frames`);
  const movingFrames = frames.filter((f) => f.g.speed > 0.05);
  note(movingFrames.length > 20, 'queen actually walks in the window', `${movingFrames.length}/${frames.length} moving (activity now: ${activity})`);

  // --- 1) planted feet never slide (between consecutive samples) ---
  let slides = 0;
  let checked = 0;
  let walkingPairs = 0;
  for (let k = 1; k < frames.length; k++) {
    const a = frames[k - 1].g.legs;
    const b = frames[k].g.legs;
    const moving = frames[k].g.speed > 0.05 || frames[k - 1].g.speed > 0.05;
    if (moving) walkingPairs++;
    for (let i = 0; i < 6; i++) {
      if (!a[i].planted || !b[i].planted) continue;
      checked++;
      if (moving && (a[i].x !== b[i].x || a[i].y !== b[i].y)) {
        slides++;
        if (slides <= 3) console.log(`  slide: leg ${i} ${a[i].x.toFixed(3)},${a[i].y.toFixed(3)} -> ${b[i].x.toFixed(3)},${b[i].y.toFixed(3)}`);
      }
    }
  }
  note(slides === 0, 'planted feet never slide while walking', `${slides} slides / ${checked} planted pairs (${walkingPairs} moving pairs)`);

  // --- 2) tripod alternation: never both groups swinging; both do swing ---
  let both = 0;
  let sawA = false;
  let sawB = false;
  let anySwing = 0;
  for (const f of frames) {
    const swingA = f.g.legs.some((l) => l.group === 'A' && !l.planted);
    const swingB = f.g.legs.some((l) => l.group === 'B' && !l.planted);
    if (swingA) sawA = true;
    if (swingB) sawB = true;
    if (swingA && swingB) both++;
    if (swingA || swingB) anySwing++;
  }
  note(both === 0, 'tripod groups never swing simultaneously', `${both} overlap frames`);
  note(sawA && sawB, 'both tripod groups step', `A=${sawA} B=${sawB}, swinging frames ${anySwing}/${frames.length}`);
  note(anySwing > frames.length * 0.25, 'walking dominates the window', `${anySwing}/${frames.length}`);

  // --- 3) in-group ripple stagger (addendum): when a leg starts, no earlier
  //        leg of its group may be in its early swing ---
  const RIPPLE = { A: [0, 4, 2], B: [3, 1, 5] }; // front→mid→rear per group
  const STAGGER = 0.13;
  let badStarts = 0;
  let starts = 0;
  for (let k = 1; k < frames.length; k++) {
    const prev = frames[k - 1].g.legs;
    const now = frames[k].g.legs;
    for (const grp of ['A', 'B']) {
      const order = RIPPLE[grp];
      for (let j = 0; j < order.length; j++) {
        const i = order[j];
        if (prev[i].planted && !now[i].planted) {
          starts++;
          for (let e = 0; e < j; e++) {
            const p = now[order[e]];
            // t≈0 means it started in the same poll interval (after this
            // leg, in the same ripple) — not a violation; only flag an
            // earlier leg demonstrably still in its early swing
            if (!p.planted && p.t > 0.02 && p.t < STAGGER * 0.9) {
              badStarts++;
              console.log(`  ripple violation: leg ${i} started while ${order[e]} at t=${p.t.toFixed(2)}`);
            }
          }
        }
      }
    }
  }
  note(starts > 8 && badStarts === 0, 'in-group ripple stagger respected', `${badStarts} violations over ${starts} step starts`);

  // --- 4) idle micro-motion: antennae move within 5 s of standing ---
  for (let i = 0; i < 40; i++) {
    const sp = await page.evaluate((id) => window.__woa.gait(id).speed, q.id);
    if (sp < 0.03) break;
    await page.waitForTimeout(150);
  }
  const idle = await sample(5000, []);
  const idf = idle.filter((f) => f.g);
  const antL = idf.map((f) => f.g.antennae[0]);
  const antR = idf.map((f) => f.g.antennae[1]);
  const spread = (a) => Math.max(...a) - Math.min(...a);
  const calmSpeed = idf.every((f) => f.g.speed < 0.05);
  note(spread(antL) > 0.08 || spread(antR) > 0.08, 'idle antennae move within 5 s', `LΔ=${spread(antL).toFixed(3)} RΔ=${spread(antR).toFixed(3)}`);
  note(calmSpeed, 'idle queen stands still', '');

  note(pageErrors.length === 0, 'no page errors', pageErrors.slice(0, 3).join(' | '));
  await browser.close();
} finally {
  await browser.close();
}

if (failures.length > 0) {
  console.error('GAIT FAILURES:', failures);
  process.exit(1);
}
console.log('GAIT OK');
process.exit(0);
