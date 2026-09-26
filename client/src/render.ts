import { Application, Container, Graphics, Text } from 'pixi.js';
import type { Sim, Ent } from './sim';
import { isAnt, lerpPos } from './sim';

const SURFACE_COLORS: Record<number, number> = {
  0: 0x4a6741,
  1: 0x4a6741,
  2: 0x4a6741,
  3: 0x4a6741,
  4: 0x40404a,
};

const UNDER_COLORS: Record<number, number> = {
  0: 0x171310,
  1: 0x5c4033,
  2: 0x4a3527,
  3: 0x6e4f3a,
  4: 0x3a3a42,
};

const BASE_PX = 30;

/** Team body palettes: [queen body, queen head, worker body, worker head, soldier body, soldier head]. */
const TEAM_COLORS: Record<number, number[]> = {
  0: [0x8e2f3c, 0x5e1e27, 0x9c6b3c, 0x6e4826, 0x3d2b1f, 0x7a2a2a],
  1: [0x3c5a8e, 0x27405e, 0x4a6b9c, 0x2e486e, 0x2b3542, 0x2a4a7a],
};

interface EntityGfx {
  c: Container;
  g: Graphics;
  t: Text;
  kind: string;
  carrying: boolean;
  hpBucket: number;
  label: string;
  flying: boolean;
  /** Short carry label ('none' | 'dirt×n' | 'egg' | food name). */
  haul: string;
  team: number;
}

const SOURCE_NAMES: Record<number, string> = {
  1: 'moss',
  2: 'mushroom',
  3: 'raspberry',
  4: 'strawberry',
  5: 'cockroach',
  6: 'caterpillar',
};

/** Unit / carried-dot colors per resource kind. */
const RES_COLORS: Record<string, number> = {
  green: 0x3fa34d,
  super: 0x4a7fd9,
  protein: 0xc05a5a,
  carbs: 0xd4a832,
  water: 0x4a9fd9,
};

function labelText(s: Ent): string {
  switch (s.kind) {
    case 'queen':
      return 'QUEEN';
    case 'worker':
      return 'worker';
    case 'soldier':
      return 'soldier';
    case 'spider':
      return 'SPIDER';
    case 'egg':
      return s.caste === 'soldier' ? 'egg(S)' : 'egg';
    case 'source':
      return SOURCE_NAMES[s.src] ?? 'source';
    case 'food':
      return s.food === 'super' ? 'SUPER' : s.food;
    default:
      return '?';
  }
}

/** Short carry tag for the redraw change-check ('none' | 'dirt' | 'egg' | food). */
function haulLabel(c: { t: 'none' } | { t: 'dirt'; blocks: number } | { t: 'egg' } | { t: 'food'; food: string }): string {
  switch (c.t) {
    case 'none':
      return 'none';
    case 'dirt':
      return `dirt×${c.blocks}`;
    case 'egg':
      return 'egg';
    case 'food':
      return c.food;
  }
}

/** Placeholder visuals for the six finite map sources; size follows the
 * remaining amount. */
function drawSource(g: Graphics, s: Extract<Ent, { kind: 'source' }>) {
  const t = s.src;
  const k = Math.max(0.25, Math.min(1, s.amount / 40));
  if (t === 1) {
    // moss: low green tufts
    g.circle(-0.2 * k, 0.1, 0.22 * k).fill(0x4a7a4a);
    g.circle(0.15 * k, 0.05, 0.18 * k).fill(0x568a52);
    g.circle(-0.02, -0.12 * k, 0.2 * k).fill(0x4a7a4a);
  } else if (t === 2) {
    // mushroom: brown cap on a stem
    g.rect(-0.08 * k, -0.1, 0.16 * k, 0.35 * k).fill(0xd8cfc0);
    g.ellipse(0, -0.15, 0.35 * k, 0.18 * k).fill(0x8a5a3a);
  } else if (t === 3) {
    // raspberry: dark red drupelet cluster
    g.circle(-0.18, 0.05, 0.16 * k).fill(0xa8324a);
    g.circle(0.15, 0.1, 0.15 * k).fill(0x9a2a42);
    g.circle(0, -0.12, 0.17 * k).fill(0xb23a52);
  } else if (t === 4) {
    // strawberry: red body + green leaf
    g.ellipse(0, 0.05, 0.28 * k, 0.22 * k).fill(0xd94a5a);
    g.ellipse(0, -0.18, 0.16 * k, 0.08 * k).fill(0x4a8a4a);
  } else if (t === 5) {
    // cockroach: dark oval + antennae
    g.ellipse(0, 0, 0.34 * k, 0.18 * k).fill(0x4a3b2a);
    g.moveTo(-0.3, -0.1).lineTo(-0.5, -0.25).stroke({ width: 0.05, color: 0x4a3b2a });
    g.moveTo(-0.3, -0.05).lineTo(-0.52, -0.1).stroke({ width: 0.05, color: 0x4a3b2a });
  } else {
    // caterpillar: green segments
    for (let i = 0; i < 4; i++) {
      g.circle(-0.3 + i * 0.2, 0, (0.16 - i * 0.01) * (0.6 + 0.4 * k)).fill(0x7aa832);
    }
  }
}

