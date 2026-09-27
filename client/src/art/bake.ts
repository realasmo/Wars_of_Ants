// Part bakery: draws every ant part once (per caste × team) into a
// RenderTexture. Per-ant rendering is sprites + transforms only — no
// per-frame Graphics rebuilds (ticket constraint).
//
// Shapes are authored in body space (thorax units, +X forward, +Y right)
// with the part's PIVOT at local (0,0); `u()` converts to texture pixels
// (Graphics takes pixel coordinates). The texture is auto-framed from the
// drawn bounds and the anchor computed so the sprite's (0,0) lands on the
// pivot.

import { Graphics, Rectangle, Texture, type Renderer as PixiRenderer } from 'pixi.js';
import { BAKE_PPU, CASTES, FOOD_COLORS, TEAM_PALETTES, type Caste, type Palette } from './ants';

const P = BAKE_PPU;
/** body-space units → texture px */
const u = (v: number): number => v * P;

export interface BakedPart {
  texture: Texture;
  ax: number;
  ay: number;
  /** body-space length along +X from the pivot (capsules/fangs) */
  len: number;
}

export interface AntParts {
  thorax: BakedPart;
  head: BakedPart;
  gaster: BakedPart;
  femur: BakedPart;
  tibia: BakedPart;
  mandible: BakedPart;
  scape: BakedPart;
  club: BakedPart;
  wingStub: BakedPart | null;
  wing: BakedPart;
  shadow: BakedPart;
}

function bake(renderer: PixiRenderer, draw: (g: Graphics) => void, len = 0): BakedPart {
  const g = new Graphics();
  draw(g);
  const b = g.getBounds();
  const pad = 3; // px, keeps outline strokes from clipping
  const frame = new Rectangle(b.x - pad, b.y - pad, b.width + pad * 2, b.height + pad * 2);
  const texture = renderer.generateTexture({ target: g, resolution: 1, frame });
  g.destroy();
  return { texture, ax: -frame.x / frame.width, ay: -frame.y / frame.height, len };
}

/** capsule along +X from (0,0) to (len,0) [units], widths w0→w1, coxa blob. */
function capsule(g: Graphics, len: number, w0: number, w1: number, fill: number, outline: number, coxa = 0) {
  len = u(len);
  w0 = u(w0);
  w1 = u(w1);
  coxa = u(coxa);
  const ow = P * 0.028;
  if (coxa > 0) g.circle(0, 0, coxa).fill(fill).stroke({ width: ow, color: outline });
  g.moveTo(0, -w0 / 2)
    .lineTo(len, -w1 / 2)
    .arc(len, 0, w1 / 2, -Math.PI / 2, Math.PI / 2)
    .lineTo(0, w0 / 2)
    .arc(0, 0, w0 / 2, Math.PI / 2, (3 * Math.PI) / 2)
    .closePath()
    .fill(fill)
    .stroke({ width: ow, color: outline });
}

const cache = new Map<string, AntParts>();

export function antParts(renderer: PixiRenderer, caste: Caste, team: number): AntParts {
  const key = `${caste}:${team}`;
  const hit = cache.get(key);
  if (hit) return hit;
  const parts = bakeParts(renderer, caste, (TEAM_PALETTES[team] ?? TEAM_PALETTES[0])[caste]);
  cache.set(key, parts);
  return parts;
}

