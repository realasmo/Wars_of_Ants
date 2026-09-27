import { Renderer } from './render';
import { Input } from './input';
import { Hud } from './hud';
import { Sim, TPS, lerpPos, isAnt, carryLabel } from './sim';
import type { AntEnt, Ent, SpiderEnt } from './sim';
import { InputLog, r2 } from './inputlog';
import { coreVersion } from './wasm';
import { DevPanel } from './devpanel';
import { consoleOpen, pollEvents } from './console';
import type { Replay } from './replay';

export class Game {
  sim: Sim;
  private renderer: Renderer;
  private input: Input;
  private hud = new Hud();
  private dev: DevPanel;
  paused = false; // public for the dev console
  private readonly log = new InputLog();
  private playerAnt: number | null = null;
  private seed: number;
  private acc = 0;
  private last = performance.now();
  private running = false;
  private hudCounter = 0;
  private deadShown = false;
  private replay: { cmds: Replay['cmds']; i: number } | null = null;
  private lastPlayerLayer: number | null = null;
  private steerTarget: { x: number; y: number } | null = null;
  private lastSteerTick = -1;
  private lastEntrance: [number, number] | null | undefined = undefined;
  /** Set by main.ts: death overlay goes back to the team menu instead of restarting in place. */
  onToMenu: (() => void) | null = null;
  private perfStart = 0;
  private perfFrames = 0;
  private perfMs = 0;
  private perfWorst = 0;
  private perfTicks = 0;

  private constructor(sim: Sim, renderer: Renderer, seed: number, replay: Replay | null) {
    this.sim = sim;
    this.renderer = renderer;
    this.seed = seed;
    this.log.setTickSource(() => this.sim.tickCount);
    this.log.setSink(document.getElementById('inputlog'));
    this.input = new Input(renderer, this.log);
    this.input.onInteract = (x, y) => this.interactAt(x, y);
    this.input.onSteer = (x, y, phase) => this.steer(x, y, phase);
    this.input.onCommand = (x, y, b) => this.handleClick(x, y, b);
    this.input.onZoom = (factor, sx, sy) => {
      if (this.playerAnt !== null && this.sim.cur.has(this.playerAnt)) this.renderer.zoomAtCenter(factor);
      else this.renderer.zoomAt(sx, sy, factor);
    };
    this.input.onCycleAnt = () => this.cycleAnt();
    this.input.onToggleLayer = () => this.toggleLayer();
    this.input.onToggleDev = () => this.dev.toggle();
    this.input.onTogglePerf = () => this.hud.togglePerf();
    this.input.onEscape = () => this.dev.setPlacement(null);
    this.input.onSquadKey = (code) => this.squadKey(code);
    this.dev = new DevPanel({
      onFood: () => this.debugSetFood(50),
      onSuper: () => this.debugSetSuper(5),
      onKillSpiders: () => this.debugKillSpiders(),
      onPause: () => this.togglePause(),
      onFF: () => this.debugStep(200),
      onPauseState: () => this.paused,
      onCoords: () => this.toggleCoords(),
    });
    document.querySelectorAll('#squadmenu .sq-btn').forEach((b) => {
      b.addEventListener('click', () => {
        this.issueFollow(Number((b as HTMLElement).dataset.sq));
      });
    });
    this.playerAnt = this.initialAnt();
    this.log.push({ type: 'start', seed, workers: sim.workers().length });
    if (replay) {
      this.replay = { cmds: [...replay.cmds].sort((a, b) => a.t - b.t), i: 0 };
      this.hud.setReplay(true);
      this.log.push({ type: 'start', note: 'replay', cmds: replay.cmds.length });
    }
  }

  /** The founding player starts as the queen; the legacy start as first worker. */
  private initialAnt(): number | null {
    if (this.sim.foundingMode) return this.sim.queenId();
    return this.sim.workers()[0] ?? null;
  }

  static async create(
    host: HTMLElement,
    seed: number,
    replay: Replay | null = null,
    team = 0,
  ): Promise<Game> {
    // v2 replays are founding sessions (team recorded); v1 replays predate
    // the founding update and no longer reproduce — loader warns
    const sim =
      replay !== null && replay.version < 2
        ? new Sim(replay.seed)
        : replay !== null
          ? new Sim(replay.seed, 3, 6, { founding: true, team: replay.team ?? 0 })
          : new Sim(seed, 3, 6, { founding: true, team });
    const renderer = await Renderer.create(sim, host);
    return new Game(sim, renderer, replay !== null ? replay.seed : seed, replay);
  }

  start(): void {
    if (this.running) return;
    this.running = true;
    requestAnimationFrame(this.frame);
  }

