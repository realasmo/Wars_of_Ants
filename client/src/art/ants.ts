// Art config for procedural ants — the single source of truth for the look.
// Render code reads these values and nothing else: changing the look means
// editing this file, never the render code (ticket: procedural-ants).
//
// LIVE SET: Camponotus (carpenter ants) — values from the photo intake
// (docs/ant-proportions-intake-camponotus.md, GLM-Vision 2026-09-27; soldier
// row Camponotus-corrected because the soldier photos were Myrmecia).
// Units: thorax length = 1.0 unless the field says otherwise.
//
// Palette notes: "glossy black" reads as near-black brown with a bright warm
// specular, not gray. The queen's photo limbs are near-black (unreadable at
// play zoom) — in-game limbs are lightened for leg visibility. Team 1 keeps
// the caste bicolor PATTERN hue-shifted to steel-blue: caste reads by shape,
// team by hue.

export type Caste = 'worker' | 'soldier' | 'queen';

export interface Palette {
  /** thorax fill (mesosoma) */
  thorax: number;
  /** head fill */
  head: number;
  /** gaster fill — darkest tagma */
  gaster: number;
  /** one subtle highlight per segment */
  highlight: number;
  /** legs + antennae — lighter than the body (bicolor) */
  limbs: number;
  /** mandibles — defaults to limbs when omitted (queen: dark, photo-correct) */
  mandible?: number;
  /** dark outline on every part */
  outline: number;
  /** antenna clubs / small accents */
  accent: number;
}

export interface CasteArt {
  /** total body length (head+thorax+gaster) in world tiles */
  bodyLenTiles: number;
  // tagma proportions, thorax-length units
  headLen: number;
  headWid: number;
  gasterLen: number;
  gasterWid: number;
  /** head silhouette: 'oval' (worker), 'rect' (straight-sided major/gyne) */
  headShape: 'oval' | 'rect';
  /** waist nodes: Camponotus = 1 (petiole only); Carebara had 2 */
  waistNodes: 1 | 2;
  petioleWid: number;
  postpetioleWid: number; // unused when waistNodes = 1
  /** pale transverse bands on the gaster (queen pubescence) */
  gasterBands: boolean;
  /** leg length relative to total body length */
  legRatio: number;
  /** splay angles (deg from body axis): front / mid / rear */
  splay: [number, number, number];
  /** leg attachment x on the thorax, front→rear, in thorax units */
  legAttach: [number, number, number];
  /** femur share of the full leg length (rest is tibia+tarsus) */
  femurFrac: number;
  /** limb stroke width in thorax units (tune for play zoom) */
  limbWid: number;
  /** planted-foot dot radius in thorax units */
  footR: number;
  /** mandible length relative to head width */
  mandibleLen: number;
  /** antenna length relative to body length */
  antennaLen: number;
  /** antenna club share of antenna length (pale 2-segment club) */
  clubFrac: number;
  /** 'stubs' = small wing sprites at rest; 'scars' = marks baked into the
   *  thorax (dealate gyne); flight wings exist either way */
  wings: 'none' | 'stubs' | 'scars';
  /** legs pull toward the body by this factor while airborne */
  tuckFrac: number;
  gait: {
    /** step trigger: planted-foot drift from rest, in body lengths */
    strideTrigger: number;
    /** landing point lead ahead of rest (× step urgency 0..1), body lengths */
    strideLead: number;
    /** a swing must land within this even if the ant halts mid-swing (s) */
    swingMaxSec: number;
    /** fake lift while swinging: tibia stretch + foot-dot shrink (0..1) */
    lift: number;
    /** in-group ripple: next leg starts after this fraction of a swing */
    stagger: number;
    /** swing overshoot past the landing point, fraction of stride */
    overshoot: number;
    /** lateral body sway amplitude, body lengths (per step) */
    sway: number;
    /** follow-through lag: head / gaster (higher = more trail on turns) */
    trailHead: number;
    trailGaster: number;
  };
  idle: {
    /** antenna wave duration range (s) */
    waveDur: [number, number];
    /** pause between waves (s) */
    waveGap: [number, number];
    /** antenna wave amplitude (rad) */
    waveAmp: number;
    /** rare head-turn interval range (s) */
    headEvery: [number, number];
    headAmp: number;
    /** gaster "breathing" scale pulse */
    breatheAmp: number;
    breatheSec: number;
  };
}