function bakeParts(renderer: PixiRenderer, caste: Caste, pal: Palette): AntParts {
  const art = CASTES[caste];
  const ow = P * 0.035;

  const bodyUnits = art.headLen + 1 + art.gasterLen;
  const legLen = art.legRatio * bodyUnits;
  const femurLen = legLen * art.femurFrac;
  const tibiaLen = legLen * (1 - art.femurFrac);
  const lw = art.limbWid;

  // --- thorax: pronotum + mesonotum (+ propodeal spines) + the two-node
  //     waist (rigid relative to the thorax — one quad, one draw) ---
  const thorax = bake(renderer, (g) => {
    // petiole + postpetiole behind the thorax (two nodes — never one blob)
    g.circle(u(-0.52), 0, u(art.petioleWid / 2 + 0.03)).fill(pal.head).stroke({ width: ow, color: pal.outline });
    g.ellipse(u(-0.34), 0, u(0.13), u(art.postpetioleWid / 2 + 0.02)).fill(pal.head).stroke({ width: ow, color: pal.outline });
    if (art.wings === 'none') {
      for (const s of [-1, 1]) {
        g.moveTo(u(-0.42), u(s * 0.3)).lineTo(u(-0.66), u(s * 0.44)).lineTo(u(-0.4), u(s * 0.42)).closePath()
          .fill(pal.thorax).stroke({ width: ow, color: pal.outline });
      }
    }
    g.ellipse(u(-0.2), 0, u(0.32), u(0.44)).fill(pal.thorax).stroke({ width: ow, color: pal.outline });
    g.ellipse(u(0.16), 0, u(0.36), u(0.5)).fill(pal.thorax).stroke({ width: ow, color: pal.outline });
    g.ellipse(u(-0.16), u(-0.16), u(0.16), u(0.1)).fill({ color: pal.highlight, alpha: 0.3 });
    g.ellipse(u(0.2), u(-0.2), u(0.18), u(0.12)).fill({ color: pal.highlight, alpha: 0.3 });
  });

  // --- head: pivot at the neck (rear center); cordate lobes on the soldier ---
  const hw = art.headWid;
  const hl = art.headLen;
  const hc = 0.08 + hl / 2;
  const head = bake(renderer, (g) => {
    if (caste === 'soldier') {
      for (const s of [-1, 1]) {
        g.circle(u(hc - hl * 0.4), u(s * hw * 0.32), u(hw * 0.3)).fill(pal.head).stroke({ width: ow, color: pal.outline });
      }
    }
    g.ellipse(u(hc), 0, u(hl / 2 + 0.04), u(hw / 2)).fill(pal.head).stroke({ width: ow, color: pal.outline });
    g.moveTo(u(hc + hl / 2 - 0.06), 0).lineTo(u(hc - hl * 0.25), 0)
      .stroke({ width: P * 0.022, color: pal.gaster, alpha: 0.6 });
    g.circle(u(hc + hl * 0.08), u(-hw * 0.36), u(0.05)).fill(pal.outline);
    g.circle(u(hc + hl * 0.08), u(hw * 0.36), u(0.05)).fill(pal.outline);
    g.ellipse(u(hc + 0.02), u(-hw * 0.2), u(hl * 0.16), u(hw * 0.1)).fill({ color: pal.highlight, alpha: 0.25 });
  });

  // --- gaster: pivot at the front (postpetiole joint); the mass extends
  //     BACKWARD (−X) from the pivot — a +X gaster lands on the head ---
  const gl = art.gasterLen;
  const gw = art.gasterWid;
  const gaster = bake(renderer, (g) => {
    if (caste === 'worker') {
      g.ellipse(u(-gl * 0.48), 0, u(gl * 0.46), u(gw / 2)).fill(pal.gaster).stroke({ width: ow, color: pal.outline });
      for (const [hx, hy] of [[0.86, -0.3], [0.95, 0], [0.86, 0.3]] as const) {
        g.moveTo(u(-gl * hx), u(hy * gw)).lineTo(u(-gl * hx - 0.09), u(hy * gw * 1.35))
          .stroke({ width: P * 0.02, color: pal.outline, alpha: 0.8 });
      }
    } else {
      // bulbous two-segment gaster; the rear segment is elongated so the
      // abdomen reads as trailing (a wide fan reads as a blob — QC rounds
      // 3–4), with a segment seam and lighter apical band
      g.ellipse(u(-gl * 0.26), 0, u(gl * 0.26), u(gw * 0.36)).fill(pal.gaster).stroke({ width: ow, color: pal.outline });
      g.ellipse(u(-gl * 0.68), 0, u(gl * 0.42), u(gw * 0.46)).fill(pal.gaster).stroke({ width: ow, color: pal.outline });
    }
    g.ellipse(u(-gl * 0.9), 0, u(gl * 0.08), u(gw * 0.2)).fill({ color: pal.highlight, alpha: 0.22 });
    g.ellipse(u(-gl * 0.5), u(-gw * 0.16), u(gl * 0.18), u(gw * 0.1)).fill({ color: pal.highlight, alpha: 0.16 });
  });

  // --- waist merged into the thorax texture (above) ---

  // --- legs: the tibia carries a widened tarsus tip at its end (the
  //     planted-foot marker) — no separate foot quads ---
  const femur = bake(
    renderer,
    (g) => capsule(g, femurLen, lw, lw * 0.68, pal.limbs, pal.outline, lw * 0.8),
    femurLen,
  );
  const tibia = bake(
    renderer,
    (g) => {
      capsule(g, tibiaLen, lw * 0.8, lw * 0.6, pal.limbs, pal.outline);
      g.circle(u(tibiaLen), 0, u(art.footR * 1.4)).fill(pal.limbs).stroke({ width: ow, color: pal.outline });
    },
    tibiaLen,
  );

  // --- mandible: curved fang pivoting at its base, dark tip ---
  const ml = art.mandibleLen * hw;
  const mandible = bake(renderer, (g) => {
    g.moveTo(0, u(ml * 0.1))
      .bezierCurveTo(u(ml * 0.45), u(ml * 0.3), u(ml * 0.8), u(ml * 0.1), u(ml), u(-ml * 0.3))
      .lineTo(u(ml * 0.88), u(-ml * 0.24))
      .bezierCurveTo(u(ml * 0.7), 0, u(ml * 0.35), u(ml * 0.12), 0, u(-ml * 0.08))
      .closePath()
      .fill(pal.limbs)
      .stroke({ width: ow, color: pal.outline });
    g.circle(u(ml * 0.97), u(-ml * 0.27), u(0.045)).fill(pal.outline);
  }, ml);

  // --- antennae: scape + pale club (the visually dominant tip) ---
  const al = art.antennaLen * bodyUnits;
  const scapeLen = al * (1 - art.clubFrac);
  const clubLen = al * art.clubFrac;
  const scape = bake(renderer, (g) => capsule(g, scapeLen, 0.07, 0.055, pal.limbs, pal.outline), scapeLen);
  const club = bake(renderer, (g) => capsule(g, clubLen, 0.085, 0.11, pal.accent, pal.outline), clubLen);

  // --- wings: stubs (queen at rest) + full flight pair; both sweep
  //     BACKWARD from a root near the thorax rear (−X content) ---
  const wingStub =
    art.wings === 'stubs'
      ? bake(renderer, (g) => {
          g.ellipse(u(-0.24), 0, u(0.18), u(0.075)).fill({ color: 0xd8cfc0, alpha: 0.5 })
            .stroke({ width: ow * 0.6, color: pal.outline, alpha: 0.55 });
        })
      : null;
  const wing = bake(renderer, (g) => {
    g.ellipse(u(-0.85), 0, u(0.82), u(0.24)).fill({ color: 0xd8cfc0, alpha: 0.5 })
      .stroke({ width: ow * 0.6, color: pal.outline, alpha: 0.55 });
    g.moveTo(u(-0.1), 0).lineTo(u(-1.5), u(-0.08)).stroke({ width: P * 0.018, color: 0xffffff, alpha: 0.3 });
  });

  // --- ground shadow: one soft quad (stacked translucent ellipses would
  //     triple the overdraw per ant — the perf budget lives in fill rate) ---
  const shadow = bake(renderer, (g) => {
    g.ellipse(0, 0, u((bodyUnits + 0.3) * 0.58), u((gw + 0.35) * 0.58))
      .fill({ color: 0x000000, alpha: 0.16 });
  });

  return { thorax, head, gaster, femur, tibia, mandible, scape, club, wingStub, wing, shadow };
}