  get controlling(): number | null {
    return this.playerAnt;
  }

  debugClick(x: number, y: number, button: number): void {
    this.handleClick(x, y, button);
  }

  debugKey(code: string): void {
    if (code === 'Tab') this.toggleLayer();
    else if (code === 'KeyC') this.cycleAnt();
  }

  // --- squad X-menu (F3 wave) ---
  private squadOpen = false;

  private squadKey(code: string): void {
    if (code === 'KeyX') {
      // worker (or soldier) leaders only — the queen's X-menu is the F4
      // brood menu, not built yet
      const me = this.playerAnt !== null ? this.sim.ant(this.playerAnt) : undefined;
      if (me === undefined || me.kind === 'queen') return;
      this.squadOpen = !this.squadOpen;
      document.getElementById('squadmenu')?.classList.toggle('hidden', !this.squadOpen);
      return;
    }
    const map: Record<string, number> = { Digit1: 0, Digit2: 1, Digit3: 3, Digit4: 2 };
    const mode = map[code];
    if (mode === undefined) return;
    this.issueFollow(mode);
  }

  private issueFollow(mode: number): void {
    if (this.playerAnt === null || this.sim.dead || this.replay !== null) return;
    const acts = ['follow-all', 'follow-one', 'follow-soldiers', 'follow-release'];
    if (this.sim.follow(this.playerAnt, mode)) {
      this.log.push({ type: 'cmd', act: acts[mode], ant: this.playerAnt });
    }
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
    this.squadOpen = false;
    document.getElementById('squadmenu')?.classList.add('hidden');
  }

  /** e2e hook: open/click the squad menu programmatically. */
  debugSquad(mode: number): boolean {
    if (this.playerAnt === null) return false;
    const ok = this.sim.follow(this.playerAnt, mode);
    if (ok) this.log.push({ type: 'cmd', act: ['follow-all', 'follow-one', 'follow-soldiers', 'follow-release'][mode], ant: this.playerAnt });
    return ok;
  }

  debugLog(): Record<string, unknown> {
    const c = this.renderer.cam;
    return {
      seed: this.seed,
      layer: this.renderer.activeLayer === 0 ? 'surface' : 'underground',
      cam: { x: r2(c.x), y: r2(c.y), zoom: r2(c.zoom) },
      timeOrigin: performance.timeOrigin,
      ...this.log.dump(),
    };
  }

  debugMark(label: string): void {
    this.log.push({ type: 'mark', label });
  }

  seedValue(): number {
    return this.seed;
  }

  /** Console-friendly spawn: kind + optional position (defaults: nest
   * entrance, or the queen when no nest exists). */
  debugSpawnAt(kind: string, x?: number, y?: number): boolean {
    if (this.sim.dead || this.replay !== null) return false;
    let px = x;
    let py = y;
    if (px === undefined || py === undefined) {
      const ent = this.sim.entrance;
      const qid = this.sim.queenId();
      const q = qid !== null ? this.sim.cur.get(qid) : undefined;
      px = ent !== null ? ent[0] + 1 : (q ? q.x : this.sim.w / 2);
      py = ent !== null ? ent[1] + 3 : (q ? q.y : this.sim.h / 2);
    }
    const id = this.sim.devSpawn(kind, px, py);
    if (id === 4294967295) return false;
    this.log.push({ type: 'cmd', act: 'dev-spawn', kind, x: r2(px), y: r2(py) });
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
    return true;
  }

  private coordsMode = false;

  toggleCoords(): boolean {
    this.coordsMode = !this.coordsMode;
    const el = document.getElementById('coords');
    if (el) el.classList.toggle('hidden', !this.coordsMode);
    return this.coordsMode;
  }

  /** Clicked-world logging for the coords tool (called from handleClick). */
  private logCoords(x: number, y: number): void {
    if (!this.coordsMode) return;
    const q = this.sim.queenId();
    const qs = q !== null ? this.sim.cur.get(q) : undefined;
    import('./console').then(({ pushLine }) => {
      pushLine(`click @ ${x.toFixed(2)}, ${y.toFixed(2)} (tile ${Math.floor(x)},${Math.floor(y)})`, 'cmd');
      if (qs) {
        pushLine(`queen @ ${qs.x.toFixed(2)}, ${qs.y.toFixed(2)} (tile ${Math.floor(qs.x)},${Math.floor(qs.y)})`, 'cmd');
      }
    });
  }

  private updateCoordsHud(): void {
    if (!this.coordsMode) return;
    const el = document.getElementById('coords');
    if (!el) return;
    const me = this.playerAnt !== null ? this.sim.cur.get(this.playerAnt) : undefined;
    if (me) {
      el.textContent = `x ${me.x.toFixed(2)} y ${me.y.toFixed(2)} · tile ${Math.floor(me.x)},${Math.floor(me.y)}`;
    }
  }