function makeLabel(text: string): Text {
  const t = new Text({
    text,
    style: {
      fontFamily: 'monospace',
      fontSize: 32,
      fill: 0xf0e8da,
      stroke: { color: 0x000000, width: 5 },
    },
  });
  t.scale.set(0.011);
  t.anchor.set(0.5);
  return t;
}

export class Renderer {
  readonly app: Application;
  private world = new Container();
  private layerC: Container[] = [new Container(), new Container()];
  private tileG: Graphics[] = [new Graphics(), new Graphics()];
  private entities = new Container();
  private sprites = new Map<number, EntityGfx>();
  private ring = new Graphics();
  private entranceMarks: [Text, Text] = [new Text(''), new Text('')];
  private sim: Sim;
  cam = { x: 48, y: 8, zoom: 1 };
  activeLayer = 1;

  private constructor(sim: Sim, app: Application) {
    this.sim = sim;
    this.app = app;
  }

  static async create(sim: Sim, host: HTMLElement): Promise<Renderer> {
    const app = new Application();
    const params = new URLSearchParams(window.location.search);
    await app.init({
      background: 0x0b0a08,
      antialias: true,
      resolution: window.devicePixelRatio || 1,
      autoDensity: true,
      preserveDrawingBuffer: params.get('e2e') === '1',
    });
    const r = new Renderer(sim, app);
    host.appendChild(app.canvas);
    app.stage.addChild(r.world);
    r.world.addChild(r.layerC[0], r.layerC[1], r.entities, r.ring);
    r.layerC[0].addChild(r.tileG[0]);
    r.layerC[1].addChild(r.tileG[1]);
    window.addEventListener('resize', () => {
      app.renderer.resize(window.innerWidth, window.innerHeight);
    });
    app.renderer.resize(window.innerWidth, window.innerHeight);
    r.drawTiles(0);
    r.drawTiles(1);
    r.makeEntranceMarkers();
    if (sim.foundingMode) {
      // founding: the camera starts on the surface, on the flying queen
      r.setActiveLayer(0);
      const q = sim.queenId();
      const qs = q !== null ? sim.cur.get(q) : undefined;
      r.cam.x = qs?.x ?? sim.w / 2;
      r.cam.y = qs?.y ?? sim.h / 2;
    } else {
      r.setActiveLayer(1);
      r.cam.x = sim.entrance?.[0] ?? sim.w / 2;
      r.cam.y = (sim.entrance?.[1] ?? 0) + 3;
    }
    r.refreshEntrance();
    r.applyCamera();
    return r;
  }

  private makeEntranceMarkers(): void {
    const under = makeLabel('EXIT ▲');
    const surface = makeLabel('NEST ▼');
    under.visible = false;
    surface.visible = false;
    this.layerC[1].addChild(under);
    this.layerC[0].addChild(surface);
    this.entranceMarks = [under, surface];
  }

  /** Position/show/hide the entrance labels; call whenever the entrance
   * changes (it appears when the founding queen creates the nest). */
  refreshEntrance(): void {
    const [under, surface] = this.entranceMarks;
    const ent = this.sim.entrance;
    if (ent === null) {
      under.visible = false;
      surface.visible = false;
      return;
    }
    under.visible = true;
    surface.visible = true;
    under.position.set(ent[0] + 1, ent[1] + 3.0);
    surface.position.set(ent[0] + 1, ent[1] - 0.4);
  }

