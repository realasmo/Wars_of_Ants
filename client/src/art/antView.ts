// The procedural ant rig: builds an ant from baked parts (bake.ts) and
// animates it — planted-feet walking (gait.ts), follow-through, idle
// micro-motion, carrying, wings. One AntView per ant; fixed sprite count;
// transforms only after construction (no per-frame Graphics rebuilds).
//
// Sprites live in three SHARED z-planes (shadows / legs / bodies) instead of
// a per-ant container: same-texture sprites stay adjacent in submission
// order, so the batcher draws each part type once per frame for the whole
// crowd. Placement is computed in world space from the ant's transform.
//
// Labels and HP bars stay on the renderer's per-entity containers. All
// cosmetic randomness is client-side, seeded per entity id — never the sim
// RNG (ticket determinism guardrail).

import { Container, Sprite, type Renderer as PixiRenderer } from 'pixi.js';
import { BAKE_PPU, CASTES, type Caste, type CasteArt } from './ants';
import { antParts, itemPart, type BakedPart } from './bake';
import { gaitStateFrom, ikKnee, replant, setMoveDir, stepGait, type GaitState } from './gait';
import type { ActivityName, Carry } from '../sim';

export interface AntFrame {
  x: number;
  y: number;
  activity: ActivityName;
  carry: Carry;
}

/** shared z-planes owned by the renderer, in draw order */
export interface AntLayers {
  shadows: Container;
  legs: Container;
  bodies: Container;
}