  debugStep(n: number): void {
    if (this.sim.dead) return;
    for (let i = 0; i < n; i++) this.tickWithReplay();
    this.followPlayerLayer();
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
  }

  debugStepTo(target: number): void {
    if (this.sim.dead) return;
    while (this.sim.tickCount < target) this.tickWithReplay();
    this.followPlayerLayer();
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
  }

  debugReplay(): Record<string, unknown> {
    const cmds = this.log.dump()
      .events.filter((e) => e.type === 'cmd' && e.src !== 'replay')
      .map((e) => {
        const c: Record<string, unknown> = { t: e.tick, act: e.act, ant: e.ant };
        if (e.x !== undefined) c.x = e.x;
        if (e.y !== undefined) c.y = e.y;
        if (e.tx !== undefined) c.tx = e.tx;
        if (e.ty !== undefined) c.ty = e.ty;
        if (e.target !== undefined) c.target = e.target;
        if (e.kind !== undefined) c.kind = e.kind;
        if (e.n !== undefined) c.n = e.n;
        if (e.layer !== undefined) c.layer = e.layer;
        if (e.soil !== undefined) c.soil = e.soil;
        return c;
      });
    return {
      version: 2,
      seed: this.seed,
      name: `session-${new Date().toISOString().slice(0, 19)}`,
      core: coreVersion(),
      team: this.sim.team(),
      ticks: this.sim.tickCount,
      cmds,
    };
  }

  debugCanon(): string {
    return this.sim.canonical();
  }

  debugSpawn(kind: string, x: number, y: number): boolean {
    if (this.sim.dead || this.replay !== null) return false;
    const id = this.sim.devSpawn(kind, x, y);
    if (id === 4294967295) return false;
    this.log.push({ type: 'cmd', act: 'dev-spawn', kind, x: r2(x), y: r2(y) });
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
    return true;
  }

  debugSetFood(n: number): void {
    if (this.sim.dead || this.replay !== null) return;
    this.sim.devSetFood(n);
    this.log.push({ type: 'cmd', act: 'dev-food', n });
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
  }

  debugSetWater(n: number): void {
    if (this.sim.dead || this.replay !== null) return;
    this.sim.devSetWater(n);
    this.log.push({ type: 'cmd', act: 'dev-water', n });
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
  }

  debugSetSuper(n: number): void {
    if (this.sim.dead || this.replay !== null) return;
    this.sim.devSetSuper(n);
    this.log.push({ type: 'cmd', act: 'dev-super', n });
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
  }

  debugKillSpiders(): void {
    if (this.sim.dead || this.replay !== null) return;
    let count = 0;
    for (const s of [...this.sim.cur.values()]) {
      if (s.kind === 'spider' && this.sim.devKill(s.id)) count++;
    }
    this.log.push({ type: 'cmd', act: 'dev-kill-spiders', count });
  }

  debugKill(id: number): boolean {
    if (this.sim.dead || this.replay !== null) return false;
    const ok = this.sim.devKill(id);
    if (ok) this.log.push({ type: 'cmd', act: 'dev-kill', target: id });
    return ok;
  }

  togglePause(): void {
    this.paused = !this.paused;
    this.log.push({ type: 'mark', label: this.paused ? 'paused' : 'resumed' });
  }

  togglePerf(): void {
    this.hud.togglePerf();
  }

  debugPixel(x: number, y: number): number[] {
    return this.renderer.pixelAt(x, y);
  }

  /** Gait telemetry for e2e: feet in world space, swing/planted state. */
  debugAnt(id: number): Record<string, unknown> | null {
    return this.renderer.antDebug(id);
  }

  /** Body-sprite placement for e2e render debugging. */
  debugAntParts(id: number): Record<string, unknown> | null {
    return this.renderer.antPartsDebug(id);
  }

  debugTile(layer: number, x: number, y: number): number {
    return this.sim.tileAt(layer, x, y);
  }

  debugSoil(layer: number, x: number, y: number): number {
    return this.sim.soilAt(layer, x, y);
  }

  debugSetSoil(layer: number, x: number, y: number, soil: number): void {
    if (this.sim.dead || this.replay !== null) return;
    this.sim.devSetSoil(layer, x, y, soil);
    this.log.push({ type: 'cmd', act: 'dev-soil', layer, x, y, soil });
  }

