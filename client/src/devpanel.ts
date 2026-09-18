// F2 dev panel: pick an entity, click the map to place it, quick actions.
// Thin UI only — every mutation goes through Game -> Sim dev_* (core),
// so it is recorded in the input log and replayable like any command.

export interface DevHooks {
  onFood: () => void;
  onSuper: () => void;
  onKillSpiders: () => void;
  onPause: () => void;
  onFF: () => void;
  onPauseState: () => boolean;
}

export class DevPanel {
  private el: HTMLElement;
  private kind: string | null = null;
  private hooks: DevHooks;

  constructor(hooks: DevHooks) {
    this.hooks = hooks;
    this.el = document.getElementById('devpanel') as HTMLElement;
    this.el.querySelectorAll<HTMLButtonElement>('button[data-spawn]').forEach((b) => {
      b.addEventListener('click', () => this.setPlacement(this.kind === b.dataset.spawn ? null : (b.dataset.spawn ?? null)));
    });
    this.el.querySelector('#dev-food')?.addEventListener('click', () => hooks.onFood());
    this.el.querySelector('#dev-super')?.addEventListener('click', () => hooks.onSuper());
    this.el.querySelector('#dev-kill-spiders')?.addEventListener('click', () => hooks.onKillSpiders());
    this.el.querySelector('#dev-ff')?.addEventListener('click', () => hooks.onFF());
    this.el.querySelector('#dev-pause')?.addEventListener('click', () => {
      hooks.onPause();
      this.syncPause();
    });
  }

  toggle(): void {
    this.el.classList.toggle('hidden');
    this.syncPause();
  }

  placement(): string | null {
    return this.kind;
  }

  setPlacement(kind: string | null): void {
    this.kind = kind;
    this.el.querySelectorAll<HTMLButtonElement>('button[data-spawn]').forEach((b) => {
      b.classList.toggle('active', b.dataset.spawn === kind);
    });
  }

  private syncPause(): void {
    const btn = this.el.querySelector<HTMLButtonElement>('#dev-pause');
    if (btn) btn.textContent = this.hooks.onPauseState() ? 'resume' : 'pause';
  }
}
