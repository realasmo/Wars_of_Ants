// The typed snapshot boundary (Option C). The core packs entities into a
// flat positional array (see core/src/sim/snapshot.rs + lib.rs); this module
// is the ONLY place that translates positions into names. Game logic and
// rendering consume the named shapes below and never see a wire code.
//
// The core exports its wire codes as JSON via snapshot_spec(); assertSpec()
// pins this decoder to the core's at boot — a mismatch fails loudly instead
// of rendering garbage (the `aux >= 2.5` bug class).

import type { WoaSim } from './wasm';

// --- wire codes (must match core/src/sim/snapshot.rs; asserted at boot) ---

const KINDS = {
  queen: 0,
  worker: 1,
  soldier: 2,
  egg: 3,
  spider: 4,
  food: 5,
  source: 6,
  collectible: 7,
} as const;

const ACTIVITY = {
  idle: 0,
  moving: 1,
  digging: 2,
  fighting: 3,
  flying: 4,
  harvesting: 5,
} as const;

const CARRY = {
  none: 0,
  dirt: 1,
  egg: 2,
  food: 3,
  wood: 4,
  wool: 5,
} as const;

const FOOD = {
  green: 0,
  super: 1,
  protein: 2,
  carbs: 3,
  water: 4,
} as const;

// --- decoded shapes ---

export type FoodName = keyof typeof FOOD; // 'green' | 'super' | 'protein' | 'carbs' | 'water'
export type ActivityName = keyof typeof ACTIVITY;
export type AntKindName = 'queen' | 'worker' | 'soldier';

export type Carry =
  | { t: 'none' }
  | { t: 'dirt'; blocks: number }
  | { t: 'egg' }
  | { t: 'food'; food: FoodName }
  | { t: 'wood' }
  | { t: 'wool' };

interface Base {
  id: number;
  /** 0 surface, 1 underground. */
  layer: number;
  x: number;
  y: number;
}

export interface AntEnt extends Base {
  kind: AntKindName;
  activity: ActivityName;
  /** HP fraction 0..1. */
  hp: number;
  carry: Carry;
  /** Starvation progress 0..1 (queen only). */
  hunger: number;
  /** Squad leader this ant follows (X-menu), or null. */
  following: number | null;
}

/** Dropped or banked food pile. */
export interface FoodEnt extends Base {
  kind: 'food';
  amount: number;
  food: FoodName;
  /** Remaining spoil fraction 0..1, or null when not spoiling (pantry). */
  spoil: number | null;
}

/** Finite map source (moss..caterpillar). */
export interface SourceEnt extends Base {
  kind: 'source';
  amount: number;
  food: FoodName;
  /** Visual type 1..6. */
  src: number;
}

export interface EggEnt extends Base {
  kind: 'egg';
  /** Remaining incubation fraction: 1 fresh → 0 ready. */
  hatchLeft: number;
  caste: 'worker' | 'soldier';
  carried: boolean;
}

/** Wet wood / dry wool collectible (nest-building, F2). */
export interface CollectibleEnt extends Base {
  kind: 'collectible';
  variant: 'wood' | 'wool';
}

export interface SpiderEnt extends Base {
  kind: 'spider';
  /** HP fraction 0..1. */
  hp: number;
  hunting: boolean;
}

export type Ent = AntEnt | FoodEnt | SourceEnt | EggEnt | SpiderEnt | CollectibleEnt;

export interface Snapshot {
  tick: number;
  dead: boolean;
  carbs: number;
  ents: Ent[];
}

const ACTIVITY_NAMES: Record<number, ActivityName> = Object.fromEntries(
  Object.entries(ACTIVITY).map(([k, v]) => [v, k as ActivityName]),
);
const FOOD_NAMES: Record<number, FoodName> = Object.fromEntries(
  Object.entries(FOOD).map(([k, v]) => [v, k as FoodName]),
);
const KIND_BY_CODE: Record<number, AntKindName> = {
  [KINDS.queen]: 'queen',
  [KINDS.worker]: 'worker',
  [KINDS.soldier]: 'soldier',
};

/** Decode the flat transport into named entities. Throws on unknown codes —
 * a stale decoder against a newer core should crash loudly, not guess. */
