import { WoaSim } from './wasm';
import { decodeSnapshot } from './decode';
import type { AntEnt, Ent } from './decode';

export type { AntEnt, EggEnt, Ent, SourceEnt, SpiderEnt, FoodEnt, Carry, FoodName, ActivityName, IntentName, BlockedName } from './decode';
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
  /** Whole-rules JSON to run under (admin panel restart path). Invalid
   * rules throw — they were validated before the restart; a silent fallback
   * to defaults would hide a tuning mistake. */
  rules?: string;
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
    if (this.foundingMode && opts.rules !== undefined) {
      this.sim = WoaSim.new_founding_rules(
        BigInt(Math.floor(seed)),
        opts.team ?? 0,
        opts.rules,
      );
    } else if (this.foundingMode) {
      this.sim = WoaSim.new_founding(BigInt(Math.floor(seed)), opts.team ?? 0);
    } else {
      this.sim = new WoaSim(BigInt(Math.floor(seed)), workers, clusters);
    }
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

  /** Queen X-menu brood order (F4): caste 0 worker, 1 soldier, 2 honey,
   * 3 medic. Returns '' on success, else the core's refusal reason. */
  brood(queen: number, caste: number): string {
    return this.sim.cmd_brood(queen, caste);
  }

  /** The queen X-menu's data (costs etc.), straight from the rules. */
  broodSpec(): BroodSpecEntry[] {
    if (this.broodSpecCache === null) {
      this.broodSpecCache = JSON.parse(this.sim.brood_spec()) as BroodSpecEntry[];
    }
    return this.broodSpecCache;
  }
  private broodSpecCache: BroodSpecEntry[] | null = null;

  devSetSoil(layer: number, x: number, y: number, soil: number): void {
    this.sim.dev_set_soil(layer, x, y, soil);
    this.refreshSoil();
  }

  /** Colony start phase: 0 flight, 1 grounded, 2 founding, 3 brood, 4 colony. */
  phase(): number {
    return this.sim.phase();
  }

  /** Team color: 0 red, 1 blue. */
  team(): number {
    return this.sim.team();
  }

  /** The designated feeder's id, or null when there is none. */
  feederId(): number | null {
    const id = this.sim.feeder_id();
    return id === 0xffffffff ? null : id;
  }

  /** What the queen requests next (founding, post-nest), or null. */
  queenRequest(): { request: string; hunger: number } | null {
    const q = this.ant(this.queenId() ?? -1);
    if (q === undefined || q.request === null) return null;
    return { request: q.request, hunger: q.hunger };
  }

  devSpawn(kind: string, x: number, y: number): number {
    return this.sim.dev_spawn(kind, x, y);
  }

  // --- rules tuning surface (admin panel; core validates atomically) ---

  /** Current rules + digest, decoded: the panel's draft source of truth. */
  rulesGet(): { digest: string; rules: Record<string, unknown> } {
    return JSON.parse(this.sim.rules_get()) as { digest: string; rules: Record<string, unknown> };
  }

  /** One cached decoded rules read for gameplay consumers (HUD, render) —
   * the same cache-invalidation contract as the other derived caches. */
  private rulesNum(key: string): number {
    if (this.rulesNumCache === null) {
      this.rulesNumCache = this.rulesGet().rules;
    }
    const v = this.rulesNumCache[key];
    if (typeof v !== 'number') throw new Error(`rules: '${key}' is missing or not a number`);
    return v;
  }
  private rulesNumCache: Record<string, unknown> | null = null;

  /** Founding water quest: water units stored on food blocks so far. */
  questWaterTally(): number {
    return this.sim.quest_water_tally();
  }

  /** Founding water quest: units required before the brood can be laid. */
  questWaterNeeded(): number {
    return this.rulesNum('founding_quest_water');
  }

  /** Typed-block storage capacity (underground blocks; pile-dot sizing). */
  foodBlockCap(): number {
    return this.rulesNum('food_block_cap');
  }

  rulesDefault(): { digest: string; rules: Record<string, unknown> } {
    return JSON.parse(this.sim.rules_default()) as { digest: string; rules: Record<string, unknown> };
  }

  /** The panel's field registry (groups, scopes, nested-shape hints). */
  rulesMeta(): RulesMeta {
    if (this.rulesMetaCache === null) {
      this.rulesMetaCache = JSON.parse(this.sim.rules_meta()) as RulesMeta;
    }
    return this.rulesMetaCache;
  }
  private rulesMetaCache: RulesMeta | null = null;

  /** Atomic whole-rules commit; ok, else every validation error. Clears
   * derived caches (brood menu costs, gameplay rules reads) — they come
   * from the rules. */
  rulesSet(json: string): { ok: boolean; digest?: string; errors?: string[] } {
    this.broodSpecCache = null;
    this.rulesNumCache = null;
    return JSON.parse(this.sim.rules_set(json));
  }

  rulesDigest(): string {
    return this.sim.rules_digest();
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

  storeHoneydew(): number {
    return this.sim.store_honeydew();
  }

  devSetHoneydew(n: number): void {
    this.sim.dev_set_honeydew(n);
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

  /** Every non-queen ant id, ascending (the cycling roster). */
  workers(): number[] {
    const out: number[] = [];
    for (const s of this.cur.values()) if (isAnt(s) && s.kind !== 'queen') out.push(s.id);
    return out.sort((a, b) => a - b);
  }

  casteCounts(): { workers: number; soldiers: number; honeys: number; medics: number } {
    let workers = 0;
    let soldiers = 0;
    let honeys = 0;
    let medics = 0;
    for (const s of this.cur.values()) {
      if (s.kind === 'worker') workers++;
      else if (s.kind === 'soldier') soldiers++;
      else if (s.kind === 'honey') honeys++;
      else if (s.kind === 'medic') medics++;
    }
    return { workers, soldiers, honeys, medics };
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
  return (
    s.kind === 'queen' ||
    s.kind === 'worker' ||
    s.kind === 'soldier' ||
    s.kind === 'honey' ||
    s.kind === 'medic'
  );
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
    case 'fallen':
      return 'downed ant';
  }
}

/** One queen X-menu entry, from the core's brood_spec (rules = truth). */
export interface BroodSpecEntry {
  code: number;
  name: string;
  protein: number;
  carbs: number;
  water: number;
  honeydew: number;
  eggTime: number;
  consumesWorker: boolean;
}

/** The admin panel's field registry, from the core's rules_meta(). */
export interface RulesMeta {
  groups: { name: string; fields: string[] }[];
  /** Leaf path → { scope: 'live' | 'new_game', group }. */
  fields: Record<string, { scope: 'live' | 'new_game'; group: string }>;
  unit_fields: string[];
  unit_groups: string[];
  brood_castes: string[];
  brood_fields: string[];
  food_kinds: string[];
}
