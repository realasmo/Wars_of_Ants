## NEXT: admin tuning panel + activity indicators (one wave) — part 1: the ?admin=1 client overlay over the GameRules data layer (GUI-first, whole-rules get/set with validation, live-vs-new-game scopes, world-gen preview; full decisions in the project memory + AGENTS quality bar). Part 2, folded in 2026-09-28: **activity indicators** — floating pixel-icon atlas over player-colony ants (💤 off-duty, per-resource harvest icons, ⛏ dig, ↗ haul home, ⚔ fight, ✚ medic, 👑 feeder, ⌖ following, ! blocked-with-reason); **visible only when the camera is BELOW a height threshold** (zoomed out — the rigs carry the information up close; the user's own rule); needs core enrichment: intent-level job + blocked-reason on the ant snapshot, wire v4→v5. After that: waves C (neutral creatures) → D (aphid farming) → E (bosses), then the lighting ticket menu.

## User TODO batch (2026-09-27, verbatim — sequenced into waves below)

- DONE (F1): Make ants moving their front hands / mandibles once they
  start farming, when an item is picked up / collected - and when picked
  up - should be displayed as carried by their front hands or mandibles
- DONE (F1): Allow queen to also farm and pick collectibles from the ground
- DONE (F2): Two new collectibles on map:
  - Wet wood: when placed in base, makes one free block food storage
  - Dry wool: when placed in base, makes one free block egg friendly
- PARTLY DONE (F3; queen menu = F4): Two new in-game menus:
  - 'x' while a regular ant is under player control:
    1 / click: all ants within visible range start following you
    2 / click: one more ant joins you per activation
    3 / click: release ants
    4 / click: all soldiers start following you
  - 'x' by the queen — new egg types:
    - Worker — 2x proteins, 1x water to spawn. When idle, defaults to
      feeding Queen, otherwise farms resources, very weak
    - Soldier — 6x proteins, 3x water to convert from worker. Muscles,
      can attack neutral / hostile creatures
    - Honey — 1x protein, 8x aphid honeydew. Spawns 1x aphid honeydew / 4mins
    - Medic — 4x proteins, 3x carbohydrates. Can transport fallen ants
      back to the nest to heal.

Agent sequencing of the batch (one ticket per wave, playtest-gated):

- **F1 — farming feel + queen parity — SHIPPED `1c21468`** (client + small core): explicit
  `harvesting` activity → mandible work animation + carried-item display
  polish; queen can farm sources and pick up ground collectibles.
- **F2 — map collectibles — SHIPPED `866ed92`** (core + client): wet wood / dry wool entities,
  carry variants, haul home, place-in-base → one block of food storage /
  egg-friendly soil.
- **F3 — squad follow — SHIPPED `e0206a0`, silver squad rings** (core + client): worker X-menu (follow-all /
  join-one / release / soldiers-follow), follow-the-leader AI.
- **P — worker priorities — SHIPPED 2026-09-27** (ticket
  docs/TICKET-worker-priorities.md): physical queen feeding (rotating
  craving cycle, one feeder worker, reserves as a real pantry pile,
  self-feeding from her own mandibles), auto-scout removed (loiter →
  return home → rest → glance at the nest mouth), followers resume their
  interrupted job on release, queen request display above her + HUD row,
  foragers prioritize the craved resource.
- **S — squad conversion — SHIPPED 2026-09-27, simplified same day** (ticket
  docs/TICKET-squad-conversion.md): a leader who farms converts the squad —
  followers (workers too) keep farming the source until re-recruited with
  another "all join"; an attack order is shared — the whole squad attacks
  the leader's target, then returns to following when it dies; feeding
  outranks conversion (the feeder may be reclaimed); fight-end releases
  player-commanded ants back to autonomy.
