import { WoaSim } from './wasm';
import { decodeSnapshot } from './decode';
import type { AntEnt, Ent } from './decode';

export type { AntEnt, EggEnt, Ent, SourceEnt, SpiderEnt, FoodEnt, Carry, FoodName, ActivityName } from './decode';
export type Snap = Ent;

export const TPS = 20;

/** Interpolated position between two snapshots, for smooth rendering between
 * fixed ticks. Snaps instead of lerps when the prev→cur jump can't be motion —
 * a layer change (entrance crossing teleports to the entrance tile) or a hop
 * larger than 2 tiles (max real per-tick motion is ~0.25 tiles in flight). */
export function lerpPos(p: Ent, s: Ent, t: number): { x: number; y: number } {
  if (p.layer !== s.layer || (s.x - p.x) ** 2 + (s.y - p.y) ** 2 > 4) return { x: s.x, y: s.y };
  return { x: p.x + (s.x - p.x) * t, y: p.y + (s.y - p.y) * t };
}

export interface SimOptions {
  /** Founding start: lone flying queen, no workers, no nest yet. */
  founding?: boolean;
  /** Team color: 0 red, 1 blue (founding mode only). */
  team?: number;
}

/** Colony start phases, mirroring the core enum. */
export const PHASE_FLIGHT = 0;
export const PHASE_GROUNDED = 1;
export const PHASE_FOUNDING = 2;
export const PHASE_BROOD = 3;
export const PHASE_COLONY = 4;

export class Sim {
  private sim: WoaSim;
  readonly w: number;
  readonly h: number;
  /** Nest hole; null until the founding queen creates the nest. Polled per tick. */
  entrance: [number, number] | null = null;
  readonly foundingMode: boolean;
  prev = new Map<number, Ent>();
  cur = new Map<number, Ent>();
  tickCount = 0;
  dead = false;
  carbs = 0;
  private tilesCache: (Uint8Array | null)[] = [null, null];
  private tilesDirtyFlag = [true, true];
  private tilesEpoch = -1n;
  private soilEpoch = -1n;
  /** Per-tile soil quality (0 none, 1 orange, 2 silver) per layer. */
  soil: Uint8Array[] = [new Uint8Array(0), new Uint8Array(0)];

  constructor(seed: number, workers = 3, clusters = 6, opts: SimOptions = {}) {
    this.foundingMode = opts.founding ?? false;
    this.sim = this.foundingMode
      ? WoaSim.new_founding(BigInt(Math.floor(seed)), opts.team ?? 0)
      : new WoaSim(BigInt(Math.floor(seed)), workers, clusters);
    const dims = this.sim.dims();
    this.w = dims[0];
    this.h = dims[1];
    const e = this.sim.entrance();
    this.entrance = e.length === 2 ? [e[0], e[1]] : null;
    this.pull();
    this.pollTiles();
    this.refreshSoil();
  }

  /** Re-pull soil grids when the core bumped its soil epoch (founding grants,
   * dev painting); cheap no-op otherwise. */
  refreshSoil(): void {
    const epoch = this.sim.soil_epoch();
    if (epoch === this.soilEpoch) return;
    this.soilEpoch = epoch;
    this.soil[0] = new Uint8Array(this.sim.soil_surface());
    this.soil[1] = new Uint8Array(this.sim.soil_underground());
  }