export const CASTES: Record<Caste, CasteArt> = {
  // minor worker: H:T:G len 0.4:1:1.1, wid 1.2:1:1.7 — long legs + long
  // scapes, active forager (the intake's strongest caste identifier)
  worker: {
    bodyLenTiles: 1.0,
    headLen: 0.4,
    headWid: 1.2,
    gasterLen: 1.1,
    gasterWid: 1.7,
    headShape: 'oval',
    waistNodes: 1,
    petioleWid: 0.5,
    postpetioleWid: 0.5,
    gasterBands: false,
    legRatio: 0.85, // photo says ~1.0; tempered for 2-tile tunnels
    splay: [40, 90, 30],
    legAttach: [0.32, 0.0, -0.38],
    femurFrac: 0.45,
    limbWid: 0.22,
    footR: 0.1,
    mandibleLen: 0.2,
    antennaLen: 0.5,
    clubFrac: 0.35,
    wings: 'none',
    tuckFrac: 0.45,
    gait: {
      strideTrigger: 0.34,
      strideLead: 0.6,
      swingMaxSec: 0.11,
      lift: 0.55,
      stagger: 0.13,
      overshoot: 0.1,
      sway: 0.012,
      trailHead: 0.22,
      trailGaster: 0.28,
    },
    idle: {
      waveDur: [0.7, 1.3],
      waveGap: [0.8, 3.5],
      waveAmp: 0.4,
      headEvery: [6, 14],
      headAmp: 0.14,
      breatheAmp: 0.015,
      breatheSec: 1.6,
    },
  },
  // soldier (major): H:T:G len 0.9:1:0.9, wid 1.5:1:1.4 — rectangular
  // red-orange head, short defensive legs, glossy black gaster
  soldier: {
    bodyLenTiles: 1.7,
    headLen: 0.9,
    headWid: 1.5,
    gasterLen: 0.9,
    gasterWid: 1.4,
    headShape: 'rect',
    waistNodes: 1,
    petioleWid: 0.5,
    postpetioleWid: 0.5,
    gasterBands: false,
    legRatio: 0.55,
    splay: [50, 75, 40],
    legAttach: [0.3, 0.0, -0.42],
    femurFrac: 0.48,
    limbWid: 0.26,
    footR: 0.11,
    mandibleLen: 0.5,
    antennaLen: 0.35,
    clubFrac: 0.38,
    wings: 'none',
    tuckFrac: 0.5,
    gait: {
      strideTrigger: 0.26,
      strideLead: 0.5,
      swingMaxSec: 0.18,
      lift: 0.5,
      stagger: 0.12,
      overshoot: 0.1,
      sway: 0.02,
      trailHead: 0.3,
      trailGaster: 0.4,
    },
    idle: {
      waveDur: [0.9, 1.6],
      waveGap: [1.2, 4.5],
      waveAmp: 0.35,
      headEvery: [8, 18],
      headAmp: 0.12,
      breatheAmp: 0.018,
      breatheSec: 2.0,
    },
  },
  // queen (gyne): orange head, LARGE black muscular mesosoma with wing
  // scars, gaster-dominant black banded gaster
  queen: {
    bodyLenTiles: 2.4,
    headLen: 0.8,
    headWid: 1.4,
    gasterLen: 1.15,
    gasterWid: 1.55,
    headShape: 'rect',
    waistNodes: 1,
    petioleWid: 0.4,
    postpetioleWid: 0.4,
    gasterBands: true,
    legRatio: 0.55,
    splay: [45, 80, 35],
    legAttach: [0.3, 0.0, -0.4],
    femurFrac: 0.46,
    limbWid: 0.24,
    footR: 0.1,
    mandibleLen: 0.25,
    antennaLen: 0.5,
    clubFrac: 0.36,
    wings: 'scars',
    tuckFrac: 0.42,
    gait: {
      strideTrigger: 0.24,
      strideLead: 0.45,
      swingMaxSec: 0.2,
      lift: 0.45,
      stagger: 0.11,
      overshoot: 0.08,
      sway: 0.03,
      trailHead: 0.2,
      trailGaster: 0.45,
    },
    idle: {
      waveDur: [1.0, 1.8],
      waveGap: [1.5, 5],
      waveAmp: 0.36,
      headEvery: [8, 16],
      headAmp: 0.12,
      breatheAmp: 0.05,
      breatheSec: 2.4,
    },
  },
};

/** Team palettes: team 0 natural Camponotus bicolor, team 1 the same caste
 * patterns hue-shifted to steel-blue. Caste reads by shape, team by hue. */
export const TEAM_PALETTES: Record<number, Record<Caste, Palette>> = {
  0: {
    worker: {
      thorax: 0xb4692f, head: 0x2a1c14, gaster: 0x241811, highlight: 0xd9a06b,
      limbs: 0xd08a4e, outline: 0x120a06, accent: 0xc9a884,
    },
    soldier: {
      thorax: 0xb04a26, head: 0xc1441e, gaster: 0x1e1410, highlight: 0xe8946a,
      limbs: 0xc25e32, outline: 0x120a06, accent: 0xd8bc9e,
    },
    queen: {
      thorax: 0x1c1815, head: 0xb14a26, gaster: 0x1e1410, highlight: 0x8d867b,
      limbs: 0x6a5a4a, mandible: 0x3a2c22, outline: 0x0c0a09, accent: 0x9a8f7e,
    },
  },
  1: {
    worker: {
      thorax: 0x4e6a8e, head: 0x1e2634, gaster: 0x1a222e, highlight: 0x8fb0d4,
      limbs: 0x7a9ac0, outline: 0x0a0e14, accent: 0xb8cce0,
    },
    soldier: {
      thorax: 0x46628c, head: 0x5070a0, gaster: 0x161c28, highlight: 0x8fb0d4,
      limbs: 0x6a8ab4, outline: 0x0a0e14, accent: 0xb0c0d8,
    },
    queen: {
      thorax: 0x1a2028, head: 0x5070a0, gaster: 0x161c28, highlight: 0x708090,
      limbs: 0x56617a, mandible: 0x2c3038, outline: 0x080c12, accent: 0xa8b8cc,
    },
  },
};

/** Carried-unit colors (matches the placeholder visuals' resource colors). */
export const FOOD_COLORS: Record<string, number> = {
  green: 0x3fa34d,
  super: 0x4a7fd9,
  protein: 0xc05a5a,
  carbs: 0xd4a832,
  water: 0x4a9fd9,
};

/** Mandible open angle (rad) per carried-item width class (dev guide §5:
 * partially open, adjusted to the item's width). */
export const CARRY_MANDIBLE_OPEN: Record<string, number> = {
  dirt: 0.16,
  food: 0.2,
  egg: 0.26,
};

/** Bake resolution: texture pixels per thorax unit. 128 keeps parts crisp at
 * max zoom (4×) where a thorax unit spans ~55 screen px. */
export const BAKE_PPU = 128;