- **DONE (F4, 2026-09-28, `6286617`→ship): brood production** — the big one,
  Wave B + part of D absorbed. Queen X-menu (X as the queen, 1–4 or click):
  Worker 2p+1w · Soldier 6p+3w **+ consumes one worker** (metamorphosis) ·
  Honey 1p+8h · Medic 4p+3c — costs paid from the PHYSICAL pantry, every
  refusal explains itself in the help bar. This also fixes the founding
  softlock (0 eggs + 0 workers is recoverable — order more workers for
  pantry protein+water). Honeydew is the 4th resource: **nettle** map
  sources (15–22u, 10s/u, 4 scattered) until Wave D's aphids; Honey ants
  secrete 1 honeydew/4 min (banks on silver, spoils loose elsewhere).
  Combat deaths of caste ants **down** them instead of killing: 60s bleed-out,
  medics auto-rescue (carry home, heal for 2 stored water, revive at full
  hp; spiders ignore downed ants; red DOWN labels + red HP bars). Squad
  **retaliation** shipped: an attacked leader's followers share the
  attacker as their target. Medics never farm (stand-by in the nest).
  Foundation: **GameRules** — every tunable (old balance consts + Config +
  worldgen literals) is now one typed, validated, digest-hashed ruleset on
  Sim (the admin panel's substrate; F4's brood costs were its first data
  rows). Wire layout v4 (stride 13). 71 core tests, e2e 3× green with
  determinism byte-match.
- **NEXT wave — admin panel + activity indicators (settled 2026-09-28, not
  started):** two features, one wave. (1) Admin tuning panel: embedded
  ?admin=1 DOM drawer (never a separate server), Tweakpane widgets (no RMB
  needed), generated from a field registry; core surface `rules_get/set/
  default/hash` — whole-object atomic commit with cross-field validation;
  live-apply vs applies-on-new-game badges; rules digest already in the
  canon since F4. (2) Activity indicators: floating pixel-art icons over
  player-colony ants — off-duty / per-resource harvest / dig / haul-home /
  fight / medic / feeder / following / **!-blocked-with-a-reason** (empty
  pantry, unreachable, soldiers-can't-dig, queen starving); icons render
  only when zoomed OUT past a height threshold (rigs show the work up
  close — user's rule); core grows an intent enum + blocked-reason enum on
  the ant snapshot (wire v5) so icons never lie (a feeder fetching from the
  pantry must not read as idle); brief confirm-pulse when an order lands.
  Order-confirmation and the !-reason system double as command-feedback.

---

> Code-quality wave shipped 2026-09-26 — audit in `docs/AUDIT.md`, everything
> fixed: typed snapshot layer (`Carry` + decode.ts + boot spec assertion),
> full-state canonical, sim/ module split, irange/pantry/queen-intent/combat
> fixes. On the shelf from the audit for Wave B design (section 6 of
> docs/AUDIT.md): workers don't fight back, best_food ignores colony needs,
> Fetch delivers one unit per trip.

> Procedural-ants ticket SHIPPED 2026-09-27 (client-only rendering wave —
> `client/src/art/`: ants.ts config, bake.ts part textures, gait.ts
> planted-feet solver, antView.ts rig). Verification: e2e/gait.mjs (planted
> feet, tripod alternation, ripple stagger, idle antennae) + smoke suite +
> GLM-Vision QC loop. Perf note: SwiftShader proxy is fill-rate bound
> (~4× placeholder pixels; main thread 90% idle) — check ?perf=1 on real
> hardware; worst-frame parity with the pre-change baseline is expected
> there, not under software rasterization.

Old Wave B shape (mostly absorbed into F4 — corpses + Medic; water's
healing sink still to design against playtesting):
- water heals (HP regen / Medic mechanic — pick one)
- corpses: dead ants become harvestable protein (design decision needed:
  corpse as resource vs pure visual decay)
- Medic caste

## Ants walking and farming.
 - When ant under player control exits the nest, the camera should switch to surface as well and same the opposite way. (DONE via the locked camera)
 - ants should have indicator associated with current activity: farming, just moving, attacking, digging (close zoom DONE — procedural rigs animate dig/harvest/carry/combat from the snapshot `activity`; far-zoom icon layer — pixel icon atlas with intent + blocked reasons — settled for the admin wave)

### Farming resources
 - dead insects

## NPC enemies
 - parasite ants

Features:
  - different ants classes, regular soldier, tank, shoter
