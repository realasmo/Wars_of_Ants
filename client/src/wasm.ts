import init, { WoaSim, core_version, game_version, snapshot_spec } from '../../core/pkg/woa_core.js';
import { assertWireSpec } from './decode';

let ready = false;

export async function initCore(): Promise<void> {
  if (!ready) {
    await init();
    // fail loudly before the first frame if this decoder no longer matches
    // the packed core sitting next to it
    assertWireSpec(snapshot_spec());
    ready = true;
  }
}

export { WoaSim };

export function coreVersion(): string {
  return core_version();
}

export function gameVersion(): string {
  return game_version();
}