  debugState(): Record<string, unknown> {
    const counts = this.sim.casteCounts();
    const spiders: Record<string, unknown>[] = [];
    const ants: Record<string, unknown>[] = [];
    for (const s of this.sim.cur.values()) {
      if (s.kind === 'spider') {
        spiders.push({ id: s.id, x: +s.x.toFixed(2), y: +s.y.toFixed(2), hp: +s.hp.toFixed(2), layer: s.layer });
      } else if (isAnt(s) && s.kind !== 'queen') {
        ants.push({ id: s.id, kind: s.kind, x: +s.x.toFixed(2), y: +s.y.toFixed(2), hp: +s.hp.toFixed(2), activity: s.activity, carrying: carryLabel(s.carry), layer: s.layer === 0 ? 'S' : 'U' });
      }
    }
    let foods = 0;
    for (const s of this.sim.cur.values()) if (s.kind === 'food' || s.kind === 'source') foods++;
    const qs = this.sim.ant(this.sim.queenId() ?? -1);
    return {
      tick: this.sim.tickCount,
      dead: this.sim.dead,
      phase: this.sim.phase(),
      phaseTime: +this.sim.phaseTime().toFixed(1),
      team: this.sim.team(),
      carbs: this.sim.storeCarbs(),
      protein: this.sim.storeProtein(),
      water: this.sim.storeWater(),
      workers: counts.workers,
      soldiers: counts.soldiers,
      eggs: this.sim.eggCount(),
      dug: this.sim.tilesDug(),
      layer: this.renderer.activeLayer === 0 ? 'surface' : 'underground',
      playerAnt: this.playerAnt,
      queen: qs
        ? {
            id: qs.id,
            x: +qs.x.toFixed(2),
            y: +qs.y.toFixed(2),
            layer: qs.layer === 0 ? 'S' : 'U',
            activity: qs.activity,
            carry: carryLabel(qs.carry),
          }
        : null,
      entrance: this.sim.entrance,
      paused: this.paused,
      spiders,
      ants,
      foods,
    };
  }

  restartFounding(team: number): void {
    if (this.replay !== null) {
      this.replay = null;
      this.hud.setReplay(false);
      this.log.push({ type: 'mark', label: 'replay-aborted-by-restart' });
    }
    this.seed = Date.now() % 0x7fffffff;
    this.log.push({ type: 'restart', seed: this.seed, team });
    this.sim = new Sim(this.seed, 3, 6, { founding: true, team });
    this.playerAnt = this.initialAnt();
    this.lastPlayerLayer = null;
    this.steerTarget = null;
    this.lastEntrance = undefined;
    this.renderer.reset(this.sim);
    this.hud.hideDead();
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
    this.acc = 0;
    this.deadShown = false;
    this.log.push({ type: 'start', seed: this.seed, team, workers: 0 });
  }

  private toggleLayer(): void {
    if (this.playerAnt !== null) return; // camera is locked to the controlled ant; Tab is spectator-only
    this.renderer.toggleLayer();
    this.log.push({ type: 'view', layer: this.renderer.activeLayer === 0 ? 'surface' : 'underground' });
  }

  /** LMB press on something interactive. True when consumed (no steering). */
  private interactAt(x: number, y: number): boolean {
    if (this.sim.dead) return false;
    const placing = this.dev.placement();
    if (placing !== null) {
      this.dev.setPlacement(null);
      this.debugSpawn(placing, x, y);
      return true;
    }
    if (this.replay !== null) return false;
    if (this.playerAnt === null) return false;
    const pick = this.pickEntity(x, y);
    if (pick && pick.kind === 'spider') {
      this.log.push({ type: 'cmd', act: 'attack', ant: this.playerAnt, target: pick.id });
      this.sim.attack(this.playerAnt, pick.id);
      this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
      return true;
    }
    // selecting the already-controlled ant must not consume the press —
    // steering starts with LMB down and the controlled ant sits at center
    if (pick && pick.id !== this.playerAnt) {
      this.playerAnt = pick.id;
      this.lastPlayerLayer = null;
      this.log.push({ type: 'cmd', act: 'select', ant: pick.id });
      this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
      return true;
    }
    return false;
  }

  private steer(x: number, y: number, phase: 'start' | 'move' | 'end'): void {
    if (phase === 'end') {
      this.steerTarget = null;
      return;
    }
    if (this.sim.dead || this.replay !== null || this.playerAnt === null) return;
    this.steerTarget = { x, y };
  }

  private cycleAnt(): void {
    // founding: the roster is the queen plus her workers
    const roster = this.sim.workers();
    if (this.sim.foundingMode) {
      const q = this.sim.queenId();
      if (q !== null) roster.unshift(q);
    }
    if (roster.length === 0) return;
    const i = this.playerAnt === null ? 0 : roster.indexOf(this.playerAnt);
    const from = this.playerAnt;
    this.playerAnt = roster[(i + 1) % roster.length];
    this.lastPlayerLayer = null; // re-baseline the follow camera on the new ant
    this.log.push({ type: 'cmd', act: 'cycle', from, to: this.playerAnt });
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
  }

