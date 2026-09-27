// Planted-feet gait solver + 2-bone leg IK — pure math, no Pixi.
//
// The single most important rule (ticket): a planted foot NEVER slides. Foot
// positions live in world (tile) space; while planted they are not touched.
// A leg lifts only when (a) its drift from the ideal rest point exceeds the
// stride trigger, (b) the OTHER tripod is fully planted, and (c) its in-group
// ripple predecessor is far enough through its own swing (stagger). Swings
// advance with distance traveled (with a time floor so a halted ant still
// lands mid-air feet) and overshoot the landing point before settling on it.
//
// Body space: +X forward through the head, +Y to the ant's RIGHT (screen
// y-down). Legs are ordered L1 L2 L3 R1 R2 R3 (front/mid/rear).
// Tripods: A = L1+R2+L3 (indices 0,4,2), B = R1+L2+R3 (3,1,5).

import type { CasteArt } from './ants';

export interface Leg {
  planted: boolean;
  /** foot position in world tiles (authoritative while planted) */
  x: number;
  y: number;
  fromX: number;
  fromY: number;
  toX: number;
  toY: number;
  /** normalized swing direction (from → to), for the overshoot offset */
  dirX: number;
  dirY: number;
  /** swing progress 0..1 */
  t: number;
  /** fake lift 0..1 while swinging (drives foot-dot shrink + tibia stretch) */
  lift: number;
}

export interface GaitParams {
  /** step trigger distance in world tiles */
  trigger: number;
  /** landing lead ahead of rest, in tiles, at full speed */
  lead: number;
  swingMaxSec: number;
  stagger: number;
  overshoot: number;
  /** reference speed (tiles/s) at which `lead` applies in full */
  refSpeed: number;
  femur: number;
  tibia: number;
}

export interface GaitState {
  legs: Leg[];
  /** rest foot + hip positions in body space (tiles), indexed like legs */
  restX: number[];
  restY: number[];
  hipX: number[];
  hipY: number[];
  /** body-space leg lengths (tiles) for IK */
  params: GaitParams;
  /** sway accumulator: +1 per trigger distance walked */
  swayPhase: number;
  /** step starts since the other tripod last stepped (alternation budget) */
  groupSteps: [number, number];
  /** last movement direction (unit), persists at rest */
  moveX: number;
  moveY: number;
}

export const GROUP_A = [0, 4, 2] as const; // L1, R2, L3 — ripple order front→rear
export const GROUP_B = [3, 1, 5] as const; // R1, L2, R3

/** 0 = group A, 1 = group B, per leg index. */
export const LEG_GROUP = [0, 1, 0, 1, 0, 1];

export function gaitStateFrom(art: CasteArt): GaitState {
  const bodyUnits = art.headLen + 1 + art.gasterLen;
  const unit = art.bodyLenTiles / bodyUnits; // tiles per thorax unit
  const legLen = art.legRatio * bodyUnits * unit;
  const legs: Leg[] = [];
  const restX: number[] = [];
  const restY: number[] = [];
  const hipX: number[] = [];
  const hipY: number[] = [];
  for (let i = 0; i < 6; i++) {
    const side = i < 3 ? -1 : 1; // L legs at -Y, R legs at +Y
    const s = i % 3; // 0 front 1 mid 2 rear
    const attach = art.legAttach[s];
    const splay = (art.splay[s] * Math.PI) / 180;
    const fwd = s === 2 ? -1 : 1; // front/mid reach forward of the hip, rear back
    const dx = Math.cos(splay) * fwd;
    const dy = Math.sin(splay) * side;
    // normalize so splay shortening doesn't shrink the leg
    const m = Math.hypot(dx, dy);
    hipX.push(attach * unit);
    hipY.push(side * 0.42 * unit);
    // knees stay bent at rest: feet sit at ~80% of full extension
    const reach = legLen * 0.8;
    restX.push(hipX[i] + (dx / m) * reach);
    restY.push(hipY[i] + (dy / m) * reach);
    legs.push({ planted: true, x: 0, y: 0, fromX: 0, fromY: 0, toX: 0, toY: 0, dirX: 1, dirY: 0, t: 1, lift: 0 });
  }
  return {
    legs,
    restX,
    restY,
    hipX,
    hipY,
    params: {
      trigger: art.gait.strideTrigger * bodyUnits * unit,
      lead: art.gait.strideLead * art.gait.strideTrigger * bodyUnits * unit,
      swingMaxSec: art.gait.swingMaxSec,
      stagger: art.gait.stagger,
      overshoot: art.gait.overshoot,
      refSpeed: 2.4,
      femur: legLen * art.femurFrac,
      tibia: legLen * (1 - art.femurFrac),
    },
    swayPhase: 0,
    groupSteps: [0, 0],
    moveX: 1,
    moveY: 0,
  };
}

