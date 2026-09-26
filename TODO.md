## NEXT SESSION: code-quality wave (decided 2026-09-26 — refactor before more content)

> Audit the code we created so far; mercilessly point out all design errors,
> wrong tool for wrong task, risky shortcuts, and bad optimizations.

For starter: apply **Option C — full typed snapshot layer**: a `decode(snap)`
boundary returning proper named shapes (queen, worker, spider, egg, source,
resource-unit), with game logic and rendering consuming names only. The
"right" long-term shape. Fold in **Option B** (explicit snapshot fields,
de-overloading `aux`/`extra`/`state`) at the same time — one format ripple
instead of two. Context: two shipped bugs (the `aux >= 2.5` egg-test
collisions) came from kind codes being load-bearing in three places (core
snapshot, right-click matrix, render dots). After the refactor: boot-time
assertion that client kind constants match core, then back to content waves
(B water/healing/corpses → C neutrals → D aphids → E bosses).

## Ants walking and farming.
 - When ant under player control exits the nest, the camera should switch to surface as well and same the opposite way. (DONE via the locked camera)
 - ants should have indicator associated with current activity: farming, just moving, attacking, digging

### Farming resources
 - dead insects

## NPC enemies
 - parasite ants

Features:
  - different ants classes, regular soldier, tank, shoter
