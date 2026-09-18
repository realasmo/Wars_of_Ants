import { Renderer } from './render';
import { Input } from './input';
import { Hud } from './hud';
import { Sim, TPS } from './sim';
import type { Snap } from './sim';
import { InputLog, r2 } from './inputlog';
import { coreVersion } from './wasm';
import { DevPanel } from './devpanel';
import type { Replay } from './replay';

export class Game {
  sim: Sim;
  private renderer: Renderer;
  private input: Input;
  private hud = new Hud();
  private dev: DevPanel;
  private paused = false;
  private readonly log = new InputLog();
  private playerAnt: number | null = null;
  private seed: number;
  private acc = 0;
  private last = performance.now();
  private running = false;
  private hudCounter = 0;
  private deadShown = false;
  private replay: { cmds: Replay['cmds']; i: number } | null = null;

  private constructor(sim: Sim, renderer: Renderer, seed: number, replay: Replay | null) {
    this.sim = sim;
    this.renderer = renderer;
    this.seed = seed;
    this.log.setTickSource(() => this.sim.tickCount);
    this.log.setSink(document.getElementById('inputlog'));
    this.input = new Input(renderer, this.log);
    this.input.onCommand = (x, y, b) => this.handleClick(x, y, b);
    this.input.onCycleAnt = () => this.cycleAnt();
    this.input.onToggleLayer = () => this.toggleLayer();
    this.input.onToggleDev = () => this.dev.toggle();
    this.input.onEscape = () => this.dev.setPlacement(null);
    this.dev = new DevPanel({
      onFood: () => this.debugSetFood(50),
      onSuper: () => this.debugSetSuper(5),
      onKillSpiders: () => this.debugKillSpiders(),
      onPause: () => this.togglePause(),
      onFF: () => this.debugStep(200),
      onPauseState: () => this.paused,
    });
    this.playerAnt = sim.workers()[0] ?? null;
    this.log.push({ type: 'start', seed, workers: sim.workers().length });
    if (replay) {
      this.replay = { cmds: [...replay.cmds].sort((a, b) => a.t - b.t), i: 0 };
      this.hud.setReplay(true);
      this.log.push({ type: 'start', note: 'replay', cmds: replay.cmds.length });
    }
  }

  static async create(host: HTMLElement, seed: number, replay: Replay | null = null): Promise<Game> {
    const sim = new Sim(seed);
    const renderer = await Renderer.create(sim, host);
    return new Game(sim, renderer, seed, replay);
  }

  start(): void {
    if (this.running) return;
    this.running = true;
    requestAnimationFrame(this.frame);
  }

  get controlling(): number | null {
    return this.playerAnt;
  }

  debugClick(x: number, y: number, button: number): void {
    this.handleClick(x, y, button);
  }

  debugKey(code: string): void {
    if (code === 'Tab') this.toggleLayer();
    else if (code === 'KeyC') this.cycleAnt();
  }

  debugLog(): Record<string, unknown> {
    const c = this.renderer.cam;
    return {
      seed: this.seed,
      layer: this.renderer.activeLayer === 0 ? 'surface' : 'underground',
      cam: { x: r2(c.x), y: r2(c.y), zoom: r2(c.zoom) },
      timeOrigin: performance.timeOrigin,
      ...this.log.dump(),
    };
  }

  debugMark(label: string): void {
    this.log.push({ type: 'mark', label });
  }

  debugStep(n: number): void {
    if (this.sim.dead) return;
    for (let i = 0; i < n; i++) this.tickWithReplay();
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
  }

  debugStepTo(target: number): void {
    if (this.sim.dead) return;
    while (this.sim.tickCount < target) this.tickWithReplay();
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
  }

  debugReplay(): Record<string, unknown> {
    const cmds = this.log.dump()
      .events.filter((e) => e.type === 'cmd' && e.src !== 'replay')
      .map((e) => {
        const c: Record<string, unknown> = { t: e.tick, act: e.act, ant: e.ant };
        if (e.x !== undefined) c.x = e.x;
        if (e.y !== undefined) c.y = e.y;
        if (e.tx !== undefined) c.tx = e.tx;
        if (e.ty !== undefined) c.ty = e.ty;
        if (e.target !== undefined) c.target = e.target;
        if (e.kind !== undefined) c.kind = e.kind;
        if (e.n !== undefined) c.n = e.n;
        return c;
      });
    return {
      version: 1,
      seed: this.seed,
      name: `session-${new Date().toISOString().slice(0, 19)}`,
      core: coreVersion(),
      ticks: this.sim.tickCount,
      cmds,
    };
  }

  debugCanon(): string {
    return this.sim.canonical();
  }

  debugSpawn(kind: string, x: number, y: number): boolean {
    if (this.sim.dead || this.replay !== null) return false;
    const id = this.sim.devSpawn(kind, x, y);
    if (id === 4294967295) return false;
    this.log.push({ type: 'cmd', act: 'dev-spawn', kind, x: r2(x), y: r2(y) });
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
    return true;
  }

