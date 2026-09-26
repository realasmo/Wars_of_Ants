import type { Sim } from './sim';

function el(id: string): HTMLElement {
  return document.getElementById(id) as HTMLElement;
}

const HELP_COLONY =
  'Hold left button: your ant follows the cursor · Left-click an ant: take control · Right-click: dig tile / attack spider / ' +
  'enter-exit nest (the marked hole) · Mouse wheel: zoom · C: control next ant · F2: dev tools · F3: perf · ' +
  'the camera follows your ant; spectate (Tab, drag, WASD) when it dies';

const HELP_FOUNDING =
  'Right-click dirt: walk there and dig (2×2) · Dirt: right-click empty space to refill a 2×2 block, haul it out and drop above ground to discard · ' +
  'Orange soil = eggs only hatch there · Silver = food never spoils there · Right-click an egg to carry it, again to place · F2 · F3';

const HELP_FLIGHT = 'Hold left button: fly toward the cursor · Right-click: land here · F2: dev tools · F3: perf';

const HELP_GROUNDED =
  'Hold left button: walk · Right-click: found the nest where the queen stands (orange/silver dust patches grant that soil nearby — dig to find it) · ' +
  'Keep ~6 tiles from the map edges · F2 · F3';

export class Hud {
  private helpText = '';

  update(sim: Sim, playerAnt: number | null, layer: number): void {
    const snap = playerAnt !== null ? sim.cur.get(playerAnt) : undefined;
    const state = snap
      ? snap.state === 1
        ? 'moving'
        : snap.state === 2
          ? 'digging'
          : snap.state === 3
            ? 'fighting'
            : snap.state === 4
              ? 'flying'
              : 'idle'
      : '';
    const counts = sim.casteCounts();
    el('stat-food').textContent = String(sim.food);
    el('stat-super').textContent = String(sim.superFood());
    el('stat-ants').textContent =
      counts.soldiers > 0 ? `${counts.workers} +${counts.soldiers}S` : String(counts.workers);
    el('stat-eggs').textContent = String(sim.eggCount());
    el('stat-dug').textContent = String(sim.tilesDug());
    const secs = Math.floor(sim.tickCount / 20);
    el('stat-time').textContent = `${Math.floor(secs / 60)}:${String(secs % 60).padStart(2, '0')}`;
    const phase = sim.phase();
    el('stat-phase').textContent = sim.foundingMode
      ? phase === 0
        ? 'flight'
        : phase === 1
          ? 'grounded'
          : phase === 2
            ? `founding ${Math.ceil(sim.phaseTime())}s`
            : phase === 3
              ? 'brood'
              : 'colony'
      : 'colony';
    el('ctrl-ant').textContent = snap
      ? snap.kind === 0
        ? `queen (${state})`
        : `ant #${playerAnt} (${state})`
      : 'spectating';
    el('ctrl-layer').textContent = layer === 0 ? 'Surface' : 'Underground';
    this.setHelp(
      phase === 0 ? HELP_FLIGHT : phase === 1 ? HELP_GROUNDED : phase >= 4 ? HELP_COLONY : HELP_FOUNDING,
    );
  }

  private setHelp(text: string): void {
    if (text === this.helpText) return;
    this.helpText = text;
    el('help').innerHTML = text;
  }

  showDead(sim: Sim, onRestart: () => void): void {
    const secs = Math.floor(sim.tickCount / 20);
    el('overlay-sub').textContent =
      `The colony lasted ${Math.floor(secs / 60)}m ${secs % 60}s — ` +
      `${sim.antsAlive()} ants, ${sim.tilesDug()} tiles dug.`;
    el('overlay').classList.remove('hidden');
    const btn = el('btn-restart');
    const clone = btn.cloneNode(true) as HTMLElement;
    btn.replaceWith(clone);
    clone.addEventListener('click', onRestart);
  }

  hideDead(): void {
    el('overlay').classList.add('hidden');
  }

  setReplay(active: boolean): void {
    el('replay-badge').classList.toggle('hidden', !active);
  }

  private perfVisible = false;

  togglePerf(): boolean {
    this.perfVisible = !this.perfVisible;
    el('perf').classList.toggle('hidden', !this.perfVisible);
    return this.perfVisible;
  }

  setPerf(text: string): void {
    if (this.perfVisible) el('perf').textContent = text;
  }
}