  setActiveLayer(layer: number): void {
    this.activeLayer = layer;
    this.layerC[0].visible = layer === 0;
    this.layerC[1].visible = layer === 1;
    this.drawTiles(layer);
    this.sim.consumeDirty(layer);
  }

  toggleLayer(): void {
    this.setActiveLayer(this.activeLayer === 0 ? 1 : 0);
  }

  drawTiles(layer: number): void {
    const g = this.tileG[layer];
    g.clear();
    const t = this.sim.tiles(layer);
    const pal = layer === 0 ? SURFACE_COLORS : UNDER_COLORS;
    for (let y = 0; y < this.sim.h; y++) {
      for (let x = 0; x < this.sim.w; x++) {
        const kind = t[y * this.sim.w + x];
        g.rect(x, y, 1, 1).fill(pal[kind]);
        // soil tint: surface dust patches anywhere; underground soil only on
        // dug-out cells (hidden under the dirt until excavated)
        const soil = this.sim.soilAt(layer, x, y);
        if (soil > 0 && (layer === 0 || kind === 0)) {
          const color = soil === 1 ? 0xc07830 : 0xb8bcc4;
          const alpha = layer === 0 ? 0.16 : 0.34;
          g.rect(x, y, 1, 1).fill({ color, alpha });
        }
      }
    }
    const [ex, ey] = this.sim.entrance ?? [-10, -10];
    const cx = ex + 1;
    const cy = ey + 1;
    g.circle(cx, cy, 2.1).fill({ color: 0xd9c27a, alpha: 0.22 });
    g.circle(cx, cy, 2.1).stroke({ width: 0.1, color: 0xd9c27a, alpha: 0.9 });
    g.circle(cx, cy, 0.7).stroke({ width: 0.08, color: 0xf0e0a0 });
  }

  private scale(): number {
    return BASE_PX * this.cam.zoom;
  }

  worldX(sx: number): number {
    return (sx - this.world.position.x) / this.scale();
  }

  worldY(sy: number): number {
    return (sy - this.world.position.y) / this.scale();
  }

  zoomAt(sx: number, sy: number, factor: number): void {
    const beforeX = this.worldX(sx);
    const beforeY = this.worldY(sy);
    this.cam.zoom = Math.min(4, Math.max(0.4, this.cam.zoom * factor));
    this.applyCamera();
    this.cam.x += beforeX - this.worldX(sx);
    this.cam.y += beforeY - this.worldY(sy);
    this.applyCamera();
  }

  /** Zoom that keeps the followed ant (screen center) fixed. */
  zoomAtCenter(factor: number): void {
    const screen = this.app.renderer.screen;
    this.zoomAt(screen.width / 2, screen.height / 2, factor);
  }

  pan(dxPx: number, dyPx: number): void {
    const s = this.scale();
    this.cam.x -= dxPx / s;
    this.cam.y -= dyPx / s;
    this.applyCamera();
  }

  /** Center the camera on a world position (used when following an ant). */
  centerOn(wx: number, wy: number): void {
    this.cam.x = wx;
    this.cam.y = wy;
    this.applyCamera();
  }

  panContinuous(dt: number, keys: Set<string>): void {
    let dx = 0;
    let dy = 0;
    if (keys.has('KeyW') || keys.has('ArrowUp')) dy -= 1;
    if (keys.has('KeyS') || keys.has('ArrowDown')) dy += 1;
    if (keys.has('KeyA') || keys.has('ArrowLeft')) dx -= 1;
    if (keys.has('KeyD') || keys.has('ArrowRight')) dx += 1;
    if (dx !== 0 || dy !== 0) {
      const speed = 16 / this.cam.zoom;
      this.cam.x += dx * speed * dt;
      this.cam.y += dy * speed * dt;
      this.applyCamera();
    }
  }