/** mulberry32 — small, seedable, good enough for idle-motion variety. */
function rngFor(id: number): () => number {
  let a = (id * 2654435761 + 0x9e3779b9) >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function shortArc(from: number, to: number): number {
  let d = (to - from) % (Math.PI * 2);
  if (d > Math.PI) d -= Math.PI * 2;
  if (d < -Math.PI) d += Math.PI * 2;
  return d;
}

interface AntennaState {
  /** <0: waiting (−remaining seconds); 0..dur: waving */
  t: number;
  dur: number;
  sign: number;
}

interface IdleChannel {
  t: number;
  dur: number;
  sign: number;
}

const KNEE: number[] = [0, 0];

export class AntView {
  readonly caste: Caste;
  readonly id: number;
  /** label offset above the body, in world tiles (scales with body size) */
  readonly labelY: number;
  private readonly renderer: PixiRenderer;
  private readonly art: CasteArt;
  private readonly unit: number; // world tiles per thorax unit
  private readonly rng: () => number;
  private readonly gait: GaitState;
  private readonly all: Sprite[] = [];

  // sprites in the shared planes
  private readonly shadowS: Sprite;
  private readonly femurS: Sprite[] = [];
  private readonly tibiaS: Sprite[] = [];
  private readonly gasterS: Sprite;
  private readonly thoraxS: Sprite;
  private readonly headS: Sprite;
  private readonly mandS: Sprite[] = [];
  private readonly antS: Sprite[] = [];
  private readonly clubS: Sprite[] = [];
  private readonly wingS: Sprite[] = [];
  private readonly wingStubS: Sprite[] = [];
  private readonly itemS: Sprite;

  // animation state
  private lastX = NaN;
  private lastY = NaN;
  private wx = 0;
  private wy = 0;
  private facing = 0;
  private angVel = 0;
  private speedEma = 0;
  private headLag = 0;
  private gasterLag = 0;
  private headRot = 0; // composed head-plane rotation this frame
  private gasterRot = 0;
  private gasterScale = 1;
  private sway = 0; // body-plane lateral offset (units)
  private wasFlying = false;
  private stale = true;
  private shown = true;
  /** idle throttle: standing ants update every 3rd frame (their motion is
   * slow breathing/antennae — visually identical, ⅓ the transform work) */
  private idleTick = 0;
  private idleSkip = false;
  private carryKey = 'none';
  private snapT = -1; // pickup snap timer (s); −1 = idle
  private releaseT = -1; // drop release timer (s)
  private itemLagY = 0;
  private readonly breathePhase: number;
  private readonly antA: [AntennaState, AntennaState] = [
    { t: -1, dur: 1, sign: 1 },
    { t: -2, dur: 1, sign: -1 },
  ];
  private headTurn: IdleChannel = { t: -3, dur: 1.2, sign: 1 };

  constructor(renderer: PixiRenderer, layers: AntLayers, caste: Caste, team: number, id: number) {
    this.renderer = renderer;
    this.caste = caste;
    this.id = id;
    this.art = CASTES[caste];
    const art = this.art;
    this.unit = art.bodyLenTiles / (art.headLen + 1 + art.gasterLen);
    this.labelY = -(0.55 + art.bodyLenTiles * 0.55);
    this.gait = gaitStateFrom(art);
    this.rng = rngFor(id);
    this.breathePhase = this.rng() * Math.PI * 2;
    for (const a of this.antA) this.scheduleAntenna(a);
    this.scheduleHeadTurn();

    const parts = antParts(renderer, caste, team);
    const add = (parent: Container, p: BakedPart): Sprite => {
      const s = new Sprite(p.texture);
      s.anchor.set(p.ax, p.ay);
      s.scale.set(this.unit / BAKE_PPU); // texture px → body units → tiles
      parent.addChild(s);
      this.all.push(s);
      return s;
    };

    // Insertion order groups same-texture sprites into contiguous runs
    // (all femurs, then all tibias; gasters before thoraxes before the head
    // stack) so the batcher draws each part type once per frame for the
    // whole crowd. Cross-ant overlaps within a part group are invisible at
    // play zoom; within one ant the planes still stack right.
    this.shadowS = add(layers.shadows, parts.shadow);
    for (let i = 0; i < 6; i++) this.femurS.push(add(layers.legs, parts.femur));
    for (let i = 0; i < 6; i++) this.tibiaS.push(add(layers.legs, parts.tibia));
    if (parts.wingStub !== null) {
      for (let i = 0; i < 2; i++) this.wingStubS.push(add(layers.bodies, parts.wingStub));
    }
    for (let i = 0; i < 2; i++) this.wingS.push(add(layers.bodies, parts.wing));
    this.gasterS = add(layers.bodies, parts.gaster);
    this.thoraxS = add(layers.bodies, parts.thorax);
    this.headS = add(layers.bodies, parts.head);
    for (let i = 0; i < 2; i++) this.mandS.push(add(layers.bodies, parts.mandible));
    for (let i = 0; i < 2; i++) this.antS.push(add(layers.bodies, parts.scape));
    for (let i = 0; i < 2; i++) this.clubS.push(add(layers.bodies, parts.club));
    this.itemS = new Sprite();
    this.itemS.anchor.set(0.5);
    this.itemS.visible = false;
    layers.bodies.addChild(this.itemS);
    this.all.push(this.itemS);
    this.mandS[1].scale.y = -this.unit / BAKE_PPU; // left mandible mirrors
    this.clubLen = parts.club.len;
  }

  private readonly clubLen: number;

  /** Off-screen / other-layer: hide the sprites; feet replant when back. */
  deactivate(): void {
    this.stale = true;
    if (this.shown) {
      this.shown = false;
      for (const s of this.all) s.visible = false;
    }
  }

  private reactivate(): void {
    if (!this.shown) {
      this.shown = true;
      for (const s of this.all) s.visible = true;
      this.itemS.visible = this.carryKey !== 'none';
    }
  }

  /** world-space placement: parent frame (px,py,pa) + local (lx,ly,la) in
   *  BODY UNITS (converted to tiles here — sprites live in tile space) */
  private static place(s: Sprite, px: number, py: number, pa: number, lx: number, ly: number, la: number, unit: number): void {
    lx *= unit;
    ly *= unit;
    const c = Math.cos(pa);
    const sn = Math.sin(pa);
    s.position.set(px + lx * c - ly * sn, py + lx * sn + ly * c);
    s.rotation = pa + la;
  }

  /** place on a nested frame (e.g. head-attached parts): sub-frame
   * (hx,hy,hr) and part local (lx,ly,la), all in body units/radians */
  private placeOn(s: Sprite, hx: number, hy: number, hr: number, lx: number, ly: number, la: number): void {
    const c = Math.cos(hr);
    const sn = Math.sin(hr);
    AntView.place(s, this.wx, this.wy, this.facing, hx + lx * c - ly * sn, hy + lx * sn + ly * c, hr + la, this.unit);
  }

  update(dt: number, f: AntFrame): void {
    const art = this.art;
    dt = Math.max(dt, 1 / 240);
    this.reactivate();
    if (this.stale || Number.isNaN(this.lastX)) {
      this.lastX = f.x;
      this.lastY = f.y;
      this.stale = false;
      this.idleSkip = false;
      replant(this.gait, f.x, f.y, this.facing);
    }
    // idle throttle: fully-planted, slow-moving ants advance at 20 Hz
    const anySwing = !this.gait.legs[0].planted || !this.gait.legs[1].planted || !this.gait.legs[2].planted
      || !this.gait.legs[3].planted || !this.gait.legs[4].planted || !this.gait.legs[5].planted;
    if (this.idleSkip) {
      this.idleTick++;
      if (this.idleTick >= 3) {
        this.idleTick = 0;
        this.idleSkip = false;
      } else {
        this.lastX = f.x;
        this.lastY = f.y;
        return;
      }
      dt *= 3; // this frame advances the two skipped frames too
    }
    let dx = f.x - this.lastX;
    let dy = f.y - this.lastY;
    this.lastX = f.x;
    this.lastY = f.y;
    let dist = Math.hypot(dx, dy);
    if (dist > 2.5) {
      // teleport (entrance snap etc.) — never drag planted feet across the map
      dist = 0;
      dx = 0;
      dy = 0;
      replant(this.gait, f.x, f.y, this.facing);
    }
    const speed = dist / dt;
    this.speedEma += (speed - this.speedEma) * 0.25;
    if (dist > 1e-6) {
      setMoveDir(this.gait, dx, dy);
      const target = Math.atan2(dy, dx);
      const turn = shortArc(this.facing, target);
      this.angVel += (turn / dt - this.angVel) * 0.2;
      const track = Math.min(1, 9 * dt * (0.35 + Math.min(1, this.speedEma / 1.5)));
      this.facing += turn * track;
    } else {
      this.angVel += -this.angVel * Math.min(1, 6 * dt);
    }

    const flying = f.activity === 'flying';
    if (this.wasFlying && !flying) replant(this.gait, f.x, f.y, this.facing);
    this.wasFlying = flying;

    this.wx = f.x;
    this.wy = f.y;

    // re-arm the idle throttle once the ant settles (planted, ~standing)
    if (!anySwing && !flying && dist < 1e-6 && this.speedEma < 0.02 && this.snapT < 0 && this.releaseT < 0) {
      this.idleSkip = true;
    } else {
      this.idleSkip = false;
      this.idleTick = 0;
    }

    // body sway over planted feet (one side-swing per step), in units
    this.sway = Math.sin(this.gait.swayPhase * Math.PI) * art.gait.sway * (art.headLen + 1 + art.gasterLen);

    if (flying) {
      this.updateFlyingLegs(dt);
    } else {
      stepGait(this.gait, f.x, f.y, this.facing, dist, this.speedEma, dt);
    }
    this.updateIdleChannels(dt, flying);
    this.updateLegs();
    this.updateBodyParts(dt, f, flying);
  }

  private updateFlyingLegs(dt: number): void {
    // airborne: no ground contact — feet drift to tucked targets near the body
    const k = Math.min(1, 10 * dt);
    const c = Math.cos(this.facing);
    const s = Math.sin(this.facing);
    const tuck = this.art.tuckFrac;
    for (let i = 0; i < 6; i++) {
      const tx = this.gait.hipX[i] + (this.gait.restX[i] - this.gait.hipX[i]) * tuck;
      const ty = this.gait.hipY[i] + (this.gait.restY[i] - this.gait.hipY[i]) * tuck;
      const wx = this.wx + tx * c - ty * s;
      const wy = this.wy + tx * s + ty * c;
      const leg = this.gait.legs[i];
      leg.planted = false;
      leg.lift = 0.15;
      leg.x += (wx - leg.x) * k;
      leg.y += (wy - leg.y) * k;
    }
  }

  private updateLegs(): void {
    const inv = 1 / this.unit; // world tiles → body units
    const c = Math.cos(this.facing);
    const s = Math.sin(this.facing);
    for (let i = 0; i < 6; i++) {
      const leg = this.gait.legs[i];
      // foot world → body space (units)
      const fx = leg.x - this.wx;
      const fy = leg.y - this.wy;
      const lx = (fx * c + fy * s) * inv;
      const ly = (-fx * s + fy * c) * inv;
      const hx = this.gait.hipX[i] * inv;
      const hy = (this.gait.hipY[i] * inv) + this.sway;
      ikKnee(hx, hy, lx, ly, this.gait.params.femur * inv, this.gait.params.tibia * inv, i < 3 ? -1 : 1, KNEE);
      // place in world space: femur hip→knee, tibia knee→foot (the widened
      // tibia tip marks the planted foot; the fake lift thins it mid-air)
      AntView.place(this.femurS[i], this.wx, this.wy, this.facing, hx, hy, Math.atan2(KNEE[1] - hy, KNEE[0] - hx), this.unit);
      AntView.place(this.tibiaS[i], this.wx, this.wy, this.facing, KNEE[0], KNEE[1], Math.atan2(ly - KNEE[1], lx - KNEE[0]), this.unit);
      this.tibiaS[i].scale.y = ((1 - leg.lift * 0.3) * this.unit) / BAKE_PPU;
    }
  }

  /** idle channels: antenna waves, head turns, breathing — set the composed
   * plane rotations consumed by updateBodyParts */
  private updateIdleChannels(dt: number, flying: boolean): void {
    const art = this.art;
    // follow-through: head + gaster trail the thorax on turns
    const headTarget = Math.max(-0.6, Math.min(0.6, -this.angVel * art.gait.trailHead));
    this.headLag += (headTarget - this.headLag) * Math.min(1, 8 * dt);
    const gasterTarget = Math.max(-0.8, Math.min(0.8, -this.angVel * art.gait.trailGaster));
    this.gasterLag += (gasterTarget - this.gasterLag) * Math.min(1, 5 * dt);
    this.gasterRot = this.gasterLag + Math.sin(this.gait.swayPhase * Math.PI + 0.6) * art.gait.sway * 2.2;
    const b =
      art.idle.breatheAmp *
      Math.sin((performance.now() / (art.idle.breatheSec * 1000)) * Math.PI * 2 + this.breathePhase);
    this.gasterScale = 1 + b;

    const idle = this.speedEma < 0.03 && !flying;
    const ht = this.headTurn;
    let headTurnAng = 0;
    if (ht.t >= 0) {
      ht.t += dt;
      if (ht.t >= ht.dur) this.scheduleHeadTurn();
      else headTurnAng = art.idle.headAmp * ht.sign * Math.sin((Math.PI * ht.t) / ht.dur);
    } else if (idle) {
      ht.t += dt;
      if (ht.t >= 0) {
        ht.t = 0;
        ht.dur = 1 + this.rng() * 0.6;
      }
    }
    this.headRot = this.headLag + headTurnAng;
  }

  private updateBodyParts(dt: number, f: AntFrame, flying: boolean): void {
    const art = this.art;
    const hl = art.headLen;
    const hw = art.headWid;
    const now = performance.now() / 1000;

    // ground shadow rides the body, faded + offset in flight
    AntView.place(this.shadowS, this.wx, this.wy, this.facing, -0.3, flying ? 0.18 : 0.06, 0, this.unit);
    this.shadowS.alpha = flying ? 0.45 : 1;

    // body-plane parts (thorax center is the ant origin; sway offsets them)
    AntView.place(this.thoraxS, this.wx, this.wy, this.facing, 0, this.sway, 0, this.unit);
    AntView.place(this.gasterS, this.wx, this.wy, this.facing, -0.72, this.sway, this.gasterRot, this.unit);
    const gs = (this.gasterScale * this.unit) / BAKE_PPU;
    this.gasterS.scale.set(gs, gs * 0.82);

    // head plane: pivot at the neck, rotation = follow-through + idle turns
    const hx = 0.56 * this.unit;
    const hy = this.sway * this.unit;
    AntView.place(this.headS, this.wx, this.wy, this.facing, hx, hy, this.headRot, this.unit);

    // mandibles: bases at the head front corners; open angle per activity
    const carrying = f.carry.t !== 'none';
    const key = carryKey(f.carry);
    if (key !== this.carryKey) {
      if (this.carryKey === 'none' && key !== 'none') this.snapT = 0;
      else if (this.carryKey !== 'none' && key === 'none') this.releaseT = 0;
      this.carryKey = key;
      this.applyItemTexture(f.carry);
    }
    if (this.snapT >= 0 && (this.snapT += dt) > 0.14) this.snapT = -1;
    if (this.releaseT >= 0 && (this.releaseT += dt) > 0.18) this.releaseT = -1;
    let open: number;
    if (f.activity === 'digging') open = 0.12 + 0.3 * (0.5 + 0.5 * Math.sin(now * 16));
    else if (f.activity === 'fighting') open = 0.16 + 0.42 * (0.5 + 0.5 * Math.sin(now * 10));
    else if (this.snapT >= 0) {
      const ph = this.snapT / 0.14;
      open = ph < 0.55 ? 0.06 + 0.5 * Math.sin((Math.PI * ph) / 0.55) : carrying ? 0.14 : 0.06;
    } else if (this.releaseT >= 0) open = 0.06 + 0.25 * Math.sin(Math.PI * (this.releaseT / 0.18));
    else open = carrying ? 0.14 : flying ? 0.04 : 0.06;
    const mbase = 0.1 + hl / 2;
    this.placeOn(this.mandS[0], hx, hy, this.headRot, mbase, hw * 0.22, open);
    this.placeOn(this.mandS[1], hx, hy, this.headRot, mbase, -hw * 0.22, -open);

    // antennae: base rotation + idle wave + counter-tilt on turns + speed sweep
    const walking = this.speedEma > 0.05;
    const counter = Math.max(-0.5, Math.min(0.5, -this.angVel * 1.1));
    const sweepBase = Math.min(1, this.speedEma / 1.8) * 0.16;
    for (let i = 0; i < 2; i++) {
      const side = i === 0 ? 1 : -1;
      const a = this.antA[i];
      let wave = 0;
      if (a.t >= 0) {
        a.t += dt;
        if (a.t >= a.dur) this.scheduleAntenna(a);
        else wave = art.idle.waveAmp * a.sign * Math.sin((Math.PI * a.t) / a.dur);
      } else if (!walking && !flying) {
        a.t += dt;
      }
      const la = side * 0.55 + wave + counter + sweepBase * side;
      const abase = 0.08 + hl * 0.42;
      this.placeOn(this.antS[i], hx, hy, this.headRot, abase, side * hw * 0.12, la);
      // club chains at the scape tip, curled slightly
      const tipAng = this.antS[i].rotation;
      this.clubS[i].position.set(
        this.antS[i].x + Math.cos(tipAng) * this.clubLen * this.unit,
        this.antS[i].y + Math.sin(tipAng) * this.clubLen * this.unit,
      );
      this.clubS[i].rotation = tipAng + side * 0.25;
    }

    // carried item between the mandible tips, swaying against head turns
    this.itemS.visible = carrying;
    if (carrying && f.carry.t === 'dirt') {
      const k = ((1 + 0.25 * (f.carry.blocks - 1)) * this.unit) / BAKE_PPU;
      this.itemS.scale.set(k, k);
    } else {
      this.itemS.scale.set(this.unit / BAKE_PPU, this.unit / BAKE_PPU);
    }
    const lagTarget = Math.max(-0.25, Math.min(0.25, this.angVel * 0.35));
    this.itemLagY += (lagTarget - this.itemLagY) * Math.min(1, 6 * dt);
    this.placeOn(this.itemS, hx, hy, this.headRot, 0.1 + hl / 2 + 0.16, this.itemLagY, 0);

    // wings (queen): stubs at rest, fluttering pair in flight; both sweep
    // back-outward (−X content, mirrored tilt per side)
    for (let i = 0; i < this.wingStubS.length; i++) {
      const side = i === 0 ? 1 : -1;
      AntView.place(this.wingStubS[i], this.wx, this.wy, this.facing, -0.08, this.sway + side * 0.3, -side * 0.52, this.unit);
    }
    for (let i = 0; i < this.wingS.length; i++) {
      const w = this.wingS[i];
      const side = i === 0 ? 1 : -1;
      w.visible = flying;
      if (flying) {
        AntView.place(w, this.wx, this.wy, this.facing, -0.12, this.sway + side * 0.28, -side * (0.45 - 0.32 * Math.sin(now * 55 + i * 2.1)), this.unit);
      }
    }
  }

  private applyItemTexture(carry: Carry): void {
    if (carry.t === 'none') return;
    this.itemS.texture = itemPart(this.renderer, carry.t === 'food' ? carry.food : carry.t).texture;
  }

  private scheduleAntenna(a: AntennaState): void {
    const g = this.art.idle;
    a.t = -(g.waveGap[0] + this.rng() * (g.waveGap[1] - g.waveGap[0]));
    a.dur = g.waveDur[0] + this.rng() * (g.waveDur[1] - g.waveDur[0]);
    a.sign = this.rng() < 0.5 ? -1 : 1;
  }

  private scheduleHeadTurn(): void {
    const g = this.art.idle;
    this.headTurn.t = -(g.headEvery[0] + this.rng() * (g.headEvery[1] - g.headEvery[0]));
    this.headTurn.sign = this.rng() < 0.5 ? -1 : 1;
  }

  /** e2e/debug: feet in world space + swing state (planted-feet checks). */
  debug(): Record<string, unknown> {
    return {
      id: this.id,
      caste: this.caste,
      facing: this.facing,
      speed: this.speedEma,
      swayPhase: this.gait.swayPhase,
      antennae: [this.antS[0].rotation, this.antS[1].rotation],
      legs: this.gait.legs.map((l, i) => ({
        i,
        group: i === 0 || i === 2 || i === 4 ? 'A' : 'B',
        planted: l.planted,
        x: l.x,
        y: l.y,
        t: l.t,
        lift: l.lift,
      })),
    };
  }

  /** e2e/debug: world placement of the core body sprites. */
  partsDebug(): Record<string, unknown> {
    const s = (sp: Sprite): Record<string, unknown> => ({
      x: +sp.x.toFixed(2),
      y: +sp.y.toFixed(2),
      rot: +sp.rotation.toFixed(2),
      vis: sp.visible,
      sx: +sp.scale.x.toFixed(4),
      sy: +sp.scale.y.toFixed(4),
      alpha: sp.alpha,
      tex: sp.texture.width + 'x' + sp.texture.height,
    });
    return {
      ant: [this.wx, this.wy],
      thorax: s(this.thoraxS),
      gaster: s(this.gasterS),
      head: s(this.headS),
      item: s(this.itemS),
      wings: this.wingS.map(s),
    };
  }

  destroy(): void {
    for (const s of this.all) {
      s.removeFromParent();
      s.destroy();
    }
  }
}

function carryKey(c: Carry): string {
  if (c.t === 'food') return `food:${c.food}`;
  return c.t;
}