  /**
   * Camera lock: while an ant is controlled, the view always shows its
   * layer (crossings logged as follow events) and free panning is off.
   * Spectating (no controlled ant) restores free pan/zoom/Tab.
   */
  private followPlayerLayer(): void {
    if (this.playerAnt === null) {
      this.lastPlayerLayer = null;
      this.input.panEnabled = true;
      return;
    }
    const me = this.sim.cur.get(this.playerAnt);
    if (me === undefined) {
      this.lastPlayerLayer = null;
      this.input.panEnabled = true;
      return;
    }
    this.input.panEnabled = false;
    const changed = this.lastPlayerLayer !== null && me.layer !== this.lastPlayerLayer;
    this.lastPlayerLayer = me.layer;
    if (me.layer !== this.renderer.activeLayer) {
      this.renderer.setActiveLayer(me.layer);
      if (changed) {
        this.log.push({ type: 'view', layer: me.layer === 0 ? 'surface' : 'underground', src: 'follow' });
      }
    }
  }

  /** Nearest interactive entity (queen/worker/soldier/spider) within pick
   * radius on the active layer. Food and eggs are not pickable — a crumb
   * next to a spider must never shadow the spider. */
  private pickEntity(x: number, y: number): AntEnt | SpiderEnt | null {
    const layer = this.renderer.activeLayer;
    let best: AntEnt | SpiderEnt | null = null;
    let bestD = 0.6 * 0.6;
    for (const s of this.sim.cur.values()) {
      if (s.layer !== layer) continue;
      if (s.kind !== 'queen' && s.kind !== 'worker' && s.kind !== 'soldier' && s.kind !== 'spider')
        continue;
      const d = (s.x - x) * (s.x - x) + (s.y - y) * (s.y - y);
      if (d < bestD) {
        bestD = d;
        best = s;
      }
    }
    return best;
  }

