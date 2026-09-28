// The admin tuning drawer (?admin=1 or F4): a Tweakpane form generated
// entirely from the core's field registry (rules_meta) — groups, per-field
// live/new-game scope badges, nested shapes. Commits are whole-object and
// atomic: the core validates everything and either applies the full ruleset
// or lists every violation. Every successful commit is logged as a
// replayable command; "apply & new game" restarts under the edited rules
// with the SAME seed so worldgen tweaks are comparable.

import { Pane } from 'tweakpane';
import type { FolderApi } from 'tweakpane';
import type { Game } from './game';
import type { RulesMeta } from './sim';

/** Label suffix marking fields that only take effect on a new game (world
 * generation consumes them before the first tick). Live fields get none. */
const NG_SUFFIX = ' ◇ng';

function stepFor(v: number): number {
  if (Number.isInteger(v)) return 1;
  if (Math.abs(v) < 1) return 0.01;
  if (Math.abs(v) < 10) return 0.1;
  return 1;
}

export class AdminPanel {
  private el: HTMLElement;
  private paneHost: HTMLElement;
  private pane: Pane | null = null;
  private digestEl: HTMLElement;
  private errorsEl: HTMLElement;
  private jsonEl: HTMLTextAreaElement;
  private dirtyEl: HTMLElement;
  private game: Game;
  /** The draft rules object in edit shape (sources carry min/max instead of
   * the wire's [min,max] tuple; the craving cycle is a proxy text field). */
  private draft: Record<string, unknown> = {};
  private committedJson = '';
  private digest = '';
  private cycleProxy = { text: '' };
  private open = false;

  constructor(game: Game) {
    this.game = game;
    this.el = document.getElementById('admin') as HTMLElement;
    this.paneHost = document.getElementById('admin-pane') as HTMLElement;
    this.digestEl = document.getElementById('admin-digest') as HTMLElement;
    this.errorsEl = document.getElementById('admin-errors') as HTMLElement;
    this.dirtyEl = document.getElementById('admin-dirty') as HTMLElement;
    this.jsonEl = document.getElementById('admin-json') as HTMLTextAreaElement;
    document.getElementById('admin-apply')?.addEventListener('click', () => this.applyLive());
    document.getElementById('admin-restart')?.addEventListener('click', () => this.applyRestart());
    document.getElementById('admin-default')?.addEventListener('click', () => this.resetDefaults());
    document.getElementById('admin-json-toggle')?.addEventListener('click', () => {
      this.jsonEl.classList.toggle('hidden');
      if (!this.jsonEl.classList.contains('hidden')) this.jsonEl.value = this.buildWire();
    });
    document.getElementById('admin-json-load')?.addEventListener('click', () => this.loadJson());
    // the panel survives restarts: every access goes through game.sim fresh
  }

  /** Open (and build) the drawer. Safe to call repeatedly. */
  show(): void {
    this.open = true;
    this.el.classList.remove('hidden');
    this.refresh();
  }

  toggle(): void {
    if (this.open) {
      this.open = false;
      this.el.classList.add('hidden');
    } else {
      this.show();
    }
  }

  isOpen(): boolean {
    return this.open;
  }

  /** Pull the current rules from the sim and rebuild the form. */
  private refresh(): void {
    const got = this.game.sim.rulesGet();
    this.draft = got.rules as Record<string, unknown>;
    for (const s of this.draft.sources as Record<string, unknown>[]) {
      const [min, max] = s.amount as [number, number];
      s.min = min;
      s.max = max;
      delete s.amount;
    }
    this.cycleProxy.text = (this.draft.craving_cycle as string[]).join(',');
    this.digest = got.digest;
    this.committedJson = this.buildWire();
    this.rebuildPane();
    this.syncStatus();
  }

  /** The wire-shape JSON for the current draft (what the core will parse). */
  private buildWire(): string {
    const r = JSON.parse(JSON.stringify(this.draft)) as Record<string, unknown>;
    for (const s of r.sources as Record<string, unknown>[]) {
      s.amount = [s.min, s.max];
      delete s.min;
      delete s.max;
    }
    r.craving_cycle = this.cycleProxy.text
      .split(',')
      .map((t) => t.trim())
      .filter((t) => t.length > 0);
    return JSON.stringify(r);
  }

