// Replay format: a seed plus sim commands stamped with the tick they fire.
// A command applies after the first sim tick that reaches its `t`.
// v2 = founding start (team recorded); v1 predates the founding update.
// Produced by __woa.replay() from the input recorder, or hand-written;
// served from client/public/replays/<name>.json and loaded via ?replay=<name>.

import { coreVersion } from './wasm';

export interface ReplayCmd {
  t: number;
  act:
    | 'move'
    | 'dig'
    | 'attack'
    | 'entrance'
    | 'land'
    | 'found'
    | 'dump'
    | 'drop'
    | 'pick-egg'
    | 'brood'
    | 'follow-all'
    | 'follow-one'
    | 'follow-soldiers'
    | 'follow-release'
    | 'dev-spawn'
    | 'dev-food'
    | 'dev-super'
    | 'dev-water'
    | 'dev-honeydew'
    | 'dev-kill'
    | 'dev-kill-spiders'
    | 'dev-soil';
  ant?: number;
  x?: number;
  y?: number;
  tx?: number;
  ty?: number;
  target?: number;
  kind?: string;
  n?: number;
  layer?: number;
  soil?: number;
}

export interface Replay {
  version: 1 | 2;
  seed: number;
  name?: string;
  /** Core build the replay was recorded on; a mismatch warns (outcomes drift with sim changes). */
  core?: string;
  /** Team color for founding sessions: 0 red, 1 blue. */
  team?: number;
  ticks?: number;
  cmds: ReplayCmd[];
}

export async function loadReplay(name: string): Promise<Replay> {
  const r = await fetch(`/replays/${name}.json`);
  if (!r.ok) throw new Error(`replay not found: ${name} (${r.status})`);
  const replay = (await r.json()) as Replay;
  if (replay.version !== 1 && replay.version !== 2) {
    throw new Error(`unsupported replay version: ${replay.version}`);
  }
  if (replay.version < 2) {
    console.warn(
      `replay "${name}" predates the founding update — it will not reproduce its original game`,
    );
  }
  if (replay.core !== undefined && replay.core !== coreVersion()) {
    console.warn(
      `replay "${name}" was recorded on ${replay.core} but this client runs ${coreVersion()}` +
        ' — it will not reproduce the original game',
    );
  }
  return replay;
}