const GROUPS = [GROUP_A, GROUP_B];

function groupSwinging(legs: Leg[], group: readonly number[]): boolean {
  for (const i of group) if (!legs[i].planted) return true;
  return false;
}

/**
 * Advance the gait one frame. `dist` = world distance the body moved this
 * frame, `speed` = tiles/s (smoothed), `moveX/moveY` = unit travel direction
 * (persistent — pass the last known one when standing). Planted feet are
 * never moved; swings may start only under the full tripod gating.
 */
export function stepGait(
  st: GaitState,
  bodyX: number,
  bodyY: number,
  rot: number,
  dist: number,
  speed: number,
  dt: number,
): void {
  const p = st.params;
  st.swayPhase += dist / p.trigger;

  const urgency = Math.min(1, speed / p.refSpeed);
  const leadX = st.moveX * p.lead * urgency;
  const leadY = st.moveY * p.lead * urgency;
  const c = Math.cos(rot);
  const s = Math.sin(rot);

  // 1) advance in-flight swings (distance-driven, with a time floor so a
  //    halted ant still lands its feet within swingMaxSec)
  for (let i = 0; i < 6; i++) {
    const leg = st.legs[i];
    if (leg.planted) continue;
    const travel = Math.max(Math.hypot(leg.toX - leg.fromX, leg.toY - leg.fromY), p.trigger * 0.6);
    leg.t += Math.max(dist / travel, dt / p.swingMaxSec);
    if (leg.t >= 1) {
      leg.t = 1;
      leg.planted = true;
      leg.x = leg.toX; // land exactly on the target — no settling slide
      leg.y = leg.toY;
      leg.lift = 0;
    } else {
      const e = leg.t * leg.t * (3 - 2 * leg.t); // smoothstep
      leg.x = leg.fromX + (leg.toX - leg.fromX) * e;
      leg.y = leg.fromY + (leg.toY - leg.fromY) * e;
      // overshoot: peak ~80% through the swing, exactly 0 at landing
      const o = p.overshoot * travel * 12.2 * leg.t ** 4 * (1 - leg.t);
      leg.x += leg.dirX * o;
      leg.y += leg.dirY * o;
      leg.lift = Math.sin(Math.PI * leg.t);
    }
  }

  // 2) start new swings — one group at a time, rippled front→mid→rear.
  // Rounds alternate via a step budget: a group gets three starts (one
  // round); without it, a freshly landed front leg re-triggers instantly
  // (the body outruns its landing prediction at speed) and monopolizes the
  // gait. The other tripod's first step resets the budget.
  for (let g = 0; g < 2; g++) {
    const own = GROUPS[g];
    const other = GROUPS[1 - g];
    if (groupSwinging(st.legs, other)) continue;
    if (st.groupSteps[g] >= own.length) {
      // this group spent its round — yield unless the other tripod is content
      let otherNeeds = false;
      for (const i of other) {
        const leg = st.legs[i];
        const ix = bodyX + (st.restX[i] * c - st.restY[i] * s) + leadX;
        const iy = bodyY + (st.restX[i] * s + st.restY[i] * c) + leadY;
        if (Math.hypot(leg.x - ix, leg.y - iy) > p.trigger) {
          otherNeeds = true;
          break;
        }
      }
      if (otherNeeds) continue;
    }
    for (let k = 0; k < own.length; k++) {
      const i = own[k];
      const leg = st.legs[i];
      if (!leg.planted) continue;
      // ripple gate: no earlier leg of this group may be in its early swing
      let blocked = false;
      for (let j = 0; j < k; j++) {
        const e = st.legs[own[j]];
        if (!e.planted && e.t < p.stagger) blocked = true;
      }
      if (blocked) continue;
      // ideal landing point: rotated rest + lead ahead of the body
      const ix = bodyX + (st.restX[i] * c - st.restY[i] * s) + leadX;
      const iy = bodyY + (st.restX[i] * s + st.restY[i] * c) + leadY;
      const dev = Math.hypot(leg.x - ix, leg.y - iy);
      if (dev <= p.trigger) continue;
      leg.planted = false;
      leg.t = 0;
      leg.fromX = leg.x;
      leg.fromY = leg.y;
      leg.toX = ix;
      leg.toY = iy;
      const m = Math.hypot(ix - leg.x, iy - leg.y) || 1;
      leg.dirX = (ix - leg.x) / m;
      leg.dirY = (iy - leg.y) / m;
      leg.lift = 0;
      st.groupSteps[g]++;
      st.groupSteps[1 - g] = 0;
    }
  }
}

