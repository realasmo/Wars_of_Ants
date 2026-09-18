// Replay format v1: a seed plus sim commands stamped with the tick they fire.
// A command applies after the first sim tick that reaches its `t`.
// Produced by __woa.replay() from the input recorder, or hand-written;
// served from client/public/replays/<name>.json and loaded via ?replay=<name>.

import { coreVersion } from './wasm';

export interface ReplayCmd {
  t: number;
  act: 'move' | 'dig' | 'attack' | 'entrance';
  ant: number;
  x?: number;
  y?: number;
  tx?: number;
  ty?: number;
  target?: number;
}

export interface Replay {
  version: 1;
  seed: number;
  name?: string;
  /** Core build the replay was recorded on; a mismatch warns (outcomes drift with sim changes). */
  core?: string;
  ticks?: number;
  cmds: ReplayCmd[];
}

export async function loadReplay(name: string): Promise<Replay> {
  const r = await fetch(`/replays/${name}.json`);
  if (!r.ok) throw new Error(`replay not found: ${name} (${r.status})`);
  const replay = (await r.json()) as Replay;
  if (replay.version !== 1) throw new Error(`unsupported replay version: ${replay.version}`);
  if (replay.core !== undefined && replay.core !== coreVersion()) {
    console.warn(
      `replay "${name}" was recorded on ${replay.core} but this client runs ${coreVersion()}` +
        ' — it will not reproduce the original game',
    );
  }
  return replay;
}