  private applyCamera(): void {
    const s = this.scale();
    this.cam.x = Math.min(this.sim.w, Math.max(0, this.cam.x));
    this.cam.y = Math.min(this.sim.h, Math.max(0, this.cam.y));
    this.world.scale.set(s);
    const screen = this.app.renderer.screen;
    this.world.position.set(screen.width / 2 - this.cam.x * s, screen.height / 2 - this.cam.y * s);
  }

  private drawEntity(g: Graphics, s: Ent, team: number): void {
    const pal = TEAM_COLORS[team] ?? TEAM_COLORS[0];
    g.clear();
    if (s.kind === 'queen') {
      if (s.activity === 'flying') {
        // wings while the founding queen is airborne
        g.moveTo(-0.1, -0.1).lineTo(-0.55, -0.5).stroke({ width: 0.09, color: 0xd8cfc0, alpha: 0.55 });
        g.moveTo(0.1, -0.1).lineTo(0.55, -0.5).stroke({ width: 0.09, color: 0xd8cfc0, alpha: 0.55 });
      }
      g.circle(0, 0, 0.5).fill(pal[0]);
      g.circle(0, -0.45, 0.22).fill(pal[1]);
      if (s.carry.t === 'dirt') {
        // excavated dirt hauled by the founding queen
        g.circle(0.3, 0.12, 0.13).fill(0x8a6d4a);
      }
    } else if (s.kind === 'worker') {
      g.ellipse(0, 0, 0.34, 0.24).fill(pal[2]);
      g.circle(0, -0.26, 0.14).fill(pal[3]);
      if (s.carry.t === 'food') {
        g.circle(0.2, 0.05, 0.13).fill(RES_COLORS[s.carry.food] ?? 0x3fa34d);
      }
    } else if (s.kind === 'soldier') {
      g.ellipse(0, 0, 0.4, 0.3).fill(pal[4]);
      g.circle(0, -0.32, 0.18).fill(pal[5]);
      if (s.carry.t === 'food') {
        g.circle(0.24, 0.06, 0.14).fill(RES_COLORS[s.carry.food] ?? 0x3fa34d);
      }
    } else if (s.kind === 'food') {
      const r = 0.18 + 0.1 * Math.min(1, s.amount / 6);
      g.circle(0, 0, r).fill(RES_COLORS[s.food] ?? 0x3fa34d);
    } else if (s.kind === 'source') {
      drawSource(g, s);
    } else if (s.kind === 'egg') {
      const soldier = s.caste === 'soldier';
      g.ellipse(0, 0, soldier ? 0.19 : 0.16, soldier ? 0.28 : 0.24).fill(soldier ? 0xbfd0e8 : 0xe8dcc8);
    } else if (s.kind === 'spider') {
      g.moveTo(-0.35, -0.1).lineTo(-0.85, -0.4).stroke({ width: 0.07, color: 0x23232e });
      g.moveTo(-0.32, 0.12).lineTo(-0.8, 0.45).stroke({ width: 0.07, color: 0x23232e });
      g.moveTo(0.35, -0.1).lineTo(0.85, -0.4).stroke({ width: 0.07, color: 0x23232e });
      g.moveTo(0.32, 0.12).lineTo(0.8, 0.45).stroke({ width: 0.07, color: 0x23232e });
      g.ellipse(0, 0.08, 0.42, 0.32).fill(0x14141c);
      g.circle(0, -0.28, 0.22).fill(0x1d1d28);
      g.circle(-0.08, -0.32, 0.05).fill(0xb03a3a);
      g.circle(0.08, -0.32, 0.05).fill(0xb03a3a);
    }
    if (isAnt(s) && s.carry.t === 'egg') {
      // carried egg rides along
      g.circle(0.3, 0.14, 0.12).fill(0xe8dcc8);
    }
    if ((isAnt(s) || s.kind === 'spider') && s.hp < 0.98) {
      g.rect(-0.4, -0.62, 0.8, 0.1).fill(0x30100e);
      g.rect(-0.4, -0.62, 0.8 * Math.max(0, s.hp), 0.1).fill(0x3fbf4f);
    }
  }

