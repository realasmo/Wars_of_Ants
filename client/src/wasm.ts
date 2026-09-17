import init, { WoaSim } from '../../core/pkg/woa_core.js';

let ready = false;

export async function initCore(): Promise<void> {
  if (!ready) {
    await init();
    ready = true;
  }
}

export { WoaSim };
