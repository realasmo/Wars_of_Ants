## NEXT: Wave B — water healing + corpses + Medic (drafts in docs/WORLD-DESIGN.md)

## User TODO batch (2026-09-27, verbatim — sequenced into waves below)

- Make ants moving their front hands / mandibles once they start farming,
  when an item is picked up / collected - and when picked up - should be
  displayed as carried by their front hands or mandibles
- Allow queen to also farm and pick collectibles from the ground
- Two new collectibles on map:
  - Wet wood: when placed in base, makes one free block food storage
  - Dry wool: when placed in base, makes one free block egg friendly
- Two new in-game menus:
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

- **F1 — farming feel + queen parity** (client + small core): explicit
  `harvesting` activity → mandible work animation + carried-item display
  polish; queen can farm sources and pick up ground collectibles.
- **F2 — map collectibles** (core + client): wet wood / dry wool entities,
  carry variants, haul home, place-in-base → one block of food storage /
  egg-friendly soil.
- **F3 — squad follow** (core + client): worker X-menu (follow-all /
  join-one / release / soldiers-follow), follow-the-leader AI.
- **F4 — brood production** (the big one, absorbs Wave B + part of D):
  queen X-menu, egg types + costs, worker→soldier conversion, Medic +
  fallen-ant rescue (needs corpse state — Wave B's corpse system),
  Honey caste + aphid honeydew economy (needs aphids or a stand-in
  resource until Wave D).

Open design questions to settle before/while building F4: does
"convert from worker" consume an existing worker; where aphid honeydew
comes from before aphid farming exists; whether followers fight with you
(F3).

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
