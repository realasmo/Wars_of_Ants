import type { Renderer } from './render';
import type { InputLog } from './inputlog';
import { r2 } from './inputlog';

export class Input {
  keys = new Set<string>();
  onCommand: (x: number, y: number, button: number) => void = () => {};
  onCycleAnt: () => void = () => {};
  onToggleLayer: () => void = () => {};
  private renderer: Renderer;
  private log?: InputLog;
  private down = false;
  private dragged = false;
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
    });

    window.addEventListener('pointermove', (e) => {
      if (!this.down) return;
      const dx = e.clientX - this.lastX;
      const dy = e.clientY - this.lastY;
      this.lastX = e.clientX;
      this.lastY = e.clientY;
      const total = Math.abs(e.clientX - this.originX) + Math.abs(e.clientY - this.originY);
      if (total > 6) this.dragged = true;
      if (this.dragged) this.renderer.pan(dx, dy);
    });

    window.addEventListener('pointerup', (e) => {
      if (!this.down) return;
      this.down = false;
      if (this.dragged) {
        this.log?.push({
          type: 'pan',
          dx: e.clientX - this.originX,
          dy: e.clientY - this.originY,
          cam: this.camState(),
        });
        return;
      }
      if (e.target !== canvas) return;
      const wx = this.renderer.worldX(e.clientX);
      const wy = this.renderer.worldY(e.clientY);
      this.log?.push({
        type: 'click',
        button: e.button,
        sx: e.clientX,
        sy: e.clientY,
        wx: r2(wx),
        wy: r2(wy),
        cam: this.camState(),
      });
      this.onCommand(wx, wy, e.button);
    });

    canvas.addEventListener(
      'wheel',
      (e) => {
        e.preventDefault();
        const factor = Math.pow(1.15, -e.deltaY / 100);
        this.renderer.zoomAt(e.clientX, e.clientY, factor);
        this.log?.push({ type: 'wheel', factor: r2(factor), sx: e.clientX, sy: e.clientY, cam: this.camState() });
      },
      { passive: false },
    );

    canvas.addEventListener('contextmenu', (e) => e.preventDefault());

    window.addEventListener('keydown', (e) => {
      if (e.repeat) return;
      this.keys.add(e.code);
      this.log?.push({ type: 'key', code: e.code });
      if (e.code === 'Tab') {
        e.preventDefault();
        this.onToggleLayer();
      } else if (e.code === 'KeyC') {
        this.onCycleAnt();
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
