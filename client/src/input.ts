import type { Renderer } from './render';
import type { InputLog } from './inputlog';
import { r2 } from './inputlog';

export class Input {
  static readonly NO_KEYS = new Set<string>();
  keys = new Set<string>();
  /** LMB drag pans the camera only while spectating (no controlled ant);
   *  while controlling, LMB is steering. */
  panEnabled = false;
  /** Left press on something interactive (ant to select, spider to attack,
   *  dev placement). Returns true when consumed — steering doesn't start. */
  onInteract: (x: number, y: number) => boolean = () => false;
  /** Hold-to-steer: the controlled ant walks toward the cursor. */
  onSteer: (x: number, y: number, phase: 'start' | 'move' | 'end') => void = () => {};
  /** Context command (right-click: dig / entrance / fallback move). */
  onCommand: (x: number, y: number, button: number) => void = () => {};
  onCycleAnt: () => void = () => {};
  onToggleLayer: () => void = () => {};
  onToggleDev: () => void = () => {};
  onSquadKey: (code: string) => void = () => {};
  onTogglePerf: () => void = () => {};
  /** F4: the admin rules drawer (created lazily by the handler). */
  onToggleAdmin: () => void = () => {};
  onEscape: () => void = () => {};
  /** Wheel zoom; anchored at the followed ant while controlling. */
  onZoom: (factor: number, sx: number, sy: number) => void = () => {};
  private renderer: Renderer;
  private log?: InputLog;
  private down = false;
  private dragged = false;
  private steering = false;
  private originX = 0;
  private originY = 0;
  private lastX = 0;
  private lastY = 0;

  constructor(renderer: Renderer, log?: InputLog) {
    this.renderer = renderer;
    this.log = log;
    const canvas = renderer.app.canvas;

    canvas.addEventListener('pointerdown', (e) => {
      this.down = true;
      this.dragged = false;
      this.originX = e.clientX;
      this.originY = e.clientY;
      this.lastX = e.clientX;
      this.lastY = e.clientY;
      if (e.button !== 0) return;
      const wx = this.renderer.worldX(e.clientX);
      const wy = this.renderer.worldY(e.clientY);
      // interactive targets first (dev placement works in any mode; ant
      // select / spider attack need a controlled ant — decided in Game)
      if (this.onInteract(wx, wy)) return;
      if (!this.panEnabled) {
        this.steering = true;
        this.onSteer(wx, wy, 'start');
        this.log?.push({ type: 'steer', phase: 'start', wx: r2(wx), wy: r2(wy), cam: this.camState() });
      }
    });

    window.addEventListener('pointermove', (e) => {
      if (!this.down) return;
      const dx = e.clientX - this.lastX;
      const dy = e.clientY - this.lastY;
      this.lastX = e.clientX;
      this.lastY = e.clientY;
      const total = Math.abs(e.clientX - this.originX) + Math.abs(e.clientY - this.originY);
      if (total > 6) this.dragged = true;
      if (this.steering) {
        this.onSteer(this.renderer.worldX(e.clientX), this.renderer.worldY(e.clientY), 'move');
      } else if (this.dragged && this.panEnabled) {
        this.renderer.pan(dx, dy);
      }
    });

    window.addEventListener('pointerup', (e) => {
      if (!this.down) return;
      this.down = false;
      if (this.steering) {
        this.steering = false;
        this.onSteer(this.renderer.worldX(e.clientX), this.renderer.worldY(e.clientY), 'end');
        this.log?.push({ type: 'steer', phase: 'end', wx: r2(this.renderer.worldX(e.clientX)), wy: r2(this.renderer.worldY(e.clientY)), cam: this.camState() });
        return;
      }
      if (this.dragged) {
        if (this.panEnabled) {
          this.log?.push({ type: 'pan', dx: e.clientX - this.originX, dy: e.clientY - this.originY, cam: this.camState() });
        }
        return;
      }
      if (e.target !== canvas) return;
      if (e.button === 2) {
        const wx = this.renderer.worldX(e.clientX);
        const wy = this.renderer.worldY(e.clientY);
        this.log?.push({ type: 'click', button: 2, sx: e.clientX, sy: e.clientY, wx: r2(wx), wy: r2(wy), cam: this.camState() });
        this.onCommand(wx, wy, 2);
      }
    });

    canvas.addEventListener(
      'wheel',
      (e) => {
        e.preventDefault();
        const factor = Math.pow(1.15, -e.deltaY / 100);
        this.onZoom(factor, e.clientX, e.clientY);
        this.log?.push({ type: 'wheel', factor: r2(factor), sx: e.clientX, sy: e.clientY, cam: this.camState() });
      },
      { passive: false },
    );

    canvas.addEventListener('contextmenu', (e) => e.preventDefault());

    window.addEventListener('keydown', (e) => {
      if (document.getElementById('console')?.classList.contains('hidden') === false) {
        return; // dev console is open: game keys (except `) are captured there
      }
      if (e.repeat) return;
      this.keys.add(e.code);
      this.log?.push({ type: 'key', code: e.code });
      if (e.code === 'Tab') {
        e.preventDefault();
        this.onToggleLayer();
      } else if (e.code === 'KeyC') {
        this.onCycleAnt();
      } else if (e.code === 'F2') {
        e.preventDefault();
        this.onToggleDev();
      } else if (e.code === 'F3') {
        e.preventDefault();
        this.onTogglePerf();
      } else if (e.code === 'F4') {
        e.preventDefault();
        this.onToggleAdmin();
      } else if (e.code === 'KeyX') {
        this.onSquadKey('KeyX');
      } else if (e.code === 'Escape') {
        this.onEscape();
      } else if (
        (e.code === 'Digit1' || e.code === 'Digit2' || e.code === 'Digit3' || e.code === 'Digit4') &&
        // digits serve whichever X-menu is open (squad or the queen's brood)
        (!document.getElementById('squadmenu')?.classList.contains('hidden') ||
          !document.getElementById('broodmenu')?.classList.contains('hidden'))
      ) {
        this.onSquadKey(e.code);
      }
    });

    window.addEventListener('keyup', (e) => this.keys.delete(e.code));
    window.addEventListener('blur', () => this.keys.clear());
  }

  private camState(): number[] {
    const c = this.renderer.cam;
    return [r2(c.x), r2(c.y), r2(c.zoom)];
  }
}
