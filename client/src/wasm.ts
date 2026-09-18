import init, { WoaSim, core_version } from '../../core/pkg/woa_core.js';

let ready = false;

export async function initCore(): Promise<void> {
  if (!ready) {
    await init();
    ready = true;
  }
}

export { WoaSim };

export function coreVersion(): string {
  return core_version();
}
