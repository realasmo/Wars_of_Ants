import type { AntEnt, Sim } from './sim';

function el(id: string): HTMLElement {
  return document.getElementById(id) as HTMLElement;
}

/** Carry suffix for the controlling line: ' · egg' etc., so a full-handed
 * ant is always visible (a refused dig is silent otherwise). */
function carrySuffix(a: AntEnt): string {
  if (a.carry.t === 'none') return '';
  if (a.carry.t === 'dirt') return ` · dirt×${a.carry.blocks}`;
  return ` · ${a.carry.t}`;
}

const HELP_COLONY =
  'Hold left button: your ant follows the cursor · Left-click an ant: take control · Right-click: dig tile / attack spider / ' +
  'enter-exit nest (the marked hole) · Mouse wheel: zoom · C: control next ant · X: squad — they follow you, farm with you ' +
  '(and keep at it), and attack your target; X, 1 recalls them · as the QUEEN, X: brood menu (order eggs for pantry food) · ' +
  'F2: dev tools · F3: perf · the camera follows your ant; spectate (Tab, drag, WASD) when it dies';

const HELP_FOUNDING =
  'Right-click dirt: walk there and dig (2×2) · You can carry two blocks — right-click empty space to refill one, haul out and drop above ground to discard all · ' +
  'Orange soil = eggs only hatch there · Silver = food never spoils there · Right-click an egg to carry it, again to place · ' +
  'X: brood menu (order eggs for pantry food) · F2 · F3';

const HELP_FLIGHT = 'Hold left button: fly toward the cursor · Right-click: land here · F2: dev tools · F3: perf';

const HELP_GROUNDED =
  'Hold left button: walk · Right-click: found the nest where the queen stands (orange/silver dust patches grant that soil nearby — dig to find it) · ' +
  'Keep ~6 tiles from the map edges · F2 · F3';

export class Hud {
  private helpText = '';

  update(sim: Sim, playerAnt: number | null, layer: number): void {
    const snap = playerAnt !== null ? sim.ant(playerAnt) : undefined;
    const state = snap ? snap.activity : '';
    const counts = sim.casteCounts();
    el('stat-protein').textContent = String(sim.storeProtein());
    el('stat-carbs').textContent = String(sim.storeCarbs());
    el('stat-water').textContent = String(sim.storeWater());
    el('stat-honeydew').textContent = String(sim.storeHoneydew());
    const wants = el('stat-wants');
    const req = sim.queenRequest();
    if (req === null) {
      wants.textContent = '—';
      wants.style.color = '';
    } else {
      wants.textContent = req.hunger > 0 ? `${req.request} NOW` : req.request;
      wants.style.color = req.hunger > 0 ? '#e8544f' : '';
    }
    const extra = [
      counts.soldiers > 0 ? `${counts.soldiers}S` : '',
      counts.honeys > 0 ? `${counts.honeys}H` : '',
      counts.medics > 0 ? `${counts.medics}M` : '',
    ].filter(Boolean).join(' ');
    el('stat-ants').textContent =
      extra.length > 0 ? `${counts.workers} +${extra}` : String(counts.workers);
    el('stat-eggs').textContent = String(sim.eggCount());
    el('stat-dug').textContent = `${sim.tilesDug()} · map ${sim.w}×${sim.h}`;
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
      ? snap.kind === 'queen'
        ? `queen (${state}${carrySuffix(snap)})`
        : `ant #${playerAnt} (${state}${carrySuffix(snap)})`
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

  /** Temporarily replace the help bar with an action hint (refusals etc.). */
  private hintTimer: ReturnType<typeof setTimeout> | null = null;

  flashHint(text: string, ms = 2200): void {
    if (this.hintTimer !== null) clearTimeout(this.hintTimer);
    el('help').textContent = text;
    this.hintTimer = setTimeout(() => {
      this.hintTimer = null;
      el('help').innerHTML = this.helpText;
    }, ms);
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
