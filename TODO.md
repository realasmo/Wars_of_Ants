## NEXT: Wave B — water healing + corpses + Medic (drafts in docs/WORLD-DESIGN.md)

> Code-quality wave shipped 2026-09-26 — audit in `docs/AUDIT.md`, everything
> fixed: typed snapshot layer (`Carry` + decode.ts + boot spec assertion),
> full-state canonical, sim/ module split, irange/pantry/queen-intent/combat
> fixes. On the shelf from the audit for Wave B design (section 6 of
> docs/AUDIT.md): workers don't fight back, best_food ignores colony needs,
> Fetch delivers one unit per trip.

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