  soilAt(layer: number, x: number, y: number): number {
    if (x < 0 || y < 0 || x >= this.w || y >= this.h) return 0;
    return this.soil[layer][y * this.w + x];
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

  attack(id: number, target: number): boolean {
    return this.sim.cmd_attack(id, target);
  }

  useEntrance(id: number): boolean {
    return this.sim.cmd_entrance(id);
  }

  /** Flying queen: fly to (x, y) and land there. */
  land(id: number, x: number, y: number): boolean {
    return this.sim.cmd_land(id, x, y);
  }

  /** Grounded queen: walk to (x, y) and found the nest at that tile. */
  found(id: number, x: number, y: number): boolean {
    return this.sim.cmd_found(id, x, y);
  }

  drop(id: number, tx: number, ty: number): boolean {
    return this.sim.cmd_drop(id, tx, ty);
  }

  /** Squad control (X-menu): mode 0 all-in-sight, 1 one, 2 soldiers, 3 release. */
  follow(leader: number, mode: number): boolean {
    return this.sim.cmd_follow(leader, mode);
  }

  pickEgg(id: number, egg: number): boolean {
    return this.sim.cmd_pick_egg(id, egg);
  }

  devSetSoil(layer: number, x: number, y: number, soil: number): void {
    this.sim.dev_set_soil(layer, x, y, soil);
    this.refreshSoil();
  }

  /** Colony start phase: 0 flight, 1 grounded, 2 founding, 3 brood, 4 colony. */
  phase(): number {
    return this.sim.phase();
  }

  /** Seconds left in the founding excavation window (0 otherwise). */
  phaseTime(): number {
    return this.sim.phase_time();
  }

  /** Team color: 0 red, 1 blue. */
  team(): number {
    return this.sim.team();
  }

  devSpawn(kind: string, x: number, y: number): number {
    return this.sim.dev_spawn(kind, x, y);
  }

  devSetFood(n: number): void {
    this.sim.dev_set_food(n);
  }

  devSetSuper(n: number): void {
    this.sim.dev_set_super(n);
  }

  devKill(id: number): boolean {
    return this.sim.dev_kill(id);
  }

  storeCarbs(): number {
    return this.sim.store_carbs();
  }

  storeProtein(): number {
    return this.sim.store_protein();
  }

  storeWater(): number {
    return this.sim.store_water();
  }

  devSetWater(n: number): void {
    this.sim.dev_set_water(n);
  }

  canonical(): string {
    return this.sim.canonical();
  }

  /** Retained game events, oldest first (capped, observational only). */
  eventLines(): string[] {
    return this.sim.events();
  }

  eventTotal(): number {
    return Number(this.sim.event_total());
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

  /** Worker + soldier ids, ascending. */
  workers(): number[] {
    const out: number[] = [];
    for (const s of this.cur.values())
      if (s.kind === 'worker' || s.kind === 'soldier') out.push(s.id);
    return out.sort((a, b) => a - b);
  }

  casteCounts(): { workers: number; soldiers: number } {
    let workers = 0;
    let soldiers = 0;
    for (const s of this.cur.values()) {
      if (s.kind === 'worker') workers++;
      else if (s.kind === 'soldier') soldiers++;
    }
    return { workers, soldiers };
  }

  queenId(): number | null {
    for (const s of this.cur.values()) if (s.kind === 'queen') return s.id;
    return null;
  }

  ant(id: number): AntEnt | undefined {
    const s = this.cur.get(id);
    return s !== undefined && isAnt(s) ? s : undefined;
  }

  eggCount(): number {
    let n = 0;
    for (const s of this.cur.values()) if (s.kind === 'egg') n++;
    return n;
  }

  /** Re-pull tile grids only when the core bumped its epoch (dig, carve,
   * refill) — replaces the old per-tick 2×9KB copy + byte-compare. */
  private pollTiles(): void {
    const epoch = this.sim.tiles_epoch();
    if (epoch === this.tilesEpoch) return;
    this.tilesEpoch = epoch;
    this.tilesCache[0] = new Uint8Array(this.sim.tiles_surface());
    this.tilesCache[1] = new Uint8Array(this.sim.tiles_underground());
    this.tilesDirtyFlag = [true, true];
  }

  private pull(): void {
    const snap = decodeSnapshot(this.sim);
    this.tickCount = snap.tick;
    this.dead = snap.dead;
    this.carbs = snap.carbs;
    const e = this.sim.entrance();
    this.entrance = e.length === 2 ? [e[0], e[1]] : null;
    const prev = this.cur;
    const cur = new Map<number, Ent>();
    for (const s of snap.ents) {
      if (!prev.has(s.id)) prev.set(s.id, s);
      cur.set(s.id, s);
    }
    this.prev = prev;
    this.cur = cur;
  }
}

export function isAnt(s: Ent): s is AntEnt {
  return s.kind === 'queen' || s.kind === 'worker' || s.kind === 'soldier';
}

/** Short carry description for logs/console ("dirt×2", "egg", "protein"). */
export function carryLabel(c: AntEnt['carry']): string {
  switch (c.t) {
    case 'none':
      return 'none';
    case 'dirt':
      return `dirt×${c.blocks}`;
    case 'egg':
      return 'egg';
    case 'food':
      return c.food;
    case 'wood':
      return 'wood';
    case 'wool':
      return 'wool';
  }
}
