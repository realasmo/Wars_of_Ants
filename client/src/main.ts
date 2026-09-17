import './style.css';
import { initCore } from './wasm';
import { Game } from './game';

declare global {
  interface Window {
    __woa?: {
      click: (x: number, y: number, button: number) => void;
      key: (code: string) => void;
      step: (n: number) => void;
      state: () => Record<string, unknown>;
      px: (x: number, y: number) => number[];
      log: () => Record<string, unknown>;
      mark: (label: string) => void;
    };
  }
}

async function main(): Promise<void> {
  await initCore();
  const params = new URLSearchParams(window.location.search);
  const seedParam = params.get('seed');
  const seed = seedParam !== null ? Number(seedParam) : Date.now() % 0x7fffffff;
  const host = document.getElementById('app');
  if (!host) throw new Error('#app element missing');
  const game = await Game.create(host, seed);
  game.start();
  window.__woa = {
    click: (x, y, button) => game.debugClick(x, y, button),
    key: (code) => game.debugKey(code),
    step: (n) => game.debugStep(n),
    state: () => game.debugState(),
    px: (x, y) => game.debugPixel(x, y),
    log: () => game.debugLog(),
    mark: (label) => game.debugMark(label),
  };
}

void main();
