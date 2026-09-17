import { WoaSim } from './wasm';

export interface Snap {
  id: number;
  kind: number;
  layer: number;
  x: number;
  y: number;
  state: number;
  extra: number;
}

export const TPS = 20;

export class Sim {
  private sim: WoaSim;
  readonly w: number;
  readonly h: number;
  readonly entrance: [number, number];
  prev = new Map<number, Snap>();
  cur = new Map<number, Snap>();
  tickCount = 0;
  dead = false;
  food = 0;
  private tilesCache: (Uint8Array | null)[] = [null, null];
  private tilesDirtyFlag = [true, true];

  constructor(seed: number, workers = 3, clusters = 6) {
    this.sim = new WoaSim(BigInt(Math.floor(seed)), workers, clusters);
    const dims = this.sim.dims();
    this.w = dims[0];
    this.h = dims[1];
    const e = this.sim.entrance();
    this.entrance = [e[0], e[1]];
    this.pull();
    this.pollTiles();
  }

  tick(): void {
    this.sim.tick(1);
    this.pull();
    this.pollTiles();
  }

  antsAlive(): number {
    return this.sim.ants_alive();
  }

  tilesDug(): number {
    return this.sim.tiles_dug();
  }

  move(id: number, x: number, y: number): boolean {
    return this.sim.cmd_move(id, x, y);
  }

  dig(id: number, tx: number, ty: number): boolean {
    return this.sim.cmd_dig(id, tx, ty);
  }

  tileAt(layer: number, x: number, y: number): number {
    if (x < 0 || y < 0 || x >= this.w || y >= this.h) return 4;
    return (this.tilesCache[layer] as Uint8Array)[y * this.w + x];
  }

  tiles(layer: number): Uint8Array {
    return this.tilesCache[layer] as Uint8Array;
  }

  consumeDirty(layer: number): boolean {
    const d = this.tilesDirtyFlag[layer];
    this.tilesDirtyFlag[layer] = false;
    return d;
  }

  workers(): number[] {
    const out: number[] = [];
    for (const s of this.cur.values()) if (s.kind === 1) out.push(s.id);
    return out.sort((a, b) => a - b);
  }

  queenId(): number | null {
    for (const s of this.cur.values()) if (s.kind === 0) return s.id;
    return null;
  }

  eggCount(): number {
    let n = 0;
    for (const s of this.cur.values()) if (s.kind === 3) n++;
    return n;
  }

  private pollTiles(): void {
    for (const layer of [0, 1]) {
      const fresh = new Uint8Array(
        layer === 0 ? this.sim.tiles_surface() : this.sim.tiles_underground(),
      );
      const old = this.tilesCache[layer];
      if (!old || old.length !== fresh.length) {
        this.tilesCache[layer] = fresh;
        this.tilesDirtyFlag[layer] = true;
        continue;
      }
      let same = true;
      for (let i = 0; i < fresh.length; i++) {
        if (fresh[i] !== old[i]) {
          same = false;
          break;
        }
      }
      if (!same) {
        this.tilesCache[layer] = fresh;
        this.tilesDirtyFlag[layer] = true;
      }
    }
  }

  private pull(): void {
    const raw = this.sim.snapshot();
    this.tickCount = Number(this.sim.tick_count());
    this.dead = this.sim.colony_dead();
    this.food = this.sim.food_store();
    const prev = this.cur;
    const cur = new Map<number, Snap>();
    const n = raw[3];
    for (let i = 0; i < n; i++) {
      const o = 4 + i * 7;
      const s: Snap = {
        id: raw[o],
        kind: raw[o + 1],
        layer: raw[o + 2],
        x: raw[o + 3],
        y: raw[o + 4],
        state: raw[o + 5],
        extra: raw[o + 6],
      };
      if (!prev.has(s.id)) prev.set(s.id, s);
      cur.set(s.id, s);
    }
    this.prev = prev;
    this.cur = cur;
  }
}
