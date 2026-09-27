## NEXT: Wave B — water healing + corpses + Medic (drafts in docs/WORLD-DESIGN.md)

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