/** Set the persistent travel direction (call every frame the body moves). */
export function setMoveDir(st: GaitState, dx: number, dy: number): void {
  const m = Math.hypot(dx, dy);
  if (m > 1e-9) {
    st.moveX = dx / m;
    st.moveY = dy / m;
  }
}

/** Snap every foot onto its ideal rest point (teleport, layer change, rig
 * resume after culling, landing after flight). */
export function replant(st: GaitState, bodyX: number, bodyY: number, rot: number): void {
  const c = Math.cos(rot);
  const s = Math.sin(rot);
  for (let i = 0; i < 6; i++) {
    const leg = st.legs[i];
    leg.planted = true;
    leg.t = 1;
    leg.lift = 0;
    leg.x = bodyX + (st.restX[i] * c - st.restY[i] * s);
    leg.y = bodyY + (st.restX[i] * s + st.restY[i] * c);
  }
}

/**
 * 2-bone IK in body space. Knee bends outward/backward (away from the head):
 * `bend` is +1 for right-side legs (+Y), -1 for left. Writes the knee point
 * to (outKx, outKy) via the returned array [kx, ky] — allocation-free when
 * the caller passes a scratch array.
 */
export function ikKnee(
  hx: number,
  hy: number,
  fx: number,
  fy: number,
  l1: number,
  l2: number,
  bend: number,
  out: number[],
): void {
  let dx = fx - hx;
  let dy = fy - hy;
  let d = Math.hypot(dx, dy);
  const maxD = l1 + l2 - 1e-4;
  const minD = Math.abs(l1 - l2) + 1e-4;
  if (d > maxD) {
    const k = maxD / d;
    dx *= k;
    dy *= k;
    d = maxD;
  } else if (d < minD) {
    const k = minD / d;
    dx *= k;
    dy *= k;
    d = minD;
  }
  const a = (l1 * l1 - l2 * l2 + d * d) / (2 * d);
  const h = Math.sqrt(Math.max(0, l1 * l1 - a * a));
  const ux = dx / d;
  const uy = dy / d;
  // perpendicular (uy, -ux): for right legs (+bend) this points outward from
  // the body and slightly backward — the natural ant knee
  out[0] = hx + ux * a + uy * h * bend;
  out[1] = hy + uy * a - ux * h * bend;
}