  private handleClick(x: number, y: number, button: number): void {
    this.logCoords(x, y);
    if (this.sim.dead) return;
    const placing = this.dev.placement();
    if (placing !== null) {
      this.dev.setPlacement(null);
      this.debugSpawn(placing, x, y);
      return;
    }
    if (this.playerAnt === null) return;
    if (this.replay !== null) return; // replay is watch-only: camera/view stay live, sim commands don't
    const pick = this.pickEntity(x, y);
    if (pick && pick.kind === 'spider') {
      this.log.push({ type: 'cmd', act: 'attack', ant: this.playerAnt, target: pick.id });
      this.sim.attack(this.playerAnt, pick.id);
      this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
      return;
    }
    if (button === 0) {
      if (pick && pick.id !== this.playerAnt) {
        this.playerAnt = pick.id;
        this.lastPlayerLayer = null; // re-baseline the follow camera on the new ant
        this.log.push({ type: 'cmd', act: 'select', ant: pick.id });
      } else {
        this.log.push({ type: 'cmd', act: 'move', ant: this.playerAnt, x: r2(x), y: r2(y) });
        this.sim.move(this.playerAnt, x, y);
      }
    } else if (button === 2) {
      const layer = this.renderer.activeLayer;
      const me = this.sim.ant(this.playerAnt);
      const tx = Math.floor(x);
      const ty = Math.floor(y);
      // founding queen context actions (flight → land, grounded → found nest)
      if (me !== undefined && me.kind === 'queen') {
        const phase = this.sim.phase();
        if (phase === 0) {
          // right-click: fly to the destination and land there
          this.log.push({ type: 'cmd', act: 'land', ant: this.playerAnt, x: r2(x), y: r2(y) });
          this.sim.land(this.playerAnt, x, y);
          this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
          return;
        }
        if (phase === 1 && me.layer === 0) {
          // right-click: walk to the chosen ground and found the nest there
          const bx = Math.floor(x) & ~1;
          const by = Math.floor(y) & ~1;
          const fits =
            bx >= 2 && by >= 2 && bx + 3 <= this.sim.w - 3 && by + 5 <= this.sim.h - 3;
          this.log.push({
            type: 'cmd',
            act: 'found',
            ant: this.playerAnt,
            x: r2(x),
            y: r2(y),
            ...(fits ? {} : { note: 'refused-near-edge' }),
          });
          this.sim.found(this.playerAnt, x, y);
          this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
          return;
        }
      }
      // dig any dirt first: the ant walks there and digs on arrival, and an
      // ant still below dirt capacity keeps digging (dig two, haul once).
      // Refusals (hands full, partial rock) fall through to the branches below.
      const kind = this.sim.tileAt(layer, tx, ty);
      const soft = kind >= 1 && kind <= 3;
      if (layer === 1 && soft && this.sim.dig(this.playerAnt, tx, ty)) {
        this.log.push({ type: 'cmd', act: 'dig', ant: this.playerAnt, tx, ty });
        this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
        return;
      }
      // digging with an egg/food in the mandibles is refused by the core —
      // explain instead of silently walking into the wall (dirt falls
      // through to the refill branch below)
      if (layer === 1 && soft && me !== undefined && me.carry.t !== 'none' && me.carry.t !== 'dirt') {
        this.hud.flashHint(
          me.carry.t === 'egg'
            ? 'Mandibles hold the egg — place it on an EMPTY cell first (right-click), then dig'
            : `Mandibles hold the ${me.carry.t} — place or drop it first, then dig`,
        );
        this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
        return;
      }
      // carrying something → place/drop it (egg, dirt, resources)
      if (me !== undefined && me.carry.t === 'egg') {
        // egg: place on the empty target cell — the ant walks there first;
        // but clicking the nest hole while carrying means "bring it home":
        // skip so the entrance crossing below takes the egg underground
        const entE = this.sim.entrance;
        const nearHoleE =
          entE !== null &&
          Math.max(Math.abs(tx - (entE[0] + 1)), Math.abs(ty - (entE[1] + 1))) <= 2;
        {
          const kind = this.sim.tileAt(me.layer, tx, ty);
          if (!nearHoleE && kind === 0 && this.sim.drop(this.playerAnt, tx, ty)) {
            this.log.push({ type: 'cmd', act: 'drop', ant: this.playerAnt, tx, ty, note: 'egg' });
            this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
            return;
          }
          if (!nearHoleE && kind !== 0) {
            this.hud.flashHint('Eggs are placed on empty cells — this tile is solid; dig it out first (empty mandibles)');
            return;
          }
        }
      }
      if (me !== undefined && me.carry.t === 'dirt') {
        if (me.layer === 1) {
          // dirt: fill the fully-empty adjacent 2×2 block — but never the
          // entrance hole: clicking near it while hauling means "carry it out"
          const ent0 = this.sim.entrance;
          const isHoleBlock =
            ent0 !== null && (tx & ~1) === ent0[0] && (ty & ~1) === ent0[1];
          const bx = tx & ~1;
          const by = ty & ~1;
          let emptyBlock = true;
          for (let dy = 0; dy < 2; dy++) {
            for (let dx = 0; dx < 2; dx++) {
              if (this.sim.tileAt(1, bx + dx, by + dy) !== 0) emptyBlock = false;
            }
          }
          if (!isHoleBlock && emptyBlock && this.sim.drop(this.playerAnt, tx, ty)) {
            this.log.push({ type: 'cmd', act: 'drop', ant: this.playerAnt, tx, ty, note: 'dirt' });
            this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
            return;
          }
        } else {
          // dirt above ground: the drop simply discards it
          this.log.push({ type: 'cmd', act: 'drop', ant: this.playerAnt, tx: 0, ty: 0, note: 'surface' });
          this.sim.drop(this.playerAnt, 0, 0);
          this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
          return;
        }
        // invalid placement → fall through (walk / entrance / dig-refused)
      }
      if (me !== undefined && (me.carry.t === 'wood' || me.carry.t === 'wool')) {
        // nest-building collectibles: underground converts an empty adjacent
        // block into food storage (wood/silver) or nursery soil (wool/orange);
        // above ground they are set back down
        if (me.layer === 1) {
          const bx = tx & ~1;
          const by = ty & ~1;
          let emptyBlock = true;
          for (let dy = 0; dy < 2; dy++) {
            for (let dx = 0; dx < 2; dx++) {
              if (this.sim.tileAt(1, bx + dx, by + dy) !== 0) emptyBlock = false;
            }
          }
          if (emptyBlock && this.sim.drop(this.playerAnt, tx, ty)) {
            this.log.push({
              type: 'cmd',
              act: 'drop',
              ant: this.playerAnt,
              tx,
              ty,
              note: me.carry.t === 'wood' ? 'food-storage block' : 'nursery block',
            });
            this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
            return;
          }
        } else if (this.sim.tileAt(0, tx, ty) === 0 && this.sim.drop(this.playerAnt, tx, ty)) {
          this.log.push({ type: 'cmd', act: 'drop', ant: this.playerAnt, tx, ty, note: 'set down' });
          this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
          return;
        }
      }
      if (me !== undefined && me.carry.t === 'food') {
        // resource unit: drop it on the empty target cell (spoils off
        // silver) — but clicking the nest hole while hauling means "bring it
        // home": skip so the entrance crossing below takes it.
        // (Named carries fix the old `aux < 2.5` test that silently
        // excluded protein/carbs/water units from manual drops.)
        const entR = this.sim.entrance;
        const nearHole =
          entR !== null &&
          Math.max(Math.abs(tx - (entR[0] + 1)), Math.abs(ty - (entR[1] + 1))) <= 2;
        const kind = this.sim.tileAt(me.layer, tx, ty);
        if (!nearHole && kind === 0 && this.sim.drop(this.playerAnt, tx, ty)) {
          this.log.push({ type: 'cmd', act: 'drop', ant: this.playerAnt, tx, ty, note: 'food' });
          this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
          return;
        }
      }
      // empty hands + adjacent egg under the cursor → pick it up
      if (me !== undefined && me.carry.t === 'none') {
        let egg: Ent | null = null;
        let bestD = 0.8 * 0.8;
        for (const s of this.sim.cur.values()) {
          if (s.kind !== 'egg' || s.layer !== layer || s.carried) continue;
          const d = (s.x - x) * (s.x - x) + (s.y - y) * (s.y - y);
          if (d < bestD) {
            bestD = d;
            egg = s;
          }
        }
        if (egg !== null && this.sim.pickEgg(this.playerAnt, egg.id)) {
          // adjacent picks happen instantly; distant ones send the ant
          // walking and it picks up on arrival (core intent)
          this.log.push({ type: 'cmd', act: 'pick-egg', ant: this.playerAnt, target: egg.id });
          this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
          return;
        }
      }
      const ent = this.sim.entrance;
      if (
        ent &&
        Math.max(Math.abs(tx - (ent[0] + 1)), Math.abs(ty - (ent[1] + 1))) <= 2
      ) {
        this.log.push({ type: 'cmd', act: 'entrance', ant: this.playerAnt });
        this.sim.useEntrance(this.playerAnt);
      } else {
        this.log.push({ type: 'cmd', act: 'move', ant: this.playerAnt, x: tx + 0.5, y: ty + 0.5 });
        this.sim.move(this.playerAnt, tx + 0.5, ty + 0.5);
      }
    }
    this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
  }