  private rebuildPane(): void {
    this.pane?.dispose();
    this.pane = new Pane({ container: this.paneHost, title: 'GameRules' });
    const pane = this.pane;
    const meta: RulesMeta = this.game.sim.rulesMeta();
    const scopeOf = (field: string): string =>
      meta.fields[field]?.scope === 'new_game' ? NG_SUFFIX : '';

    for (const [gi, group] of meta.groups.entries()) {
      // the first group opens so the form doesn't read as an empty shell
      const f = pane.addFolder({ title: group.name, expanded: gi === 0 });
      for (const field of group.fields) {
        const suffix = scopeOf(field);
        if (field === 'sources') {
          this.addSourcesEditor(f, meta);
        } else if (field === 'brood') {
          this.addBroodEditor(f, meta);
        } else if (meta.unit_groups.includes(field)) {
          const uf = f.addFolder({ title: field });
          const stats = this.draft[field] as Record<string, number>;
          for (const leaf of meta.unit_fields) {
            uf.addBinding(stats, leaf, { label: leaf + suffix, step: stepFor(stats[leaf]) });
          }
        } else if (field === 'craving_cycle') {
          f.addBinding(this.cycleProxy, 'text', { label: 'craving_cycle' + suffix });
        } else {
          const v = this.draft[field];
          if (typeof v === 'number') {
            f.addBinding(this.draft, field, { label: field + suffix, step: stepFor(v) });
          } else {
            f.addBinding(this.draft, field, { label: field + suffix });
          }
        }
      }
    }
    pane.on('change', () => this.syncStatus());
  }

  private addSourcesEditor(f: FolderApi, meta: RulesMeta): void {
    const sources = this.draft.sources as Record<string, unknown>[];
    sources.forEach((s, i) => {
      const sf = f.addFolder({ title: `#${i + 1} · ${String(s.kind)}` });
      const kindOptions: Record<string, string> = {};
      for (const k of meta.food_kinds) kindOptions[k] = k;
      sf.addBinding(s, 'kind', { label: 'kind', options: kindOptions });
      for (const leaf of ['src', 'min', 'max', 'harvest', 'count']) {
        const v = s[leaf] as number;
        sf.addBinding(s, leaf, { label: leaf + NG_SUFFIX, step: stepFor(v) });
      }
    });
  }

  private addBroodEditor(f: FolderApi, meta: RulesMeta): void {
    const brood = this.draft.brood as Record<string, Record<string, number | boolean>>;
    for (const caste of meta.brood_castes) {
      const cf = f.addFolder({ title: caste });
      const costs = brood[caste];
      for (const leaf of meta.brood_fields) {
        const v = costs[leaf];
        if (typeof v === 'boolean') {
          cf.addBinding(costs, leaf, { label: leaf });
        } else {
          cf.addBinding(costs, leaf, { label: leaf, step: stepFor(v) });
        }
      }
    }
  }

  private syncStatus(): void {
    this.digestEl.textContent = this.digest;
    const dirty = this.buildWire() !== this.committedJson;
    this.dirtyEl.textContent = dirty ? '● uncommitted changes' : '';
    this.dirtyEl.classList.toggle('dirty', dirty);
  }

  private showErrors(errs: string[] | null): void {
    if (errs === null || errs.length === 0) {
      this.errorsEl.classList.add('hidden');
      this.errorsEl.textContent = '';
    } else {
      this.errorsEl.classList.remove('hidden');
      this.errorsEl.textContent = errs.map((e) => `· ${e}`).join('\n');
    }
  }

  /** Commit the draft to the running sim (live fields apply immediately). */
  private applyLive(): void {
    const res = this.game.simRulesSet(this.buildWire());
    if (res.ok) {
      this.showErrors(null);
      this.refresh();
    } else {
      this.showErrors(res.errors ?? ['unknown error']);
    }
  }

  /** Commit, then restart the game under the edited rules (same seed, so
   * worldgen tweaks are apples-to-apples). */
  private applyRestart(): void {
    const json = this.buildWire();
    const res = this.game.simRulesSet(json);
    if (!res.ok) {
      this.showErrors(res.errors ?? ['unknown error']);
      return;
    }
    this.showErrors(null);
    this.game.restartWithRules(json);
    this.refresh();
  }

  private resetDefaults(): void {
    const defaults = this.game.sim.rulesDefault();
    this.draft = defaults.rules as Record<string, unknown>;
    for (const s of this.draft.sources as Record<string, unknown>[]) {
      const [min, max] = s.amount as [number, number];
      s.min = min;
      s.max = max;
      delete s.amount;
    }
    this.cycleProxy.text = (this.draft.craving_cycle as string[]).join(',');
    this.digest = `${defaults.digest} (defaults, not applied)`;
    this.rebuildPane();
    this.syncStatus();
  }

  /** Replace the draft with JSON pasted into the textarea, then rebuild. */
  private loadJson(): void {
    try {
      const parsed = JSON.parse(this.jsonEl.value) as Record<string, unknown>;
      this.draft = parsed;
      for (const s of this.draft.sources as Record<string, unknown>[]) {
        const [min, max] = s.amount as [number, number];
        s.min = min;
        s.max = max;
        delete s.amount;
      }
      this.cycleProxy.text = (this.draft.craving_cycle as string[]).join(',');
      this.rebuildPane();
      this.syncStatus();
      this.showErrors(null);
    } catch (e) {
      this.showErrors([`JSON: ${String(e)}`]);
    }
  }

  /** e2e/debug: the panel's current wire JSON + digest + dirty flag. */
  debugState(): Record<string, unknown> {
    return { open: this.open, digest: this.digest, dirty: this.buildWire() !== this.committedJson };
  }
}
