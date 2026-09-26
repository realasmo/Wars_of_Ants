import { chromium } from 'playwright';
const b = await chromium.launch({ executablePath: '/usr/bin/chromium', headless: true,
  args: ['--no-sandbox', '--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
const p = await b.newPage();
p.on('console', (m) => { if (m.text().includes('debug')) console.log('[page]', m.text()); });
await p.goto('http://[::1]:5173/?seed=42');
await p.waitForSelector('#menu:not(.hidden)', { timeout: 30000 });
await p.click('#btn-play');
await p.click('#btn-team-red');
await p.waitForFunction(() => window.__woa !== undefined, null, { timeout: 30000 });
await p.waitForTimeout(300);
const st = () => p.evaluate(() => window.__woa.state());
await p.evaluate((q) => { window.__woa.click(q.x, q.y, 2); window.__woa.click(q.x, q.y, 2); }, (await st()).queen);
await p.evaluate(() => window.__woa.step(1250));
let s = await st();
if (s.phase === 3) {
  const q = s.queen;
  await p.evaluate((p2) => window.__woa.setsoil(1, Math.floor(p2.x), Math.floor(p2.y), 1), { x: q.x, y: q.y });
  await p.evaluate(() => window.__woa.step(3800));
}
await p.evaluate(() => window.__woa.setfood(200));
await p.evaluate(() => window.__woa.key('KeyC'));
s = await st();
const wid = s.playerAnt;
const w = s.ants.find((a) => a.id === wid);
await p.evaluate((p2) => { window.__woa.click(p2.x, p2.y, 0); window.__woa.spawn('strawberry', p2.x, p2.y); }, { x: w.x, y: w.y });
await p.evaluate(() => window.__woa.step(140));
s = await st();
const w2 = s.ants.find((a) => a.id === wid);
console.log('carrying:', w2.carrying, 'entrance:', JSON.stringify(s.entrance));
const ent = s.entrance;
await p.evaluate((e) => window.__woa.click(e[0] + 1, e[1] + 1, 2), ent);
await p.evaluate(() => window.__woa.step(300));
s = await st();
const w3 = s.ants.find((a) => a.id === wid);
console.log('final:', w3.layer, w3.carrying);
await b.close();