  private frame = (now: number): void => {
    const rawMs = now - this.last;
    const dt = Math.min(0.25, rawMs / 1000);
    this.last = now;
    this.perfFrames++;
    this.perfMs += rawMs;
    if (rawMs > this.perfWorst) this.perfWorst = rawMs;
    if (!this.sim.dead && !this.paused) {
      this.acc += dt;
      const step = 1 / TPS;
      while (this.acc >= step) {
        this.tickWithReplay();
        this.acc -= step;
      }
      this.followPlayerLayer();
      const ant = this.playerAnt;
      const me = ant !== null ? this.sim.cur.get(ant) : undefined;
      if (me !== undefined && ant !== null) {
        // hold-to-steer: re-issue the move at most every 4 ticks
        if (this.steerTarget !== null && this.sim.tickCount - this.lastSteerTick >= 4) {
          this.lastSteerTick = this.sim.tickCount;
          this.log.push({ type: 'cmd', act: 'move', ant, x: r2(this.steerTarget.x), y: r2(this.steerTarget.y), note: 'steer' });
          this.sim.move(ant, this.steerTarget.x, this.steerTarget.y);
        }
        // camera tracks the same interpolated position the sprite is drawn at —
        // centering on the raw tick snapshot judders at sim rate (20 Hz)
        const p = this.sim.prev.get(ant) ?? me;
        const pos = lerpPos(p, me, Math.min(1, this.acc * TPS));
        this.renderer.centerOn(pos.x, pos.y);
      }
      if (++this.hudCounter >= 5) {
        this.hudCounter = 0;
        this.hud.update(this.sim, this.playerAnt, this.renderer.activeLayer);
      }
    }
    if (this.sim.dead && !this.deadShown) {
      this.deadShown = true;
      this.log.push({ type: 'death', tick: this.sim.tickCount, ants: this.sim.workers().length });
      this.hud.showDead(this.sim, () => {
        this.hud.hideDead();
        if (this.onToMenu !== null) this.onToMenu();
        else this.restartFounding(0);
      });
    }
    // the entrance appears when the founding queen creates the nest
    if (this.sim.entrance !== this.lastEntrance) {
      this.lastEntrance = this.sim.entrance;
      this.sim.refreshSoil(); // founding grants paint soil blocks at this moment
      this.renderer.drawTiles(0);
      this.sim.consumeDirty(0);
      this.renderer.drawTiles(1);
      this.sim.consumeDirty(1);
      this.renderer.refreshEntrance();
    }
    this.renderer.panContinuous(dt, this.playerAnt === null ? this.input.keys : Input.NO_KEYS);
    if (this.sim.consumeDirty(this.renderer.activeLayer)) {
      this.renderer.drawTiles(this.renderer.activeLayer);
    }
    this.renderer.renderEntities(this.acc * TPS, dt, this.playerAnt);
    if (this.perfStart === 0) this.perfStart = now;
    if (now - this.perfStart >= 500) {
      const secs = (now - this.perfStart) / 1000;
      const layer = this.renderer.activeLayer;
      let shown = 0;
      for (const s of this.sim.cur.values()) if (s.layer === layer) shown++;
      this.hud.setPerf(
        `fps ${Math.round(this.perfFrames / secs)} · ${Math.round(this.perfMs / this.perfFrames)}ms avg · ${Math.round(this.perfWorst)}ms worst\n` +
          `sim ${Math.round(this.perfTicks / secs)} tps · ${this.sim.cur.size} entities (${shown} shown)\n` +
          `zoom ${this.renderer.cam.zoom.toFixed(2)} · tick ${this.sim.tickCount}`,
      );
      this.perfStart = now;
      this.perfFrames = 0;
      this.perfMs = 0;
      this.perfWorst = 0;
      this.perfTicks = 0;
    }
    this.updateCoordsHud();
    if (consoleOpen()) pollEvents();
    requestAnimationFrame(this.frame);
  };

