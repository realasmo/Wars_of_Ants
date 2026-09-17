import type { Renderer } from './render';

export class Input {
  keys = new Set<string>();
  onCommand: (x: number, y: number, button: number) => void = () => {};
  onCycleAnt: () => void = () => {};
  onToggleLayer: () => void = () => {};
  private renderer: Renderer;
  private down = false;
  private dragged = false;
  private originX = 0;
  private originY = 0;
  private lastX = 0;
  private lastY = 0;

  constructor(renderer: Renderer) {
    this.renderer = renderer;
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
      if (this.dragged) return;
      if (e.target !== canvas) return;
      this.onCommand(this.renderer.worldX(e.clientX), this.renderer.worldY(e.clientY), e.button);
    });

    canvas.addEventListener(
      'wheel',
      (e) => {
        e.preventDefault();
        const factor = Math.pow(1.15, -e.deltaY / 100);
        this.renderer.zoomAt(e.clientX, e.clientY, factor);
      },
      { passive: false },
    );

    canvas.addEventListener('contextmenu', (e) => e.preventDefault());

    window.addEventListener('keydown', (e) => {
      if (e.repeat) return;
      this.keys.add(e.code);
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
}
