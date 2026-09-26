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
  // --- menu flow: title → team select (red) ---
  await page.waitForSelector('#menu:not(.hidden)', { timeout: 30000 });
  await page.click('#btn-play');
  await page.click('#btn-team-red');
  await page.waitForFunction(() => window.__woa !== undefined, null, { timeout: 30000 });
  await page.waitForTimeout(600);

  const state = () => page.evaluate(() => window.__woa.state());
  const tile = (layer, x, y) => page.evaluate((a) => window.__woa.tile(a.l, a.x, a.y), { l: layer, x, y });
  const dump = async (name) => {
    const s = await state();
    writeFileSync(path.join(OUT, `${name}.json`), JSON.stringify(s, null, 2));
    return s;
  };
  const shot = (name) => page.screenshot({ path: path.join(OUT, `${name}.png`) });

  // --- founding start: lone flying queen on the surface ---
  let s = await dump('s01-flight-start');
  await shot('s01-flight');
  console.log('start:', JSON.stringify({ tick: s.tick, phase: s.phase, team: s.team, queen: s.queen }));
  if (s.phase !== 0) failures.push(`expected flight phase, got ${s.phase}`);
  if (s.team !== 0) failures.push(`expected red team, got ${s.team}`);
  if (s.queen === null || s.queen.layer !== 'S' || s.queen.state !== 4) {
    failures.push(`queen not flying on surface: ${JSON.stringify(s.queen)}`);
  }
  if (s.ants.length !== 0) failures.push(`expected no ants at start, got ${s.ants.length}`);
  if (s.entrance !== null) failures.push('entrance should not exist during flight');

  // --- flight steering (hold LMB right of the camera-locked queen) ---
  {
    const before = s.queen.x;
    await page.mouse.move(840, 300);
    await page.mouse.down();
    await page.mouse.move(940, 300, { steps: 10 });
    await page.evaluate(() => window.__woa.step(80));
    await page.mouse.up();
    const s2 = await state();
    const after = s2.queen.x;
    console.log('fly:', JSON.stringify({ before, after: +after.toFixed(2) }));
    if (after - before < 0.5) failures.push(`flight steering did not move the queen (${before} -> ${after})`);
  }

  // --- land (RMB), walk, found the nest (RMB) ---
  s = await state();
  await page.evaluate((p) => window.__woa.click(p.x, p.y, 2), { x: s.queen.x, y: s.queen.y });
  s = await state();
  if (s.phase !== 1) failures.push(`land did not enter grounded phase (${s.phase})`);
  await page.evaluate((p) => window.__woa.click(p.x, p.y, 0), { x: s.queen.x + 4, y: s.queen.y });
  await page.evaluate(() => window.__woa.step(80));
  s = await state();
  const nestTile = [Math.floor(s.queen.x) & ~1, Math.floor(s.queen.y) & ~1]; // block origin
  await page.evaluate((p) => window.__woa.click(p.x, p.y, 2), { x: s.queen.x, y: s.queen.y });
  await page.evaluate(() => window.__woa.step(3));
  s = await state();
  await shot('s02-nest');
  console.log('nest:', JSON.stringify({ phase: s.phase, entrance: s.entrance, layer: s.layer, dug: s.dug }));
  if (s.phase !== 2) failures.push(`found did not enter founding phase (${s.phase})`);
  if (!Array.isArray(s.entrance) || s.entrance[0] !== nestTile[0] || s.entrance[1] !== nestTile[1]) {
    failures.push(`entrance ${JSON.stringify(s.entrance)} not at queen tile ${nestTile}`);
  }
  if (s.queen.layer !== 'U') failures.push('queen did not move underground on founding');
  if (s.layer !== 'underground') failures.push('camera did not follow the queen underground');
  if (s.dug < 10) failures.push(`starter chamber not fully carved (${s.dug})`);

  // --- dig a wall, refill it, dig again, haul the dirt out, drop it above ---
  const [ex, ey] = s.entrance;
  // frontier blocks: fully-soft 2×2 blocks touching the chamber
  const blocks = [
    [ex + 2, ey + 2], [ex + 2, ey], [ex + 2, ey + 4],
    [ex - 2, ey + 2], [ex, ey + 6], [ex + 2, ey + 6],
  ];
  let wall = null;
  for (const [bx, by] of blocks) {
    const cells = await page.evaluate((b) => {
      const ks = [];
      for (let dy = 0; dy < 2; dy++) for (let dx = 0; dx < 2; dx++) {
        ks.push(window.__woa.tile(1, b.bx + dx, b.by + dy));
      }
      return ks;
    }, { bx, by });
    if (cells.every((k) => k >= 1 && k <= 3)) {
      wall = { wx: bx, wy: by };
      break;
    }
  }
  if (!wall) {
    failures.push('no fully-soft block around the starter chamber (seed-dependent?)');
  } else {
    // right-click distant dirt: the queen walks there and digs on arrival
    await page.evaluate((p) => window.__woa.click(p.x + 0.5, p.y + 0.5, 2), { x: wall.wx, y: wall.wy });
    await page.evaluate(() => window.__woa.step(140));
    const dugEmpty = await tile(1, wall.wx, wall.wy);
    if (dugEmpty !== 0) failures.push(`walk-to-dig did not clear the block (kind ${dugEmpty})`);
    s = await state();
    if (s.queen.aux !== 2) failures.push(`queen not hauling dirt after dig (aux ${s.queen.aux})`);
    // refill the whole block
    await page.evaluate((p) => window.__woa.click(p.x + 0.5, p.y + 0.5, 2), { x: wall.wx, y: wall.wy });
    await page.evaluate(() => window.__woa.step(3)); // force a snapshot pull
    s = await state();
    const refilled = await tile(1, wall.wx, wall.wy);
    console.log('dirt:', JSON.stringify({ aux: s.queen.aux, refilled }));
    if (refilled < 1 || refilled > 3) failures.push(`dump did not refill the block (kind ${refilled})`);
    if (s.queen.aux !== 0) failures.push(`queen still hauling after dump (aux ${s.queen.aux})`);
    // dig again and haul it out through the entrance
    await page.evaluate((p) => window.__woa.click(p.x + 0.5, p.y + 0.5, 2), { x: wall.wx, y: wall.wy });
    await page.evaluate(() => window.__woa.step(140));
    await page.evaluate((p) => window.__woa.click(p.x + 0.5, p.y + 0.5, 2), { x: ex, y: ey });
    await page.evaluate(() => window.__woa.step(120));
    s = await state();
    if (s.queen.layer !== 'S') failures.push('queen did not reach the surface while hauling');
    await page.evaluate((p) => window.__woa.click(p.x, p.y, 2), { x: s.queen.x + 3, y: s.queen.y });
    await page.evaluate(() => window.__woa.step(3)); // force a snapshot pull
    s = await state();
    if (s.queen.aux !== 0) failures.push(`surface dump did not discard the dirt (aux ${s.queen.aux})`);
    // back into the nest — the founding queen must not linger among spiders
    await page.evaluate((p) => window.__woa.click(p.x + 0.5, p.y + 0.5, 2), { x: ex, y: ey });
    await page.evaluate(() => window.__woa.step(120));
    s = await state();
    console.log('haul:', JSON.stringify({ layer: s.queen.layer, aux: s.queen.aux, dead: s.dead }));
    if (s.queen.layer !== 'U') failures.push('queen did not return into the nest after hauling');
    if (s.dead) failures.push('colony died during the founding haul');
  }

  // --- founding timer → brood: eggs wait until orange soil exists ---
  await page.evaluate(() => window.__woa.step(1250));
  s = await dump('s03-brood');
  console.log('brood:', JSON.stringify({ phase: s.phase, eggs: s.eggs }));
  if (s.phase !== 3) failures.push(`founding timer did not end in brood (${s.phase})`);
  if (s.eggs !== 4) failures.push(`expected 4 founding eggs, got ${s.eggs}`);
  await page.evaluate(() => window.__woa.step(3700));
  s = await state();
  if (s.phase !== 3 || s.workers !== 0) {
    failures.push(`eggs hatched without orange soil (phase ${s.phase}, workers ${s.workers})`);
  }
  // paint orange under the queen only: some eggs hatch, the rest wait —
  // leaving eggs for the transport round-trip below
  const q0 = s.queen;
  await page.evaluate((p) => window.__woa.setsoil(1, Math.floor(p.x), Math.floor(p.y), 1), { x: q0.x, y: q0.y });
  await page.evaluate(() => window.__woa.step(80));
  s = await dump('s04-colony');
  await shot('s04-colony');
  console.log('colony:', JSON.stringify({ phase: s.phase, workers: s.workers, dead: s.dead }));
  if (s.phase !== 4) failures.push(`hatching on orange did not start the colony phase (${s.phase})`);
  if (s.workers < 1) failures.push(`no egg hatched on orange soil (workers ${s.workers})`);
  // egg transport round-trip with one of the remaining eggs
  const eggState = await state();
  if (eggState.eggs > 0) {
    // walk the queen onto an egg cell, then right-click to pick it up
    const eg = eggState.queen;
    await page.evaluate((p) => window.__woa.click(p.x, p.y, 0), { x: eg.x + 1, y: eg.y });
    await page.evaluate(() => window.__woa.step(60));
    await page.evaluate((p) => window.__woa.click(p.x, p.y, 2), { x: eg.x + 1, y: eg.y });
    await page.evaluate(() => window.__woa.step(3));
    s = await state();
    if (s.queen.aux !== 3) failures.push(`queen not marked as egg-carrier (aux ${s.queen.aux})`);
    // place it back on an adjacent empty cell
    const qn = s.queen;
    await page.evaluate((p) => window.__woa.click(p.x, p.y, 2), { x: qn.x + 1, y: qn.y });
    await page.evaluate(() => window.__woa.step(3));
    s = await state();
    if (s.queen.aux !== 0) failures.push(`egg not placed (aux ${s.queen.aux})`);
    const log2 = await page.evaluate(() => window.__woa.log());
    const acts2 = [...new Set(log2.events.filter((e) => e.type === 'cmd').map((e) => e.act))];
    if (!acts2.includes('pick-egg')) failures.push('right-click did not pick up an adjacent egg');
    if (!acts2.includes('drop')) failures.push('egg drop not logged');
  }

  // --- control a worker: cycle to it (C — click-selecting a foraging worker
  // is racy), walk to the surface, steer, fight ---
  const queenId = s.queen ? s.queen.id : null;
  const worker = s.ants[0];
  if (!worker) {
    failures.push('no worker to control after hatch');
  } else {
    await page.evaluate(() => window.__woa.pause()); // freeze positions while switching
    await page.evaluate(() => window.__woa.key('KeyC'));
    s = await state();
    if (s.playerAnt === queenId || s.playerAnt === null) {
      failures.push(`C did not cycle from the queen to a worker (playerAnt ${s.playerAnt})`);
    }
    // out through the entrance for the fight (entrance toggles layers)
    const me0 = s.ants.find((a) => a.id === s.playerAnt);
    if (me0 && me0.layer === 'U') {
      await page.evaluate((p) => window.__woa.click(p.x + 0.5, p.y + 0.5, 2), { x: ex, y: ey });
    }
    await page.evaluate(() => window.__woa.pause()); // resume
    // the cycled worker may be mid-delivery and dive into the nest — keep
    // sending it up until it actually holds the surface
    let surfaced = false;
    for (let i = 0; i < 4 && !surfaced; i++) {
      await page.evaluate(() => window.__woa.step(150));
      await page.evaluate(() => window.__woa.pause());
      s = await state();
      const w = s.ants.find((a) => a.id === s.playerAnt);
      surfaced = !!(w && w.layer === 'S');
      if (!surfaced) {
        await page.evaluate((p) => window.__woa.click(p.x + 0.5, p.y + 0.5, 2), { x: ex, y: ey });
      }
      await page.evaluate(() => window.__woa.pause()); // resume
    }
    s = await state();
    const me = s.ants.find((a) => a.id === s.playerAnt);
    if (!me || me.layer !== 'S') failures.push('worker did not reach the surface');

    // hold-to-steer the worker (camera locked on it)
    const before = s.ants.find((a) => a.id === s.playerAnt);
    await page.mouse.move(840, 300);
    await page.mouse.down();
    await page.mouse.move(940, 300, { steps: 10 });
    await page.evaluate(() => window.__woa.step(80));
    await page.mouse.up();
    const s2 = await state();
    const after = s2.ants.find((a) => a.id === s.playerAnt);
    console.log('steer:', JSON.stringify({ before: before && before.x, after: after && +after.x.toFixed(2) }));
    if (before && after && after.x - before.x < 0.5) {
      failures.push(`steering did not move the worker (${before.x} -> ${after.x})`);
    }

    // fight: pause so the wandering spider can't dodge the coordinate click
    await page.evaluate(() => window.__woa.pause());
    const spider = (await state()).spiders[0];
    if (spider) {
      await page.evaluate((p) => window.__woa.click(p.x, p.y, 0), { x: spider.x, y: spider.y });
      await page.evaluate(() => window.__woa.pause()); // resume
      await page.evaluate(() => window.__woa.step(1500));
      await page.waitForTimeout(300);
      s = await dump('s05-fight');
      await shot('s05-fight');
      const hurt = s.spiders.find((sp) => sp.id === spider.id);
      console.log('fight:', JSON.stringify({ spiderBefore: spider.hp, spiderAfter: hurt ? hurt.hp : 'dead', antsLeft: s.ants.length }));
      if (hurt && hurt.hp >= 1.0) failures.push('attack did not damage the spider');
      if (s.ants.length === 0) failures.push('all ants died');
    } else {
      console.log('no spider visible on surface — skipping fight');
      await page.evaluate(() => window.__woa.pause()); // resume
    }
  }

  await page.evaluate(() => window.__woa.step(3000));
  await page.waitForTimeout(400);
  s = await dump('s06-late');
  console.log('late:', JSON.stringify({ tick: s.tick, workers: s.workers, food: s.food, dead: s.dead }));
  if (s.dead) failures.push('colony died during the late window');

  const ilog = await page.evaluate(() => window.__woa.log());
  writeFileSync(path.join(OUT, 's07-inputlog.json'), JSON.stringify(ilog, null, 2));
  const types = [...new Set(ilog.events.map((e) => e.type))];
  const acts = [...new Set(ilog.events.filter((e) => e.type === 'cmd').map((e) => e.act))];
  console.log('inputlog:', JSON.stringify({ count: ilog.count, dropped: ilog.dropped, types, acts }));
  if (ilog.count === 0) failures.push('input log recorded no events');
  else {
    if (!types.includes('start')) failures.push('input log missing start event');
    if (!types.includes('view')) failures.push('input log missing view toggle');
    for (const want of ['land', 'found', 'drop', 'entrance', 'pick-egg']) {
      if (!acts.includes(want)) failures.push(`input log missing ${want} command`);
    }
    if (acts.includes('dig') === false && !failures.some((f) => f.includes('wall'))) {
      failures.push('input log missing dig command');
    }
    const ticks = ilog.events.map((e) => e.tick);
    if (ticks.some((tk, i) => i > 0 && tk < ticks[i - 1])) {
      failures.push('input log ticks not monotonic');
    }
  }

  // --- dev tools: console API + F2 panel ---
  await page.evaluate(() => window.__woa.setfood(999));
  await page.evaluate(() => window.__woa.spawn('spider', 50.5, 3.5));
  await page.evaluate(() => window.__woa.step(3));
  s = await state();
  if (s.carbs < 500) failures.push(`dev setfood failed: ${s.carbs}`);
  if (s.spiders.length !== 3) failures.push(`dev spawn spider failed: ${s.spiders.length}`);
  const foodsBefore = s.foods;
  await page.mouse.click(640, 400); // focus canvas for key events
  await page.keyboard.press('F2');
  const panelVisible = await page.evaluate(() => !document.getElementById('devpanel').classList.contains('hidden'));
  if (!panelVisible) failures.push('F2 did not open the dev panel');
  await page.click('button[data-spawn="moss"]');
  await page.mouse.click(640, 420);
  await page.evaluate(() => window.__woa.step(3));
  s = await state();
  // one dev food pile = 45 units spread across cells at the 6/cell cap
  if (s.foods <= foodsBefore) failures.push(`panel food placement failed: ${s.foods} vs ${foodsBefore}`);
  await page.keyboard.press('F2');

  await page.keyboard.press('F3');
  await page.waitForTimeout(700);
  const perfText = await page.evaluate(() => document.getElementById('perf').textContent);
  if (!perfText || !perfText.includes('fps')) failures.push(`perf overlay not reporting: ${perfText}`);
  else console.log('perf:', perfText.split('\n')[0]);
  await page.keyboard.press('F3');

  // --- regenerate the determinism fixture from this very session ---
  const replayExport = await page.evaluate(() => window.__woa.replay());
  const fixturePath = path.join(ROOT, 'public', 'replays', 'determinism.json');
  writeFileSync(fixturePath, JSON.stringify(replayExport, null, 1) + '\n');
  console.log('fixture:', JSON.stringify({ version: replayExport.version, team: replayExport.team, ticks: replayExport.ticks, cmds: replayExport.cmds.length }));

  // --- cross-platform determinism: same replay on native and WASM ---
  const nativeDump = await new Promise((resolve, reject) => {
    const rustBin = `${process.env.HOME}/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin`;
    const p = spawn(
      'cargo',
      ['run', '--example', 'determinism_dump', '--', fixturePath],
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

  // step and read canon in ONE JS task — a separate round-trip lets the
  // realtime rAF loop tick once in between and skew the comparison
  const wasmCanon = await page.evaluate((target) => {
    window.__woa.stepTo(target);
    return window.__woa.canon();
  }, replayExport.ticks);
  writeFileSync(path.join(OUT, 's08-wasm-canon.txt'), wasmCanon);
  writeFileSync(path.join(OUT, 's08-native-canon.txt'), nativeDump);
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
