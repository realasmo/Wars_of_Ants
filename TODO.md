## NEXT: wave C — neutral creatures (earthworm / snail / rove beetle — designs + costs in docs/WORLD-DESIGN.md; the F4 admin drawer makes their tuning live from day one). After that: D (aphid farming — honeydew already flows from nettles + Honey ants), E (bosses), then the lighting ticket menu (baked AO first; one effect per ticket, rendering-only).

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
- **DONE (admin + indicators, 2026-09-28, ticket
  docs/TICKET-admin-indicators.md; 0.1.04.58-dev):** two features, one wave.
  (1) **Admin tuning drawer** — `?admin=1` / F4 opens a Tweakpane form
  generated from the core's field registry (`rules_meta`): 12 grouped
  sections, `◇ng` badges on new-game-only fields, whole-object atomic
  commits with every validation error listed; live fields apply to the
  running sim (entity stats re-derived, hp fraction preserved), "apply &
  new game" restarts under the rules with the SAME seed; commits are logged
  (`dev-rules`) and replay byte-identically; rules digest displayed + folded
  into the canon since F4. (2) **Activity indicators** — zoomed out (≤0.75)
  every player ant floats a 16px pixel-art icon (client/src/art/icons.ts):
  per-resource tinted harvest droplets, dig, spoil-haul, haul-home, fight,
  medic ✚, feeder 👑-crown, follow reticle, honey production, off-duty Zz;
  core derives an `Intent` + honest `Blocked` reason (empty pantry / medic
  needs water / pantry full / queen starving — wire v5, p8/p9) so icons
  never lie; a gold pulse + icon pop confirms every landed order; refused
  digs explain themselves (soldiers can't dig); the controlling HUD line
  carries the intent + blocked reason. 87 core tests, e2e ×green with
  determinism byte-match.

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
 - ants should have indicator associated with current activity: farming, just moving, attacking, digging (close zoom DONE — procedural rigs animate dig/harvest/carry/combat from the snapshot `activity`; far-zoom icon layer DONE — pixel icon atlas with intent + blocked reasons, admin wave 2026-09-28)

### Farming resources
 - dead insects

## NPC enemies
 - parasite ants

Features:
  - different ants classes, regular soldier, tank, shoter