export function decodeSnapshot(sim: WoaSim): Snapshot {
  const raw = sim.snapshot();
  const n = raw[3];
  const ents: Ent[] = [];
  for (let i = 0; i < n; i++) {
    const o = 4 + i * 11;
    const id = raw[o];
    const kindCode = raw[o + 1];
    const layer = raw[o + 2];
    const x = raw[o + 3];
    const y = raw[o + 4];
    const p0 = raw[o + 5];
    const p1 = raw[o + 6];
    const p2 = raw[o + 7];
    const p3 = raw[o + 8];
    const p4 = raw[o + 9];
    const p5 = raw[o + 10];
    if (kindCode === KINDS.queen || kindCode === KINDS.worker || kindCode === KINDS.soldier) {
      const carryTag = p2;
      const carry: Carry =
        carryTag === CARRY.none
          ? { t: 'none' }
          : carryTag === CARRY.dirt
            ? { t: 'dirt', blocks: p3 }
            : carryTag === CARRY.egg
              ? { t: 'egg' }
              : { t: 'food', food: foodName(p3) };
      ents.push({
        id,
        kind: KIND_BY_CODE[kindCode],
        layer,
        x,
        y,
        activity: activityName(p0),
        hp: p1,
        carry,
        hunger: p4,
        following: p5 > 0.5 ? p5 - 1 : null,
      });
    } else if (kindCode === KINDS.food) {
      ents.push({
        id,
        kind: 'food',
        layer,
        x,
        y,
        amount: p0,
        food: foodName(p1),
        spoil: p2 < 0 ? null : p2,
      });
    } else if (kindCode === KINDS.source) {
      ents.push({
        id,
        kind: 'source',
        layer,
        x,
        y,
        amount: p0,
        food: foodName(p1),
        src: p2,
      });
    } else if (kindCode === KINDS.egg) {
      ents.push({
        id,
        kind: 'egg',
        layer,
        x,
        y,
        hatchLeft: p0,
        caste: p1 > 0.5 ? 'soldier' : 'worker',
        carried: p2 > 0.5,
      });
    } else if (kindCode === KINDS.collectible) {
      ents.push({
        id,
        kind: 'collectible',
        layer,
        x,
        y,
        variant: p0 > 0.5 ? 'wool' : 'wood',
      });
    } else if (kindCode === KINDS.spider) {
      ents.push({
        id,
        kind: 'spider',
        layer,
        x,
        y,
        hp: p0,
        hunting: p1 > 0.5,
      });
    } else {
      throw new Error(`decodeSnapshot: unknown entity kind code ${kindCode} (id ${id})`);
    }
  }
  return { tick: raw[0], dead: raw[1] > 0.5, carbs: raw[2], ents };
}

function activityName(code: number): ActivityName {
  const n = ACTIVITY_NAMES[code];
  if (n === undefined) throw new Error(`decodeSnapshot: unknown activity code ${code}`);
  return n;
}

function foodName(code: number): FoodName {
  const n = FOOD_NAMES[code];
  if (n === undefined) throw new Error(`decodeSnapshot: unknown food code ${code}`);
  return n;
}

/** Boot-time assertion: the core's wire spec must equal this decoder's
 * constants. Call once before the first snapshot is decoded. */
export function assertWireSpec(specJson: string): void {
  let spec: Record<string, unknown>;
  try {
    spec = JSON.parse(specJson);
  } catch (e) {
    throw new Error(`assertWireSpec: core spec is not JSON: ${specJson.slice(0, 80)}`);
  }
  const groups: Array<[string, Record<string, number>]> = [
    ['kinds', KINDS],
    ['activity', ACTIVITY],
    ['carry', CARRY],
    ['food', FOOD],
  ];
  for (const [name, local] of groups) {
    const remote = spec[name] as Record<string, number> | undefined;
    if (remote === undefined) throw new Error(`assertWireSpec: core spec has no "${name}" group`);
    for (const [key, code] of Object.entries(local)) {
      if (remote[key] !== code) {
        throw new Error(
          `assertWireSpec: ${name}.${key} is ${remote[key]} in core but ${code} in the client decoder — rebuild the WASM core or update decode.ts`,
        );
      }
    }
    const extra = Object.keys(remote).filter((k) => !(k in local));
    if (extra.length > 0) {
      throw new Error(`assertWireSpec: core added ${name} codes the decoder lacks: ${extra.join(', ')}`);
    }
  }
}