  private tickWithReplay(): void {
    this.sim.tick();
    this.perfTicks++;
    const rp = this.replay;
    if (!rp) return;
    let applied = false;
    while (rp.i < rp.cmds.length && rp.cmds[rp.i].t <= this.sim.tickCount) {
      const c = rp.cmds[rp.i++];
      const ev: { type: string; src: string; act: unknown; ant: number } & Record<string, unknown> = {
        type: 'cmd',
        src: 'replay',
        act: c.act,
        ant: c.ant ?? 0,
      };
      if (c.x !== undefined) ev.x = c.x;
      if (c.y !== undefined) ev.y = c.y;
      if (c.tx !== undefined) ev.tx = c.tx;
      if (c.ty !== undefined) ev.ty = c.ty;
      if (c.target !== undefined) ev.target = c.target;
      this.log.push(ev);
      if (c.act === 'move') this.sim.move(c.ant ?? 0, c.x ?? 0, c.y ?? 0);
      else if (c.act === 'dig') this.sim.dig(c.ant ?? 0, c.tx ?? 0, c.ty ?? 0);
      else if (c.act === 'attack') this.sim.attack(c.ant ?? 0, c.target ?? 0);
      else if (c.act === 'entrance') this.sim.useEntrance(c.ant ?? 0);
      else if (c.act === 'land') this.sim.land(c.ant ?? 0, c.x ?? 0, c.y ?? 0);
      else if (c.act === 'found') this.sim.found(c.ant ?? 0, c.x ?? 0, c.y ?? 0);
      else if (c.act === 'dump' || c.act === 'drop')
        this.sim.drop(c.ant ?? 0, c.tx ?? 0, c.ty ?? 0);
      else if (c.act === 'pick-egg') this.sim.pickEgg(c.ant ?? 0, c.target ?? 0);
      else if (c.act === 'follow-all') this.sim.follow(c.ant ?? 0, 0);
      else if (c.act === 'follow-one') this.sim.follow(c.ant ?? 0, 1);
      else if (c.act === 'follow-soldiers') this.sim.follow(c.ant ?? 0, 2);
      else if (c.act === 'follow-release') this.sim.follow(c.ant ?? 0, 3);
      else if (c.act === 'dev-spawn') this.sim.devSpawn(String(c.kind), c.x ?? 0, c.y ?? 0);
      else if (c.act === 'dev-food') this.sim.devSetFood(Number(c.n ?? 0));
      else if (c.act === 'dev-super') this.sim.devSetSuper(Number(c.n ?? 0));
      else if (c.act === 'dev-kill-spiders') {
        for (const s of [...this.sim.cur.values()]) if (s.kind === 'spider') this.sim.devKill(s.id);
      } else if (c.act === 'dev-kill') this.sim.devKill(Number(c.target ?? 0));
      else if (c.act === 'dev-water') this.sim.devSetWater(Number(c.n ?? 0));
      else if (c.act === 'dev-soil')
        this.sim.devSetSoil(Number(c.layer ?? 0), c.x ?? 0, c.y ?? 0, Number(c.soil ?? 0));
      applied = true;
    }
    if (applied && rp.i >= rp.cmds.length) {
      this.replay = null;
      this.hud.setReplay(false);
    }
  };
}
