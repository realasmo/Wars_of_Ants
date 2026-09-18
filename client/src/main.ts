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
      spawn: (kind: string, x?: number, y?: number) => boolean;
      setfood: (n: number) => void;
      setsuper: (n: number) => void;
      kill: (id: number) => boolean;
      killspiders: () => void;
      pause: () => void;
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
    spawn: (kind, x, y) => game.debugSpawn(kind, x ?? game.sim.entrance[0] + 0.5, y ?? game.sim.entrance[1] + 0.5),
    setfood: (n) => game.debugSetFood(n),
    setsuper: (n) => game.debugSetSuper(n),
    kill: (id) => game.debugKill(id),
    killspiders: () => game.debugKillSpiders(),
    pause: () => game.togglePause(),
  };
}

void main();
