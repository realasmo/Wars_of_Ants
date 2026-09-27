// Art config for procedural ants — the single source of truth for the look.
// Render code reads these values and nothing else: changing the look means
// editing this file, never the render code (ticket: procedural-ants).
//
// Proportion values come from the photo intake (docs/ant-proportions-intake.md,
// Carebara diversa, GLM-Vision 2026-09-27). Units: thorax length = 1.0 unless
// the field says otherwise. The queen is extrapolated (no queen photos): the
// intake's starting point is the soldier's gaster emphasis pushed further with
// a worker-sized head, plus wing stubs.
//
// Palette hexes are re-tuned for the game's dark underground background —
// the intake's museum-flash hues are hue references only. The one contrast
// rule the intake marks as reliable: dark gaster vs lighter mesosoma, and
// bicolor (lighter) limbs on every caste.

export type Caste = 'worker' | 'soldier' | 'queen';

export interface Palette {
  /** thorax fill (mesosoma) — the light tagma */
  thorax: number;
  /** head fill — darker than thorax, glossy */
  head: number;
  /** gaster fill — darkest tagma */
  gaster: number;
  /** one subtle highlight per segment */
  highlight: number;
  /** legs + mandibles — distinctly lighter than the body (bicolor) */
  limbs: number;
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
  headWid: number; // note: worker/soldier heads are WIDER than the thorax
  gasterLen: number;
  gasterWid: number;
  petioleWid: number;
  postpetioleWid: number;
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
  wings: 'none' | 'stubs';
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
  // worker (minor): H:T:G len 0.65:1:0.55, wid 1.4:1:1.2 — brisk, small steps
  worker: {
    bodyLenTiles: 1.0,
    headLen: 0.65,
    headWid: 1.4,
    gasterLen: 0.62,
    gasterWid: 1.2,
    petioleWid: 0.3,
    postpetioleWid: 0.5,
    legRatio: 0.5,
    splay: [45, 85, 45],
    legAttach: [0.32, 0.0, -0.38],
    femurFrac: 0.46,
    limbWid: 0.24,
    footR: 0.1,
    mandibleLen: 0.4,
    antennaLen: 0.34,
    clubFrac: 0.38,
    wings: 'none',
    tuckFrac: 0.45,
    gait: {
      strideTrigger: 0.32,
      strideLead: 0.55,
      swingMaxSec: 0.12,
      lift: 0.55,
      stagger: 0.13,
      overshoot: 0.1,
      sway: 0.012,
      trailHead: 0.22,
      trailGaster: 0.3,
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
  // soldier (supermajor): H:T:G len 1.6:1:1.6, wid 3:1:3 — heavy, slow, wide splay
  soldier: {
    bodyLenTiles: 2.0,
    headLen: 1.6,
    headWid: 3.0,
    gasterLen: 1.8,
    gasterWid: 2.2,
    petioleWid: 0.55,
    postpetioleWid: 0.7,
    legRatio: 0.5,
    splay: [70, 55, 45],
    legAttach: [0.3, 0.0, -0.42],
    femurFrac: 0.48,
    limbWid: 0.26,
    footR: 0.11,
    mandibleLen: 0.35,
    antennaLen: 0.35,
    clubFrac: 0.4,
    wings: 'none',
    tuckFrac: 0.5,
    gait: {
      strideTrigger: 0.26,
      strideLead: 0.5,
      swingMaxSec: 0.17,
      lift: 0.5,
      stagger: 0.12,
      overshoot: 0.1,
      sway: 0.02,
      trailHead: 0.28,
      trailGaster: 0.38,
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
  // queen: extrapolated — the intake's wide physogastric guess read as a
  // vertical blob in QC (rounds 3–4), so the gaster is elongated instead
  // (still the dominant tagma) + worker-sized head + wing stubs
  queen: {
    bodyLenTiles: 2.7,
    headLen: 0.7,
    headWid: 1.5,
    gasterLen: 2.5,
    gasterWid: 2.5,
    petioleWid: 0.35,
    postpetioleWid: 0.6,
    legRatio: 0.48,
    splay: [62, 78, 58],
    legAttach: [0.3, 0.0, -0.4],
    femurFrac: 0.46,
    limbWid: 0.24,
    footR: 0.1,
    mandibleLen: 0.3,
    antennaLen: 0.4,
    clubFrac: 0.36,
    wings: 'stubs',
    tuckFrac: 0.42,
    gait: {
      strideTrigger: 0.24,
      strideLead: 0.45,
      swingMaxSec: 0.2,
      lift: 0.45,
      stagger: 0.11,
      overshoot: 0.08,
      sway: 0.028,
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

/** Team palettes: team 0 red/rust (natural Carebara hues), team 1 steel-blue.
 * Same silhouettes, color only — caste reads by shape, team by hue. */
export const TEAM_PALETTES: Record<number, Record<Caste, Palette>> = {
  0: {
    worker: {
      thorax: 0xa06a2e, head: 0x4e2e13, gaster: 0x38210f, highlight: 0xe8c884,
      limbs: 0xd8a45e, outline: 0x1c1006, accent: 0xf0e0b0,
    },
    soldier: {
      thorax: 0x7a2d1c, head: 0x4e1a10, gaster: 0x33120a, highlight: 0xd8a080,
      limbs: 0xb06a30, outline: 0x160a06, accent: 0xe8d0b8,
    },
    queen: {
      thorax: 0x8e3a24, head: 0x5a2014, gaster: 0x46180c, highlight: 0xe0b090,
      limbs: 0xc08848, outline: 0x180c06, accent: 0xf0e0c0,
    },
  },
  1: {
    worker: {
      thorax: 0x5a7195, head: 0x26344a, gaster: 0x1e2a3c, highlight: 0xb8cce4,
      limbs: 0x8aa0bc, outline: 0x0c1220, accent: 0xd8e4f0,
    },
    soldier: {
      thorax: 0x3e5478, head: 0x22304a, gaster: 0x182238, highlight: 0x9cb4d4,
      limbs: 0x7490b0, outline: 0x080e1a, accent: 0xc8d8ec,
    },
    queen: {
      thorax: 0x46608a, head: 0x243455, gaster: 0x1a2740, highlight: 0xa8c0dc,
      limbs: 0x7c94b8, outline: 0x0a101c, accent: 0xd0e0f0,
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

/** Bake resolution: texture pixels per thorax unit. 128 keeps parts crisp at
 * max zoom (4×) where a thorax unit spans ~55 screen px. */
export const BAKE_PPU = 128;