// --- carried items (team/caste-independent) ---

export interface ItemPart extends BakedPart {
  /** visual half-sizes in thorax units */
  rx: number;
  ry: number;
}

const itemCache = new Map<string, ItemPart>();

export function itemPart(renderer: PixiRenderer, kind: 'dirt' | 'egg' | string): ItemPart {
  const hit = itemCache.get(kind);
  if (hit) return hit;
  const o = P * 0.03;
  let part: ItemPart;
  if (kind === 'dirt') {
    part = {
      ...bake(renderer, (g) => {
        g.circle(u(-0.08), u(-0.06), u(0.16)).fill(0x8a6d4a).stroke({ width: o, color: 0x4a3a26 });
        g.circle(u(0.1), u(0.02), u(0.18)).fill(0x8a6d4a).stroke({ width: o, color: 0x4a3a26 });
        g.circle(u(-0.02), u(0.12), u(0.14)).fill(0x7a5f40).stroke({ width: o, color: 0x4a3a26 });
        g.circle(u(0.06), u(-0.1), u(0.045)).fill(0x5f4a30);
      }),
      rx: 0.3,
      ry: 0.3,
    };
  } else if (kind === 'egg') {
    part = {
      ...bake(renderer, (g) => {
        g.ellipse(0, 0, u(0.13), u(0.2)).fill(0xe8dcc8).stroke({ width: o, color: 0x9a8a6a });
        g.ellipse(u(-0.03), u(-0.06), u(0.05), u(0.08)).fill({ color: 0xffffff, alpha: 0.35 });
      }),
      rx: 0.13,
      ry: 0.2,
    };
  } else {
    const color = FOOD_COLORS[kind] ?? FOOD_COLORS.green;
    part = {
      ...bake(renderer, (g) => {
        g.circle(0, 0, u(0.14)).fill(color).stroke({ width: o, color: 0x1a140a });
        g.circle(u(-0.04), u(-0.04), u(0.04)).fill({ color: 0xffffff, alpha: 0.3 });
      }),
      rx: 0.14,
      ry: 0.14,
    };
  }
  itemCache.set(kind, part);
  return part;
}
