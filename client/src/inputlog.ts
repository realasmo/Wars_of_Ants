// Ring buffer of discrete input/command events, stamped with the sim tick.
// Dev tool: dumped via window.__woa.log() to reconstruct what the player did
// (a dump maps 1:1 onto __woa.click/key calls, so it can become an e2e scenario).

export interface InputEvent {
  seq: number;
  t: number; // ms since page load (performance.now())
  tick: number;
  type: string; // start | click | key | wheel | pan | view | cmd | death | restart | mark
  [key: string]: unknown;
}

const CAP = 500;

export const r2 = (v: number): number => Math.round(v * 100) / 100;

export class InputLog {
  private events: InputEvent[] = [];
  private next = 0;
  private ctx: () => number = () => 0; // current sim tick
  private sink: HTMLElement | null = null; // hidden DOM mirror, readable without JS eval
  dropped = 0;

  setTickSource(fn: () => number): void {
    this.ctx = fn;
  }

  setSink(el: HTMLElement | null): void {
    this.sink = el;
  }

  push(e: { type: string } & Record<string, unknown>): void {
    this.events.push({ seq: this.next++, t: Math.round(performance.now()), tick: this.ctx(), ...e });
    if (this.events.length > CAP) {
      this.dropped += this.events.length - CAP;
      this.events.splice(0, this.events.length - CAP);
    }
    if (this.sink) this.sink.textContent = JSON.stringify(this.dump());
  }

  dump(): { count: number; dropped: number; events: InputEvent[] } {
    return { count: this.events.length, dropped: this.dropped, events: [...this.events] };
  }
}
