import type { Sim } from './sim';

function el(id: string): HTMLElement {
  return document.getElementById(id) as HTMLElement;
}

export class Hud {
  update(sim: Sim, playerAnt: number | null, layer: number): void {
    const snap = playerAnt !== null ? sim.cur.get(playerAnt) : undefined;
    const state = snap
      ? snap.state === 1
        ? 'moving'
        : snap.state === 2
          ? 'digging'
          : snap.state === 3
            ? 'fighting'
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
    el('ctrl-ant').textContent = snap ? `ant #${playerAnt} (${state})` : 'spectating';
    el('ctrl-layer').textContent = layer === 0 ? 'Surface' : 'Underground';
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
}
