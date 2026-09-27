import type { Game } from './game';

/**
 * Quake-style dev console: Backquote (`) rolls a half-transparent window
 * down from the top. Live feed of sim events (polled per frame while open)
 * plus client-side lines, and a command line.
 */

let game: Game | null = null;
let open = false;
let seenTotal = -1;
const history: string[] = [];
let historyIdx = -1;
const MAX_LINES = 300;

export function bindConsole(g: Game): void {
  game = g;
  const input = document.getElementById('console-in') as HTMLInputElement;
  window.addEventListener('keydown', (e) => {
    if (e.code === 'Backquote') {
      e.preventDefault();
      toggle();
      return;
    }
    if (!open) return;
    if (document.activeElement !== input) return;
    if (e.code === 'Enter') {
      const line = input.value.trim();
      input.value = '';
      if (line.length > 0) {
        history.push(line);
        historyIdx = history.length;
        pushLine(`> ${line}`, 'cmd');
        run(line);
      }
    } else if (e.code === 'ArrowUp') {
      e.preventDefault();
      if (historyIdx > 0) input.value = history[--historyIdx];
    } else if (e.code === 'ArrowDown') {
      e.preventDefault();
      if (historyIdx < history.length - 1) input.value = history[++historyIdx];
      else {
        historyIdx = history.length;
        input.value = '';
      }
    }
    e.stopPropagation();
  });
  // NOTE: no stopPropagation here — the window-level handlers decide for
  // themselves (the game checks whether the console is open); blocking the
  // bubble would also block our own window listener below.
  pushLine('dev console — ` to toggle, type "help"', 'sys');
}

function toggle(): void {
  open = !open;
  document.getElementById('console')?.classList.toggle('hidden', !open);
  const input = document.getElementById('console-in') as HTMLInputElement;
  if (open) {
    input.focus();
    seenTotal = -1; // flush the backlog into view on open
  } else {
    input.blur();
  }
}

export function consoleOpen(): boolean {
  return open;
}

export function pushLine(text: string, cls = 'ev'): void {
  const out = document.getElementById('console-out');
  if (!out) return;
  const div = document.createElement('div');
  div.className = `console-line ${cls}`;
  div.textContent = text;
  out.appendChild(div);
  while (out.childElementCount > MAX_LINES && out.firstChild) {
    out.removeChild(out.firstChild);
  }
  out.scrollTop = out.scrollHeight;
}

/** Pull new sim events into the feed; call once per frame. */
export function pollEvents(): void {
  const g = game;
  if (g === null) return;
  const total = g.sim.eventTotal();
  if (seenTotal === -1) {
    // first poll after opening: show only the fresh tail, not 400 lines
    const lines = g.sim.eventLines();
    const tail = lines.slice(-25);
    for (const l of tail) pushLine(l);
    seenTotal = total;
    return;
  }
  if (total === seenTotal) return;
  const lines = g.sim.eventLines();
  // the ring caps at 400; reconstruct what's new from the total delta
  const delta = Math.min(total - seenTotal, lines.length);
  for (let i = lines.length - delta; i < lines.length; i++) {
    pushLine(lines[i]);
  }
  seenTotal = total;
}

