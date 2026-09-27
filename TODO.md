## NEXT: Wave B — water healing + corpses + Medic (drafts in docs/WORLD-DESIGN.md)

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
- **F4 — brood production — NEXT** (the big one, absorbs Wave B + part of D):
  queen X-menu, egg types + costs, worker→soldier conversion (consumes the
  worker), Medic + fallen-ant rescue (needs corpse state — Wave B's corpse
  system), Honey caste + nettle-sourced honeydew economy (15–22 units,
  10s/unit, 4 scattered) until Wave D's aphids; squads: soldiers fight for
  the leader (attack-click + retaliate), worker followers never fight, the
  whole squad farms once the leader starts.

F4 design settled 2026-09-27: convert-from-worker consumes the existing
worker; honeydew comes from farming nettle (new map source) until aphids
exist; followers — soldiers attack what you attack and retaliate when
you're hit, workers never fight, and the whole squad farms once you start.

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

Wave B shape (decide details against playtesting):
- water heals (HP regen / Medic mechanic — pick one)
- corpses: dead ants become harvestable protein (design decision needed:
  corpse as resource vs pure visual decay)
- Medic caste

## Ants walking and farming.
 - When ant under player control exits the nest, the camera should switch to surface as well and same the opposite way. (DONE via the locked camera)
 - ants should have indicator associated with current activity: farming, just moving, attacking, digging (the snapshot now exposes `activity` — client work only)

### Farming resources
 - dead insects

## NPC enemies
 - parasite ants

Features:
  - different ants classes, regular soldier, tank, shoter
