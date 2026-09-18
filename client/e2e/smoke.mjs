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

  const ent = s.entrance;
  const before = s.ants.find((a) => a.id === s.playerAnt);
  await page.evaluate(
    (p) => window.__woa.click(p.x, p.y, 2),
    { x: ent[0] + 0.5, y: ent[1] + 0.5 },
  );
  await page.evaluate(() => window.__woa.step(600));
  s = await dump('s03b-entrance');
  const after = s.ants.find((a) => a.id === s.playerAnt);
  console.log(
    'entrance:',
    JSON.stringify({
      before: before ? before.layer : '?',
      after: after ? after.layer : 'gone',
    }),
  );
  if (!after) failures.push('player ant vanished after entrance command');
  else if (before && before.layer === after.layer) {
    failures.push(`entrance right-click did not move ant between layers (${before.layer})`);
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

  const ilog = await page.evaluate(() => window.__woa.log());
  writeFileSync(path.join(OUT, 's06-inputlog.json'), JSON.stringify(ilog, null, 2));
  const types = [...new Set(ilog.events.map((e) => e.type))];
  const acts = [...new Set(ilog.events.filter((e) => e.type === 'cmd').map((e) => e.act))];
  console.log('inputlog:', JSON.stringify({ count: ilog.count, dropped: ilog.dropped, types, acts }));
  if (ilog.count === 0) failures.push('input log recorded no events');
  else {
    if (!types.includes('start')) failures.push('input log missing start event');
    if (!types.includes('view')) failures.push('input log missing view toggle');
    if (!acts.includes('entrance')) failures.push('input log missing entrance command');
    if (spider && !acts.includes('attack')) failures.push('input log missing attack command');
    const ticks = ilog.events.map((e) => e.tick);
    if (ticks.some((tk, i) => i > 0 && tk < ticks[i - 1])) {
      failures.push('input log ticks not monotonic');
    }
    if (Math.max(...ticks) > s.tick) {
      failures.push(`input log tick ahead of sim (max ${Math.max(...ticks)} vs state ${s.tick})`);
    }
  }

  // --- cross-platform determinism: same replay on native and WASM ---
  const replayRel = 'client/public/replays/determinism.json';
  const nativeDump = await new Promise((resolve, reject) => {
    const rustBin = `${process.env.HOME}/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin`;
    const p = spawn(
      'cargo',
      ['run', '--example', 'determinism_dump', '--', path.resolve(ROOT, '..', replayRel)],
      { cwd: path.resolve(ROOT, '..'), env: { ...process.env, PATH: `${rustBin}:${process.env.PATH}` } },
    );
    let out = '';
    p.stdout.on('data', (d) => (out += d));
    p.stderr.on('data', (d) => process.stderr.write(`[cargo] ${d}`));
    p.on('close', (code) => (code === 0 ? resolve(out.trim()) : reject(new Error(`determinism_dump exited ${code}`))));
  });

  await page.goto(`${baseUrl}?replay=determinism`);
  await page.waitForFunction(() => window.__woa !== undefined, null, { timeout: 30000 });

  // replay is watch-only: clicking during replay must not issue a sim command
  await page.evaluate(() => window.__woa.click(48.5, 5.5, 0));
  const replayLog = await page.evaluate(() => window.__woa.log());
  const userCmdsDuringReplay = replayLog.events.filter((e) => e.type === 'cmd' && e.src !== 'replay');
  if (userCmdsDuringReplay.length > 0) {
    failures.push(`sim command issued during replay: ${JSON.stringify(userCmdsDuringReplay[0])}`);
  }

  await page.evaluate(() => window.__woa.stepTo(3000));
  const wasmCanon = await page.evaluate(() => window.__woa.canon());
  writeFileSync(path.join(OUT, 's07-wasm-canon.txt'), wasmCanon);
  writeFileSync(path.join(OUT, 's07-native-canon.txt'), nativeDump);
  const detOk = wasmCanon === nativeDump;
  console.log('determinism:', JSON.stringify({ match: detOk, len: wasmCanon.length, nativeLen: nativeDump.length }));
  if (!detOk) {
    const at = [...wasmCanon].findIndex((c, i) => c !== nativeDump[i]);
    failures.push(`native vs WASM canonical state differ at char ${at} (of ${nativeDump.length})`);
  }
  const badgeHidden = await page.evaluate(() => document.getElementById('replay-badge').classList.contains('hidden'));
  if (!badgeHidden) failures.push('REPLAY badge still visible after replay finished');

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