function run(raw: string): void {
  const g = game;
  if (g === null) return;
  const parts = raw.split(/\s+/);
  const cmd = parts[0].toLowerCase();
  const num = (i: number) => Number(parts[i]);
  const argErr = (usage: string) => pushLine(`usage: ${usage}`, 'err');
  switch (cmd) {
    case 'help':
      pushLine(
        'spawn <kind> [x y] — worker soldier egg spider wood wool moss mushroom raspberry strawberry cockroach caterpillar nettle honey medic\n' +
          '  pantry units: pantry-protein | pantry-water | pantry-honeydew (physical piles)\n' +
          'setfood <n> | setsuper <n> | setwater <n> | sethoneydew <n> — stores\n' +
          'brood <worker|soldier|honey|medic> — queen X-menu order\n' +
          'soil <layer> <x> <y> <0|1|2> — paint soil block (orange=1 silver=2)\n' +
          'kill <id> | killspiders | pause | step <n>\n' +
          'coords — toggle click/world coordinate logging\n' +
          'state | canon | seed | events | clear',
        'sys',
      );
      break;
    case 'spawn': {
      if (parts.length < 2) return argErr('spawn <kind> [x y]');
      const kind = parts[1];
      const x = parts.length >= 4 ? num(2) : undefined;
      const y = parts.length >= 4 ? num(3) : undefined;
      const ok = g.debugSpawnAt(kind, x, y);
      pushLine(ok ? `spawned ${kind}` : `spawn refused (${kind})`, ok ? 'ev' : 'err');
      break;
    }
    case 'setfood':
    case 'setcarbs':
      if (parts.length < 2) return argErr('setfood <n>');
      g.debugSetFood(num(1));
      pushLine(`carbs = ${num(1)}`, 'cmd');
      break;
    case 'setsuper':
    case 'setprotein':
      if (parts.length < 2) return argErr('setsuper <n>');
      g.debugSetSuper(num(1));
      pushLine(`protein = ${num(1)}`, 'cmd');
      break;
    case 'setwater':
      if (parts.length < 2) return argErr('setwater <n>');
      g.debugSetWater(num(1));
      pushLine(`water = ${num(1)}`, 'cmd');
      break;
    case 'sethoneydew':
      if (parts.length < 2) return argErr('sethoneydew <n>');
      g.debugSetHoneydew(num(1));
      pushLine(`honeydew = ${num(1)}`, 'cmd');
      break;
    case 'brood': {
      if (parts.length < 2) return argErr('brood <worker|soldier|honey|medic>');
      const codes: Record<string, number> = { worker: 0, soldier: 1, honey: 2, medic: 3 };
      const code = codes[parts[1]];
      if (code === undefined) return argErr('brood <worker|soldier|honey|medic>');
      const reason = g.debugBrood(code);
      pushLine(reason === '' ? `brood ordered: ${parts[1]}` : `refused: ${reason}`, reason === '' ? 'cmd' : 'err');
      break;
    }
    case 'soil': {
      if (parts.length < 5) return argErr('soil <layer> <x> <y> <0|1|2>');
      g.debugSetSoil(num(1), num(2), num(3), num(4));
      pushLine(`soil painted (${num(1)},${num(2)},${num(3)}) = ${num(4)}`, 'cmd');
      break;
    }
    case 'kill': {
      if (parts.length < 2) return argErr('kill <id>');
      const ok = g.debugKill(num(1));
      pushLine(ok ? `killed #${num(1)}` : `kill refused (#${num(1)})`, ok ? 'ev' : 'err');
      break;
    }
    case 'killspiders':
      g.debugKillSpiders();
      pushLine('spiders killed', 'cmd');
      break;
    case 'pause':
      g.togglePause();
      pushLine(g.paused ? 'paused' : 'resumed', 'cmd');
      break;
    case 'step': {
      if (parts.length < 2) return argErr('step <n>');
      g.debugStep(num(1));
      pushLine(`stepped ${num(1)} ticks (t=${g.sim.tickCount})`, 'cmd');
      break;
    }
    case 'coords':
      pushLine(g.toggleCoords() ? 'coords logging ON — clicks log world positions' : 'coords logging OFF', 'sys');
      break;
    case 'state': {
      const s = g.debugState() as Record<string, unknown>;
      const q = s.queen as Record<string, unknown> | undefined;
      const feeder = s.feeder === null || s.feeder === undefined ? 'none' : `#${s.feeder}`;
      pushLine(
        `t=${s.tick} phase=${s.phase} P/C/W/H=${s.protein}/${s.carbs}/${s.water}/${s.honeydew} ` +
          `workers=${s.workers} eggs=${s.eggs} feeder=${feeder} ` +
          `queen-wants=${q?.request ?? '—'} (hunger ${q?.hunger ?? 0}) dead=${s.dead}`,
        'sys',
      );
      break;
    }
    case 'canon':
      pushLine(g.debugCanon().slice(0, 160) + '…', 'sys');
      break;
    case 'seed':
      pushLine(`seed = ${g.seedValue()}`, 'sys');
      break;
    case 'events': {
      const lines = g.sim.eventLines();
      for (const l of lines.slice(-20)) pushLine(l);
      break;
    }
    case 'clear': {
      const out = document.getElementById('console-out');
      if (out) out.innerHTML = '';
      seenTotal = g.sim.eventTotal();
      break;
    }
    default:
      pushLine(`unknown command: ${cmd} (try "help")`, 'err');
  }
}
