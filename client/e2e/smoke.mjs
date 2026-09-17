import { chromium } from 'playwright';
import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const PORT = Number(process.env.WOA_PORT || 5199);
const SEED = process.env.WOA_SEED || '42';
const OUT = process.env.WOA_OUT || '/tmp/opencode/woa-shots';
mkdirSync(OUT, { recursive: true });

async function waitServer(port, timeoutMs) {
  const urls = [`http://127.0.0.1:${port}/`, `http://[::1]:${port}/`];
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    for (const url of urls) {
      try {
        const r = await fetch(url, { signal: AbortSignal.timeout(2000) });
        if (r.ok) return url;
      } catch {}
    }
    await new Promise((r) => setTimeout(r, 400));
  }
  throw new Error(`dev server not ready on port ${port}`);
}

const server = spawn('npm', ['run', 'dev', '--', '--port', String(PORT), '--strictPort'], {
  cwd: ROOT,
  stdio: ['ignore', 'pipe', 'pipe'],
  detached: true,
});
server.stderr.on('data', (d) => process.stderr.write(`[vite] ${d}`));

let failures = [];
let baseUrl;
try {
  baseUrl = await waitServer(PORT, 30000);
  const browser = await chromium.launch({
    executablePath: '/usr/bin/chromium',
    headless: true,
    args: [
      '--no-sandbox',
      '--use-gl=angle',
      '--use-angle=swiftshader',
      '--enable-unsafe-swiftshader',
    ],
  });
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const pageErrors = [];
  page.on('pageerror', (e) => pageErrors.push(String(e)));
  page.on('console', (m) => {
    if (m.type() === 'error') pageErrors.push(`console: ${m.text()}`);
  });

  await page.goto(`${baseUrl}?seed=${SEED}&e2e=1`);
  await page.waitForFunction(() => window.__woa !== undefined, null, { timeout: 30000 });
  await page.waitForTimeout(1200);

  const state = () => page.evaluate(() => window.__woa.state());
  const dump = async (name) => {
    const s = await state();
    writeFileSync(path.join(OUT, `${name}.json`), JSON.stringify(s, null, 2));
    return s;
  };
  const shot = (name) => page.screenshot({ path: path.join(OUT, `${name}.png`) });
  const px = (x, y) => page.evaluate((p) => window.__woa.px(p.x, p.y), { x, y });
  const near = (got, want, tol = 40) => {
    const d = Math.max(Math.abs(got[0] - want[0]), Math.abs(got[1] - want[1]), Math.abs(got[2] - want[2]));
    return d <= tol;
  };
  const DIRT = [0x5c, 0x40, 0x33];

  const hudText = async () =>
    Object.fromEntries(
      await page.evaluate(() =>
        ['stat-food', 'stat-super', 'stat-ants', 'stat-eggs', 'stat-dug', 'stat-time', 'ctrl-ant', 'ctrl-layer'].map(
          (id) => [id, document.getElementById(id)?.textContent ?? '?'],
        ),
      ),
    );

  let s = await dump('s01-start');
  await shot('s01-underground-start');
  console.log('start:', JSON.stringify({ tick: s.tick, ants: s.ants.length, food: s.food }));

  const pxDirt = await px(50.5, 2.5);
  if (!near(pxDirt, DIRT)) failures.push(`dirt tile color wrong: ${pxDirt} vs ${DIRT}`);

  const hud1 = await hudText();
  console.log('hud:', JSON.stringify(hud1));

  await page.evaluate(() => window.__woa.step(600));
  await page.waitForTimeout(400);
  s = await dump('s02-30s');
  await shot('s02-underground-30s');
  console.log('after 30s:', JSON.stringify({ tick: s.tick, dug: s.dug, food: s.food, eggs: s.eggs }));

  await page.evaluate(() => window.__woa.key('Tab'));
  await page.waitForTimeout(400);
  await shot('s03-surface');
  s = await dump('s03-surface');
  console.log('surface:', JSON.stringify({ layer: s.layer, spiders: s.spiders.length }));

  const surfaceAnts = s.ants.filter((a) => a.layer === 'S');
  const anchor = surfaceAnts[0] ?? s.ants[0];
  if (anchor) {
    const pxAnt = await px(anchor.x, anchor.y);
    if (pxAnt[3] === 0) failures.push('ant pixel fully transparent (render broken?)');
    let fogSample = null;
    for (let k = 1; k <= 6 && !fogSample; k++) {
      const fx = anchor.x + 12 * k;
      const fy = anchor.y + 8 * k;
      if (fx > 92 || fy > 92) break;
      const minDist = Math.min(
        ...s.ants.map((a) => Math.hypot(a.x - fx, a.y - fy)),
      );
      const visibleHere = s.ants.some((a) => Math.hypot(a.x - fx, a.y - fy) <= 10.5);
      if (minDist > 11.5 && !visibleHere) {
        const got = await px(fx, fy);
        if (!(got[3] === 0 && got[0] === 0 && got[1] === 0)) fogSample = { fx, fy, got, minDist: +minDist.toFixed(1) };
      }
    }
    if (!fogSample) {
      console.log('fog: no unlit on-screen tile found to sample (ants cover view) — skipped');
    } else if (!(fogSample.got[0] < 60 && fogSample.got[1] < 60 && fogSample.got[2] < 60)) {
      failures.push(`fog not darkening distant tiles: ${JSON.stringify(fogSample)}`);
    }
    console.log('pixels:', JSON.stringify({ dirt: pxDirt, ant: pxAnt, fog: fogSample?.got ?? null }));
  }

  const spider = s.spiders[0];
  if (spider) {
    await page.evaluate((p) => window.__woa.click(p.x, p.y, 0), { x: spider.x, y: spider.y });
    await page.evaluate(() => window.__woa.step(1500));
    await page.waitForTimeout(400);
    s = await dump('s04-fight');
    await shot('s04-fight');
    const after = s.spiders.find((sp) => sp.id === spider.id);
    console.log('fight:', JSON.stringify({ spiderBefore: spider.hp, spiderAfter: after ? after.hp : 'dead', antsLeft: s.ants.length }));
    if (after && after.hp >= 1.0) failures.push('attack did not damage the spider');
    if (s.ants.length === 0) failures.push('all ants died');
  } else {
    console.log('no spider visible on surface state — skipping fight');
  }

  await page.evaluate(() => window.__woa.key('Tab'));
  await page.evaluate(() => window.__woa.step(3000));
  await page.waitForTimeout(400);
  s = await dump('s05-late');
  await shot('s05-underground-late');
  console.log('late:', JSON.stringify({ tick: s.tick, workers: s.workers, soldiers: s.soldiers, dug: s.dug, dead: s.dead }));

  if (pageErrors.length > 0) failures.push(`page errors: ${pageErrors.slice(0, 5).join(' | ')}`);
  await browser.close();
} finally {
  try {
    process.kill(-server.pid, 'SIGKILL');
  } catch {}
}

if (failures.length > 0) {
  console.error('FAILURES:', failures);
  process.exit(1);
}
console.log('SMOKE OK — shots and states in', OUT);
process.exit(0);
