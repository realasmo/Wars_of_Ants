import './style.css';
import { initCore } from './wasm';
import { Game } from './game';
import { loadReplay } from './replay';
import type { Replay } from './replay';

declare global {
  interface Window {
    __woa?: {
      click: (x: number, y: number, button: number) => void;
      key: (code: string) => void;
      step: (n: number) => void;
      stepTo: (target: number) => void;
      state: () => Record<string, unknown>;
      px: (x: number, y: number) => number[];
      log: () => Record<string, unknown>;
      mark: (label: string) => void;
      replay: () => Record<string, unknown>;
      canon: () => string;
    };
  }
}

async function main(): Promise<void> {
  await initCore();
  const params = new URLSearchParams(window.location.search);
  const replayName = params.get('replay');
  let replay: Replay | null = null;
  if (replayName !== null) replay = await loadReplay(replayName);
  const seedParam = params.get('seed');
  const seed = replay !== null ? replay.seed : seedParam !== null ? Number(seedParam) : Date.now() % 0x7fffffff;
  const host = document.getElementById('app');
  if (!host) throw new Error('#app element missing');
  const game = await Game.create(host, seed, replay);
  game.start();
  window.__woa = {
    click: (x, y, button) => game.debugClick(x, y, button),
    key: (code) => game.debugKey(code),
    step: (n) => game.debugStep(n),
    stepTo: (target) => game.debugStepTo(target),
    state: () => game.debugState(),
    px: (x, y) => game.debugPixel(x, y),
    log: () => game.debugLog(),
    mark: (label) => game.debugMark(label),
    replay: () => game.debugReplay(),
    canon: () => game.debugCanon(),
  };
}

void main();