  debugSetFood(n: number): void {
    if (this.sim.dead || this.replay !== null) return;
    this.sim.devSetFood(n);
    this.log.push({ type: 'cmd', act: 'dev-food', n });
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
  }

  debugSetSuper(n: number): void {
    if (this.sim.dead || this.replay !== null) return;
    this.sim.devSetSuper(n);
    this.log.push({ type: 'cmd', act: 'dev-super', n });
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
  }

  debugKillSpiders(): void {
    if (this.sim.dead || this.replay !== null) return;
    let count = 0;
    for (const s of [...this.sim.cur.values()]) {
      if (s.kind === 4 && this.sim.devKill(s.id)) count++;
    }
    this.log.push({ type: 'cmd', act: 'dev-kill-spiders', count });
  }

  debugKill(id: number): boolean {
    if (this.sim.dead || this.replay !== null) return false;
    const ok = this.sim.devKill(id);
    if (ok) this.log.push({ type: 'cmd', act: 'dev-kill', target: id });
    return ok;
  }

  togglePause(): void {
    this.paused = !this.paused;
    this.log.push({ type: 'mark', label: this.paused ? 'paused' : 'resumed' });
  }

  debugPixel(x: number, y: number): number[] {
    return this.renderer.pixelAt(x, y);
  }

  debugState(): Record<string, unknown> {
    const counts = this.sim.casteCounts();
    const spiders: Record<string, unknown>[] = [];
    const ants: Record<string, unknown>[] = [];
    for (const s of this.sim.cur.values()) {
      if (s.kind === 4) {
        spiders.push({ id: s.id, x: +s.x.toFixed(2), y: +s.y.toFixed(2), hp: +s.hp.toFixed(2), layer: s.layer });
      } else if (s.kind === 1 || s.kind === 5) {
        ants.push({ id: s.id, kind: s.kind === 5 ? 'soldier' : 'worker', x: +s.x.toFixed(2), y: +s.y.toFixed(2), hp: +s.hp.toFixed(2), state: s.state, carrying: s.extra, layer: s.layer === 0 ? 'S' : 'U' });
      }
    }
    let foods = 0;
    for (const s of this.sim.cur.values()) if (s.kind === 2) foods++;
    return {
      tick: this.sim.tickCount,
      dead: this.sim.dead,
      food: this.sim.food,
      super: this.sim.superFood(),
      workers: counts.workers,
      soldiers: counts.soldiers,
      eggs: this.sim.eggCount(),
      dug: this.sim.tilesDug(),
      layer: this.renderer.activeLayer === 0 ? 'surface' : 'underground',
      playerAnt: this.playerAnt,
      entrance: this.sim.entrance,
      paused: this.paused,
      spiders,
      ants,
      foods,
    };
  }

  restart(): void {
    if (this.replay !== null) {
      this.replay = null;
      this.hud.setReplay(false);
      this.log.push({ type: 'mark', label: 'replay-aborted-by-restart' });
    }
    this.log.push({ type: 'restart', seed: Date.now() % 0x7fffffff });
    this.seed = Date.now() % 0x7fffffff;
    this.sim = new Sim(this.seed);
    this.playerAnt = this.sim.workers()[0] ?? null;
    this.renderer.reset(this.sim);
    this.hud.hideDead();
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
    this.acc = 0;
    this.deadShown = false;
    this.log.push({ type: 'start', seed: this.seed, workers: this.sim.workers().length });
  }

  private toggleLayer(): void {
    this.renderer.toggleLayer();
    this.log.push({ type: 'view', layer: this.renderer.activeLayer === 0 ? 'surface' : 'underground' });
  }

  private cycleAnt(): void {
    const workers = this.sim.workers();
    if (workers.length === 0) return;
    const i = this.playerAnt === null ? 0 : workers.indexOf(this.playerAnt);
    const from = this.playerAnt;
    this.playerAnt = workers[(i + 1) % workers.length];
    this.log.push({ type: 'cmd', act: 'cycle', from, to: this.playerAnt });
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
  }

  private pickEntity(x: number, y: number): Snap | null {
    const layer = this.renderer.activeLayer;
    let best: Snap | null = null;
    let bestD = 0.6 * 0.6;
    for (const s of this.sim.cur.values()) {
      if (s.layer !== layer) continue;
      const d = (s.x - x) * (s.x - x) + (s.y - y) * (s.y - y);
      if (d < bestD) {
        bestD = d;
        best = s;
      }
    }
    return best;
  }

