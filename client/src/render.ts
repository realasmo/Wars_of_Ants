import { Application, Container, Graphics } from 'pixi.js';
import type { Sim, Snap } from './sim';

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

interface EntityGfx {
  g: Graphics;
  kind: number;
  carrying: boolean;
}

export class Renderer {
  readonly app: Application;
  private world = new Container();
  private layerC: Container[] = [new Container(), new Container()];
  private tileG: Graphics[] = [new Graphics(), new Graphics()];
  private entities = new Container();
  private sprites = new Map<number, EntityGfx>();
  private ring = new Graphics();
  private sim: Sim;
  cam = { x: 48, y: 8, zoom: 1 };
  activeLayer = 1;

  private constructor(sim: Sim, app: Application) {
    this.sim = sim;
    this.app = app;
  }

  static async create(sim: Sim, host: HTMLElement): Promise<Renderer> {
    const app = new Application();
    await app.init({
      background: 0x0b0a08,
      antialias: true,
      resolution: window.devicePixelRatio || 1,
      autoDensity: true,
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
    r.setActiveLayer(1);
    r.cam.x = sim.entrance[0];
    r.cam.y = sim.entrance[1] + 3;
    r.applyCamera();
    return r;
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
        g.rect(x, y, 1, 1).fill(pal[t[y * this.sim.w + x]]);
      }
    }
    const [ex, ey] = this.sim.entrance;
    g.circle(ex + 0.5, ey + 0.5, 0.42).stroke({ width: 0.07, color: 0xd9c27a });
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

  pan(dxPx: number, dyPx: number): void {
    const s = this.scale();
    this.cam.x -= dxPx / s;
    this.cam.y -= dyPx / s;
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

  private drawEntity(g: Graphics, s: Snap): void {
    g.clear();
    if (s.kind === 0) {
      g.circle(0, 0, 0.5).fill(0x8e2f3c);
      g.circle(0, -0.45, 0.22).fill(0x5e1e27);
    } else if (s.kind === 1) {
      g.ellipse(0, 0, 0.34, 0.24).fill(0x9c6b3c);
      g.circle(0, -0.26, 0.14).fill(0x6e4826);
      if (s.extra > 0.5) g.circle(0.2, 0.05, 0.13).fill(0x3fa34d);
    } else if (s.kind === 2) {
      const r = 0.18 + 0.14 * Math.min(1, s.extra / 45);
      g.circle(0, 0, r).fill(0x3fa34d);
    } else if (s.kind === 3) {
      g.ellipse(0, 0, 0.16, 0.24).fill(0xe8dcc8);
    }
  }

  renderEntities(alpha: number, playerAnt: number | null): void {
    for (const [id, e] of this.sprites) {
      if (!this.sim.cur.has(id)) {
        this.entities.removeChild(e.g);
        e.g.destroy();
        this.sprites.delete(id);
      }
    }
    for (const s of this.sim.cur.values()) {
      let e = this.sprites.get(s.id);
      if (!e) {
        const g = new Graphics();
        e = { g, kind: -1, carrying: false };
        this.sprites.set(s.id, e);
        this.entities.addChild(g);
      }
      if (e.kind !== s.kind || e.carrying !== s.extra > 0.5) {
        e.kind = s.kind;
        e.carrying = s.extra > 0.5;
        this.drawEntity(e.g, s);
      }
      const p = this.sim.prev.get(s.id) ?? s;
      const t = Math.min(1, Math.max(0, alpha));
      e.g.position.set(p.x + (s.x - p.x) * t, p.y + (s.y - p.y) * t);
      e.g.visible = s.layer === this.activeLayer;
      if (s.kind === 1 && s.state === 2) {
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
        const t = Math.min(1, Math.max(0, alpha));
        this.ring
          .circle(p.x + (s.x - p.x) * t, p.y + (s.y - p.y) * t, 0.5)
          .stroke({ width: 0.06, color: 0xd9c27a });
      }
    }
  }

  reset(sim: Sim): void {
    this.sim = sim;
    for (const [, e] of this.sprites) {
      this.entities.removeChild(e.g);
      e.g.destroy();
    }
    this.sprites.clear();
    this.drawTiles(0);
    this.drawTiles(1);
    this.setActiveLayer(this.activeLayer);
    this.cam.x = sim.entrance[0];
    this.cam.y = sim.entrance[1] + 3;
    this.cam.zoom = 1;
    this.applyCamera();
  }
}
