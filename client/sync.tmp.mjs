import { chromium } from 'playwright';
const b = await chromium.launch({ executablePath: '/usr/bin/chromium', headless: true,
  args: ['--no-sandbox', '--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
const p = await b.newPage();
p.on('pageerror', (e) => console.log('[pageerror]', String(e).slice(0, 200)));
await p.goto('http://[::1]:5173/?seed=42');
await p.waitForSelector('#menu:not(.hidden)', { timeout: 30000 });
await p.click('#btn-play');
await p.click('#btn-team-red');
await p.waitForFunction(() => window.__woa !== undefined, null, { timeout: 30000 });
await p.waitForTimeout(300);
const r = await p.evaluate(() => {
  const before = window.__woa.state();
  window.__woa.click(48.5, 48.5, 2);
  const after = window.__woa.state();
  return JSON.stringify({ before: before.phase, after: after.phase, tick: after.tick });
});
console.log('sync click:', r);
const r2 = await p.evaluate(() => {
  window.__woa.click(55.5, 52.5, 2);
  const s = window.__woa.state();
  return JSON.stringify({ phase: s.phase, queenState: s.queen.state, x: s.queen.x, y: s.queen.y });
});
console.log('distant click:', r2);
await b.close();