  private handleClick(x: number, y: number, button: number): void {
    if (this.sim.dead) return;
    const placing = this.dev.placement();
    if (placing !== null) {
      this.dev.setPlacement(null);
      this.debugSpawn(placing, x, y);
      return;
    }
    if (this.playerAnt === null) return;
    if (this.replay !== null) return; // replay is watch-only: camera/view stay live, sim commands don't
    const layer = this.renderer.activeLayer;
    const pick = this.pickEntity(x, y);
    if (pick && pick.kind === 4) {
      this.log.push({ type: 'cmd', act: 'attack', ant: this.playerAnt, target: pick.id });
      this.sim.attack(this.playerAnt, pick.id);
      this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
      return;
    }
    if (button === 0) {
      if (pick && (pick.kind === 1 || pick.kind === 5)) {
        this.playerAnt = pick.id;
        this.log.push({ type: 'cmd', act: 'select', ant: pick.id });
      } else {
        this.log.push({ type: 'cmd', act: 'move', ant: this.playerAnt, x: r2(x), y: r2(y) });
        this.sim.move(this.playerAnt, x, y);
      }
    } else if (button === 2) {
      const [ex, ey] = this.sim.entrance;
      const tx = Math.floor(x);
      const ty = Math.floor(y);
      if (Math.max(Math.abs(tx - ex), Math.abs(ty - ey)) <= 2) {
        this.log.push({ type: 'cmd', act: 'entrance', ant: this.playerAnt });
        this.sim.useEntrance(this.playerAnt);
      } else {
        const kind = this.sim.tileAt(layer, tx, ty);
        const soft = kind >= 1 && kind <= 3;
        const me = this.sim.cur.get(this.playerAnt);
        const adjacent =
          me !== undefined &&
          Math.max(Math.abs(me.x - tx - 0.5), Math.abs(me.y - ty - 0.5)) <= 1.5;
        if (layer === 1 && soft && adjacent) {
          if (this.sim.dig(this.playerAnt, tx, ty)) {
            this.log.push({ type: 'cmd', act: 'dig', ant: this.playerAnt, tx, ty });
          } else {
            this.log.push({ type: 'cmd', act: 'move', ant: this.playerAnt, x: tx + 0.5, y: ty + 0.5, note: 'dig-refused' });
            this.sim.move(this.playerAnt, tx + 0.5, ty + 0.5);
          }
        } else {
          this.log.push({ type: 'cmd', act: 'move', ant: this.playerAnt, x: tx + 0.5, y: ty + 0.5 });
          this.sim.move(this.playerAnt, tx + 0.5, ty + 0.5);
        }
      }
    }
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
  }

  private frame = (now: number): void => {
    const dt = Math.min(0.25, (now - this.last) / 1000);
    this.last = now;
    if (!this.sim.dead && !this.paused) {
      this.acc += dt;
      const step = 1 / TPS;
      while (this.acc >= step) {
        this.tickWithReplay();
        this.acc -= step;
      }
      if (++this.hudCounter >= 5) {
        this.hudCounter = 0;
        this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
      }
    }
    if (this.sim.dead && !this.deadShown) {
      this.deadShown = true;
      this.log.push({ type: 'death', tick: this.sim.tickCount, ants: this.sim.workers().length });
      this.hud.showDead(this.sim, () => this.restart());
    }
    this.renderer.panContinuous(dt, this.input.keys);
    if (this.sim.consumeDirty(this.renderer.activeLayer)) {
      this.renderer.drawTiles(this.renderer.activeLayer);
    }
    this.renderer.renderEntities(this.acc * TPS, this.playerAnt);
    requestAnimationFrame(this.frame);
  };

  private tickWithReplay(): void {
    this.sim.tick();
    const rp = this.replay;
    if (!rp) return;
    let applied = false;
    while (rp.i < rp.cmds.length && rp.cmds[rp.i].t <= this.sim.tickCount) {
      const c = rp.cmds[rp.i++];
      const ev: { type: string; src: string; act: unknown; ant: number } & Record<string, unknown> = {
        type: 'cmd',
        src: 'replay',
        act: c.act,
        ant: c.ant ?? 0,
      };
      if (c.x !== undefined) ev.x = c.x;
      if (c.y !== undefined) ev.y = c.y;
      if (c.tx !== undefined) ev.tx = c.tx;
      if (c.ty !== undefined) ev.ty = c.ty;
      if (c.target !== undefined) ev.target = c.target;
      this.log.push(ev);
      if (c.act === 'move') this.sim.move(c.ant ?? 0, c.x ?? 0, c.y ?? 0);
      else if (c.act === 'dig') this.sim.dig(c.ant ?? 0, c.tx ?? 0, c.ty ?? 0);
      else if (c.act === 'attack') this.sim.attack(c.ant ?? 0, c.target ?? 0);
      else if (c.act === 'entrance') this.sim.useEntrance(c.ant ?? 0);
      else if (c.act === 'dev-spawn') this.sim.devSpawn(String(c.kind), c.x ?? 0, c.y ?? 0);
      else if (c.act === 'dev-food') this.sim.devSetFood(Number(c.n ?? 0));
      else if (c.act === 'dev-super') this.sim.devSetSuper(Number(c.n ?? 0));
      else if (c.act === 'dev-kill-spiders') {
        for (const s of [...this.sim.cur.values()]) if (s.kind === 4) this.sim.devKill(s.id);
      } else if (c.act === 'dev-kill') this.sim.devKill(Number(c.target ?? 0));
      applied = true;
    }
    if (applied && rp.i >= rp.cmds.length) {
      this.replay = null;
      this.hud.setReplay(false);
    }
  };
}
