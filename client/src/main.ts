import './style.css';
import { gameVersion, initCore } from './wasm';
import { Game } from './game';
import { loadReplay } from './replay';
import type { Replay } from './replay';
import { chooseTeam } from './menu';
import { bindConsole } from './console';

declare global {
  interface Window {
    __woa?: {
      click: (x: number, y: number, button: number) => void;
      key: (code: string) => void;
      step: (n: number) => void;
      stepTo: (target: number) => void;
      state: () => Record<string, unknown>;
      px: (x: number, y: number) => number[];
      tile: (layer: number, x: number, y: number) => number;
      soil: (layer: number, x: number, y: number) => number;
      setsoil: (layer: number, x: number, y: number, soil: number) => void;
      log: () => Record<string, unknown>;
      mark: (label: string) => void;
      replay: () => Record<string, unknown>;
      canon: () => string;
      spawn: (kind: string, x?: number, y?: number) => boolean;
      setfood: (n: number) => void;
      setsuper: (n: number) => void;
      setwater: (n: number) => void;
      kill: (id: number) => boolean;
      killspiders: () => void;
      pause: () => void;
      gait: (id: number) => Record<string, unknown> | null;
      squad: (mode: number) => boolean;
      brood: (code: number) => string;
      parts: (id: number) => Record<string, unknown> | null;
      rules: () => Record<string, unknown>;
      rulesGet: () => Record<string, unknown>;
      rulesSet: (json: string) => Record<string, unknown>;
      rulesRestart: (json: string) => void;
      icons: () => Record<string, unknown>;
    };
  }
}

async function main(): Promise<void> {
  await initCore();
  // game version in the menu and the in-game corner (single source: core)
  const ver = gameVersion();
  const mv = document.getElementById('menu-version');
  if (mv !== null) mv.textContent = ver;
  const cv = document.getElementById('version');
  if (cv !== null) cv.textContent = ver;
  const params = new URLSearchParams(window.location.search);
  const replayName = params.get('replay');
  let replay: Replay | null = null;
  if (replayName !== null) replay = await loadReplay(replayName);
  const seedParam = params.get('seed');
  const host = document.getElementById('app');
  if (!host) throw new Error('#app element missing');

  // replays go straight to the game; a fresh run opens at the team menu
  let game: Game;
  if (replay !== null) {
    game = await Game.create(host, replay.seed, replay);
  } else {
    const seed = seedParam !== null ? Number(seedParam) : Date.now() % 0x7fffffff;
    const team = await chooseTeam();
    game = await Game.create(host, seed, null, team);
  }
  game.start();
  bindConsole(game);
  if (params.get('perf') !== null) game.togglePerf();
  // the admin rules drawer opens itself with ?admin=1 (F4 toggles it too)
  if (params.get('admin') !== null) game.admin.show();
  // death overlay → team menu → brand-new founding game
  game.onToMenu = () => {
    void chooseTeam().then((team) => game.restartFounding(team));
  };
  window.__woa = {
    click: (x, y, button) => game.debugClick(x, y, button),
    key: (code) => game.debugKey(code),
    step: (n) => game.debugStep(n),
    stepTo: (target) => game.debugStepTo(target),
    state: () => game.debugState(),
    px: (x, y) => game.debugPixel(x, y),
    tile: (layer, x, y) => game.debugTile(layer, x, y),
    soil: (layer, x, y) => game.debugSoil(layer, x, y),
    setsoil: (layer, x, y, soil) => game.debugSetSoil(layer, x, y, soil),
    log: () => game.debugLog(),
    mark: (label) => game.debugMark(label),
    replay: () => game.debugReplay(),
    canon: () => game.debugCanon(),
    spawn: (kind, x, y) => {
      const ent = game.sim.entrance;
      return game.debugSpawn(
        kind,
        x ?? (ent !== null ? ent[0] + 0.5 : game.sim.w / 2),
        y ?? (ent !== null ? ent[1] + 0.5 : game.sim.h / 2),
      );
    },
    setfood: (n) => game.debugSetFood(n),
    setsuper: (n) => game.debugSetSuper(n),
    setwater: (n) => game.debugSetWater(n),
    kill: (id) => game.debugKill(id),
    killspiders: () => game.debugKillSpiders(),
    pause: () => game.togglePause(),
    gait: (id) => game.debugAnt(id),
    squad: (mode) => game.debugSquad(mode),
    brood: (code) => game.debugBrood(code),
    parts: (id) => game.debugAntParts(id),
    rules: () => game.debugRules(),
    rulesGet: () => game.sim.rulesGet() as Record<string, unknown>,
    rulesSet: (json) => game.simRulesSet(json) as Record<string, unknown>,
    rulesRestart: (json) => game.restartWithRules(json),
    icons: () => game.debugIcons(),
  };
}

void main();
