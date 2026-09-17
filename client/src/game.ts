import { Renderer } from './render';
import { Input } from './input';
import { Hud } from './hud';
import { Sim, TPS } from './sim';
import type { Snap } from './sim';

export class Game {
  sim: Sim;
  private renderer: Renderer;
  private input: Input;
  private hud = new Hud();
  private playerAnt: number | null = null;
  private seed: number;
  private acc = 0;
  private last = performance.now();
  private running = false;
  private hudCounter = 0;
  private deadShown = false;

  private constructor(sim: Sim, renderer: Renderer) {
    this.sim = sim;
    this.renderer = renderer;
    this.seed = sim.entrance[0];
    this.input = new Input(renderer);
    this.input.onCommand = (x, y, b) => this.handleClick(x, y, b);
    this.input.onCycleAnt = () => this.cycleAnt();
    this.input.onToggleLayer = () => renderer.toggleLayer();
    this.playerAnt = sim.workers()[0] ?? null;
  }

  static async create(host: HTMLElement, seed: number): Promise<Game> {
    const sim = new Sim(seed);
    const renderer = await Renderer.create(sim, host);
    return new Game(sim, renderer);
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
    if (code === 'Tab') this.renderer.toggleLayer();
    else if (code === 'KeyC') this.cycleAnt();
  }

  debugStep(n: number): void {
    if (this.sim.dead) return;
    for (let i = 0; i < n; i++) this.sim.tick();
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
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
      spiders,
      ants,
    };
  }

  restart(): void {
    this.seed = Date.now() % 0x7fffffff;
    this.sim = new Sim(this.seed);
    this.playerAnt = this.sim.workers()[0] ?? null;
    this.renderer.reset(this.sim);
    this.hud.hideDead();
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
    this.acc = 0;
    this.deadShown = false;
  }

  private cycleAnt(): void {
    const workers = this.sim.workers();
    if (workers.length === 0) return;
    const i = this.playerAnt === null ? 0 : workers.indexOf(this.playerAnt);
    this.playerAnt = workers[(i + 1) % workers.length];
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
    if (this.sim.dead || this.playerAnt === null) return;
    const layer = this.renderer.activeLayer;
    const pick = this.pickEntity(x, y);
    if (pick && pick.kind === 4) {
      this.sim.attack(this.playerAnt, pick.id);
      this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
      return;
    }
    if (button === 0) {
      if (pick && (pick.kind === 1 || pick.kind === 5)) {
        this.playerAnt = pick.id;
      } else {
        this.sim.move(this.playerAnt, x, y);
      }
    } else if (button === 2) {
      const tx = Math.floor(x);
      const ty = Math.floor(y);
      const kind = this.sim.tileAt(layer, tx, ty);
      const soft = kind >= 1 && kind <= 3;
      const me = this.sim.cur.get(this.playerAnt);
      const adjacent =
        me !== undefined &&
        Math.max(Math.abs(me.x - tx - 0.5), Math.abs(me.y - ty - 0.5)) <= 1.5;
      if (layer === 1 && soft && adjacent) {
        if (!this.sim.dig(this.playerAnt, tx, ty)) {
          this.sim.move(this.playerAnt, tx + 0.5, ty + 0.5);
        }
      } else {
        this.sim.move(this.playerAnt, tx + 0.5, ty + 0.5);
      }
    }
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
  }

  private frame = (now: number): void => {
    const dt = Math.min(0.25, (now - this.last) / 1000);
    this.last = now;
    if (!this.sim.dead) {
      this.acc += dt;
      const step = 1 / TPS;
      while (this.acc >= step) {
        this.sim.tick();
        this.acc -= step;
      }
      if (++this.hudCounter >= 5) {
        this.hudCounter = 0;
        this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
      }
    }
    if (this.sim.dead && !this.deadShown) {
      this.deadShown = true;
      this.hud.showDead(this.sim, () => this.restart());
    }
    this.renderer.panContinuous(dt, this.input.keys);
    if (this.sim.consumeDirty(this.renderer.activeLayer)) {
      this.renderer.drawTiles(this.renderer.activeLayer);
    }
    this.renderer.renderEntities(this.acc * TPS, this.playerAnt);
    requestAnimationFrame(this.frame);
  };
}