  renderEntities(alpha: number, playerAnt: number | null): void {
    for (const [id, e] of this.sprites) {
      if (!this.sim.cur.has(id)) {
        this.entities.removeChild(e.c);
        e.c.destroy({ children: true });
        this.sprites.delete(id);
      }
    }
    const layer = this.activeLayer;
    for (const s of this.sim.cur.values()) {
      let e = this.sprites.get(s.id);
      if (!e) {
        const c = new Container();
        const g = new Graphics();
        const t = makeLabel(labelText(s));
        t.position.set(0, -0.85);
        c.addChild(g, t);
        e = { c, g, t, kind: '', carrying: false, hpBucket: -1, label: '', flying: false, haul: 'none', team: -1 };
        this.sprites.set(s.id, e);
        this.entities.addChild(c);
      }
      // carried eggs ride their carrier — draw the dot there instead
      if (s.kind === 'egg' && s.carried) {
        e.c.visible = false;
        continue;
      }
      const carrying = isAnt(s) && s.carry.t !== 'none';
      const hpBucket = Math.floor((isAnt(s) || s.kind === 'spider' ? s.hp : 1) * 8);
      const label = labelText(s);
      const flying = isAnt(s) && s.activity === 'flying';
      const haul = isAnt(s) ? haulLabel(s.carry) : 'none';
      const team = this.sim.team();
      if (
        e.kind !== s.kind ||
        e.carrying !== carrying ||
        e.hpBucket !== hpBucket ||
        e.flying !== flying ||
        e.haul !== haul ||
        e.team !== team
      ) {
        e.kind = s.kind;
        e.carrying = carrying;
        e.hpBucket = hpBucket;
        e.flying = flying;
        e.haul = haul;
        e.team = team;
        this.drawEntity(e.g, s, team);
      }
      if (e.label !== label) {
        e.label = label;
        e.t.text = label;
      }
      const p = this.sim.prev.get(s.id) ?? s;
      const pos = lerpPos(p, s, Math.min(1, Math.max(0, alpha)));
      e.c.position.set(pos.x, pos.y);
      e.c.visible = s.layer === layer;
      if (s.kind === 'worker' && s.activity === 'digging') {
        e.g.rotation = Math.sin(s.x * 7 + s.y * 3) * 0.4;
      } else {
        e.g.rotation = 0;
      }
    }
    this.ring.clear();
    if (playerAnt !== null) {
      const s = this.sim.cur.get(playerAnt);
      if (s && s.layer === this.activeLayer) {
        const p = this.sim.prev.get(playerAnt) ?? s;
        const pos = lerpPos(p, s, Math.min(1, Math.max(0, alpha)));
        this.ring
          .circle(pos.x, pos.y, 0.5)
          .stroke({ width: 0.06, color: 0xd9c27a });
      }
    }
  }

  pixelAt(worldX: number, worldY: number): number[] {
    const s = this.scale();
    const res = this.app.renderer.resolution;
    const sx = Math.round((this.world.position.x + worldX * s) * res);
    const sy = Math.round((this.world.position.y + worldY * s) * res);
    const c = document.createElement('canvas');
    c.width = 1;
    c.height = 1;
    const ctx = c.getContext('2d');
    if (!ctx) return [-1, -1, -1, -1];
    ctx.drawImage(this.app.canvas, sx, sy, 1, 1, 0, 0, 1, 1);
    const d = ctx.getImageData(0, 0, 1, 1).data;
    return [d[0], d[1], d[2], d[3]];
  }

  reset(sim: Sim): void {
    this.sim = sim;
    for (const [, e] of this.sprites) {
      this.entities.removeChild(e.c);
      e.c.destroy({ children: true });
    }
    this.sprites.clear();
    this.drawTiles(0);
    this.drawTiles(1);
    if (sim.foundingMode) {
      this.setActiveLayer(0);
      const q = sim.queenId();
      const qs = q !== null ? sim.cur.get(q) : undefined;
      this.cam.x = qs?.x ?? sim.w / 2;
      this.cam.y = qs?.y ?? sim.h / 2;
    } else {
      this.setActiveLayer(1);
      this.cam.x = sim.entrance?.[0] ?? sim.w / 2;
      this.cam.y = (sim.entrance?.[1] ?? 0) + 3;
    }
    this.refreshEntrance();
    this.cam.zoom = 1;
    this.applyCamera();
  }
}
