# Code-quality audit — 2026-09-26

Scope: everything shipped through `20cebef` (~6.5k lines: core, client, e2e, examples).
Verdict up front: **the architecture is right and the discipline is unusually good**
(deterministic core, versioned formats, behavioral tests, dev tooling) — but the code
carries one systemic design error that has already shipped three bugs, plus a handful of
latent correctness bugs, several scaling shortcuts that will hurt in Phase 4, and hygiene
debt. Every finding below is fixed in this wave or explicitly scheduled.

Ranking: 🔴 live bug · 🟠 latent bug (correct today by accident) · 🟡 design error ·
🔵 performance shortcut · ⚪ hygiene.

---

## 1. The systemic error: one overloaded snapshot, three shipped bugs

**🔴 A1. `aux`/`extra`/`state`/`hp` each mean a different thing per entity kind, and
consumers re-derive meaning with float-range tests.**

`sim.rs:3033-3137` packs every entity into `{id, kind, layer, x, y, state, extra, hp, aux}`
where each of the last four fields changes meaning by kind:

| field | ant | food pile | source | egg | spider |
|---|---|---|---|---|---|
| `state` | 0 idle/1 moving/2 digging/3 fighting/4 flying | 0 loose/1 pantry | 2 | 0/1 carried | 0/1 hunting |
| `extra` | carry amount (queen: hunger frac) | amount | amount | hatch frac | hp frac |
| `hp` | hp frac | spoil frac | **source type 1..6** | 1.0 | hp frac |
| `aux` | carry kind code | food kind code | food kind | soldier flag | 0 |

Three client sites decode these by numeric ranges, and all three have been wrong:

1. **Shipped & fixed earlier:** the `aux >= 2.5` egg-detection collisions (the trigger
   for this wave).
2. **🔴 Live — manual food drops are impossible in the founding economy.**
   `game.ts:600`: the drop branch tests `me.aux < 2.5`, but carried protein/carbs/water
   have aux codes 4/5/6. Right-clicking with a carried resource falls through to plain
   move. The comment above the line (`resource units (4–6) … take the drop branch below`)
   says the opposite of what the code does — the condition was written for the legacy
   Green/Super codes (0/1) and never widened.
3. **🔴 Live — water and carbs piles are labeled "protein".** `render.ts:75`:
   `s.aux > 4.5 ? 'protein' : s.aux > 5.5 ? 'water' : …` — the `> 4.5` arm shadows the
   water arm, so aux 5 (carbs) and 6 (water) both render as `protein`.

Root cause in the core: `Carrying { amount, kind: FoodKind }` (`components.rs:117`) where
`FoodKind` doubles as a carried-item code (`Dirt = 2, Egg = 3`, kept "for snapshot-code
stability" per its own comment). Because `kind` is never cleared when `amount` hits 0:

- **🔴 Live — phantom carried-egg dot:** a player-controlled worker that places an egg
  keeps `kind == Egg`, so `render.ts:368`'s `aux >= 2.5 && aux < 3.5` draws a ghost egg
  on an empty-handed ant until it picks something else.
- **🔴 Live — right-click egg pickup breaks permanently per ant:** after a worker drops
  protein, stale `kind == Protein` (aux 4) fails `game.ts:616`'s "empty hands" test
  (`aux <= 1.5`) forever.

**Fix (this wave): Option C + Option B.** Core snapshot becomes a typed
`enum EntitySnap { Ant{..}, Food{..}, Egg{..}, Spider{..} }` with one named field per
fact; `Carrying` becomes `enum Carry { Dirt{blocks}, Egg, Food(FoodKind), None }`;
`FoodKind` loses `Dirt`/`Egg` and means only "edible resource". The WASM transport stays
a compact positional array (it is a wire format, versioned, single-sourced), and the new
TS `decode(snap)` boundary turns it into named shapes (`'queen' | 'worker' | 'egg' | …`)
that all game logic and rendering consume. A boot-time assertion pins the client's kind
table to the core's (core exports the codes; client asserts on startup).

Principle violated: *codes that are load-bearing across a boundary must have exactly one
producer and one decoder, both typed.* Every place that wrote `aux > 2.5` was
hand-translating an enum it couldn't see.

## 2. Correctness bugs (core)

**🟠 A2. Queen retry-freeze.** `sim.rs:1845`: the queen branch of `worker_ai` skips ants
with `retry > 0`, but the decrement at `sim.rs:1932` lives only in the worker path the
queen never reaches. Any `set_retry` on the queen (set on failed mid-walk re-routes:
`sim.rs:1862, 1882, 1894, 1924`) permanently disables her walk-to-act intents — she can
never auto-resolve `land_after`/`found_after`/`dig_after`/`drop_after`/`pick_after`/`
`attack_after` again (manual commands still route, so it degrades quietly instead of
soft-locking). Today it is nearly untriggerable (she is the only digger); the moment
Wave B adds concurrent diggers that wall corridors off, it becomes a live softlock class.
Also `sim.rs:1921-1925`: a failed queen attack sets retry but never clears
`attack_after`. Fix: decrement retry in the queen branch too (or hoist the decrement
before the caste split) and clear the intent when deferring.

**🟠 A3. `Rng::irange` is half-open but every caller believes it is inclusive.**
`rng.rs:30-32` returns `[lo, hi-1]`. Consequences, all silent balance mis-tunes:

- Source amounts never reach their documented max (`SOURCES` moss 15–20 yields ≤19,
  strawberry 40–60 yields ≤59, …).
- `PATCH_GRANT_MIN..MAX` (3,6) grants 3–5, never 6 — README promises 3–6.
- Patch wobble `PATCH_W + irange(0, 2*WOBBLE) - WOBBLE` is asymmetric [-2, +1].
- Founding-grant offsets `irange(0, 16) - 8` are asymmetric [-8, +7].
- `hi == lo` divides by zero (panic) — a footgun for future single-value ranges.

Fix: make `irange` inclusive `[lo, hi]` (callers' intent), guard the degenerate range.
This intentionally changes worldgen draws (all seeds shift) — acceptable: replays are
core-stamped and warn; the determinism fixture regenerates each e2e run.

**🟠 A4. `canonical_state()` is not a full-state digest.** `sim.rs:3004-3031` formats the
*render snapshot* plus a few counters — and omits state that changes future evolution:
the soil grids (`dev_set_soil` is replayable!), `colony.known`, harvest progress
(`Food.progress`), dig progress (`AntState::Digging.progress`), every `WorkerAi` field
(jobs, retry, walk-to intents), `Combat.atk_t`, spider wander/dest/target. Two builds
can diverge in any of these and still pass the cross-platform determinism test. This is
the same conflation as A1 — *render view used as state digest*. Fix: canonical walks the
actual ECS components (a true full-state string), independent of the client snapshot.

**🟠 A5. Friendly fire is enabled.** `Command::Attack` (`sim.rs:596-611`) accepts any
`Ant` or `Predator` target: order a worker to attack your own queen → colony dead. The
client offers this freely (click any ant → attack intent resolves for same-layer). Fix:
`Attack` only valid against `Predator`s (ant-vs-ant combat returns with enemy colonies).

**🟠 A6. Combat chases move in straight lines through walls.** `sim.rs:2477-2484`: a
fighting ant glides directly toward its target with no terrain check — underground this
walks through dirt and rock. Latent only because combat is de-facto surface-only today
(spiders never leave the surface). Fix: underground chases route via pathfinding (the
machinery exists); surface keeps the straight line (terrain is uniform).

## 3. Design errors / wrong tool

**🟡 B1. `FoodKind` triple duty** — world food types, carried-item codes, and resource
types in one enum (`components.rs:26-39`). Covered by the Option B fix above; recorded
because it is the root of A1's client bugs.

**🟡 B2. `sim.rs` is a 3,150-line god file** — worldgen, commands, AI, six systems,
spawning, food logistics, snapshot and canonical in one `impl Sim` with ~60 methods.
Fix this wave: split into `sim/` modules (worldgen, commands, ai, systems, food,
snapshot) with `pub(crate)` internals — pure code motion, no behavior change.

**🟡 B3. Two economies coexist in one sim.** The legacy founded start (`Sim::new`:
green clusters, auto-laying, Super-food soldiers, `START_FOOD`) branches the founding
economy in `queen_system`, `movement`, `eggs`, starvation grace, and egg hatching
(`colony.founding` tests appear 8×). The real game is founding; the legacy path exists
for `colony.rs`/`dev.rs` tests and dev-spawn food. Verdict: keep (the tests are good
regression coverage and Wave B still needs a non-founding economy to evolve into), but
quarantine the naming (`Sim::new_test_colony`) so nobody mistakes it for a supported
game mode. Revisit when Wave B redesigns brood production.

**🟡 B4. The replay act vocabulary is hand-synced in three places** — `game.ts`
`tickWithReplay` (silently skips unknown acts), `determinism_dump.rs` (panics on
unknown), and the export mapping in `debugReplay`. A new act must be added in all three
or the determinism fixture diverges/panics. Root fix is a single serde'd command enum in
core with TS emitting act names — that's Phase-4 protocol work; this wave we keep the
three sites but make the TS side *loud* (console.error on unknown act, matching the
Rust panic) so drift is caught by the e2e run, and we note it in the versioning rule.

**🟡 B5. Entity ids live in a `BTreeMap<u32, Entity>` that is inverted into a `HashMap`
every snapshot call** (`sim.rs:3034-3038`) — per frame, per canonical. Fix: store the id
as an ECS component (`EntId(u32)`); the snapshot reads it in the query; the reverse map
and its per-frame inversion disappear.

**🟡 B6. `worker_ai` duplicates the queen and worker intent-resolution blocks**
(`sim.rs:1866-1926` vs `1969-2005`: identical dig/drop/pick resolution ~40 lines twice)
inside a 375-line function with a 13-tuple destructured clone (`sim.rs:1813`). Fix
during the split: one `resolve_intents` used by both castes; the queen keeps only her
flight/found specials.

**🟡 B7. Ten hand-rolled `set_*` mutators** (`set_state/set_job/set_pending/set_retry/
set_attack_after/set_dig_after/set_drop_after/set_pick_after/set_land_after/
set_found_after`) — borrow-checker boilerplate. Collapse into one `with_ai(id, f)` +
`set_state`; also removes the "did I remember all ten" failure mode.

**🟡 B8. Unchecked indexing at trust boundaries.** `Grid::get/set` panic on out-of-bounds
(`world.rs:37-43`); `soil_at` indexes directly (`sim.rs:1031-1037`); `Config` is
unvalidated (width 0 → overflow panics in worldgen). Every current caller happens to
clamp first — that's accident, not design. Fix: `debug_assert` in `Grid::get/set`,
validate `Config`, and give `soil_at` a checked variant used where coords come from
positions.

**🟡 B9. Layer-blind / layer-hardcoded helpers.** `food_on_tile` ignores layer (its only
caller guards surface, today); `spawn_food` hardcodes `Layer::Surface` in its cap logic;
`spawn_source` hardcodes surface. Cheap now, a trap when underground food matters
(spoiling drops already exist underground!). Fix: explicit layer parameters everywhere.

## 4. Performance shortcuts (fine at 60 entities, wrong for Phase 4)

**🔵 C1. Per-tick id-list allocations inside nested loops.** `ant_ids()/food_ids()/
egg_ids()` allocate a Vec per call and are called per candidate tile: `cell_food` runs
per ring tile in `pantry_tile` per idle carrier per tick; `store_food` scans all food per
delivery; `discover` is O(ants×sources) with a Vec per tick; `best_food` per idle ant.
Fix now (cheap, contained): food entities are static — maintain a
`BTreeMap<(layer, tile), BTreeSet<id>>` index updated on spawn/despawn only (BTreeMap
keeps iteration deterministic). The ants/spiders counts get the same treatment only if
the numbers ever justify it.

**🔵 C2. `AntState` cloned per ant per tick** (`worker_ai`, `movement`, `digging`,
`combat` each `(*q).clone()` — including the `Vec<Vec2>` path) — a 20 tps allocation
machine born of query-borrow juggling. Fix where natural during the split (movement can
take `&mut` through `query_one_mut` with the state swapped via `mem::replace`); the
remaining clones get a comment until Phase 4 profiling says more.

**🔵 C3. Full tile grids copied across the WASM boundary every tick.**
`sim.ts:239-262` pulls `tiles_surface()/tiles_underground()` (2×9 KB) and byte-compares
them in JS, 20×/s, forever — because the core has no way to say "tiles changed". Fix:
core keeps a `tiles_epoch` (and `soil_epoch`) counter bumped by the only mutators
(dig, found, refill, dev ops); the client re-pulls only when the epoch moves. Same
answer will apply to the Phase-4 network protocol.

**🔵 C4. Event log uses `Vec::remove(0)`** at cap — O(n) shift of 400 strings per event.
`VecDeque` (trivial, included in the split).

## 5. Hygiene

- ⚪ `client/farm6.tmp.mjs`, `client/sync.tmp.mjs` — throwaway Playwright scripts,
  **committed**. Delete.
- ⚪ `TODO.md.save` — editor backup, untracked (gitignored pattern); delete.
- ⚪ `determinism_dump.rs:104-108` — dead no-op closure + no-op match from an old
  refactor. Delete.
- ⚪ `lib.rs:84-90` — `cmd_dump` kept as a pure alias of `cmd_drop`; the replay format
  carries `dump` as an act name. Keep the alias but route the *act name* through one
  constant so the duplication is documented, not accidental.
- ⚪ `sim.ts:268` — `food` field actually means carbs (`// legacy field name`), and the
  e2e reads `s.food`. Rename to `carbs` everywhere (client state dump too).
- ⚪ Tests and e2e assert raw kind/aux codes (`e.kind == 3`, `aux == 2.0`) — the same
  smell as A1, one layer down. Updated to the typed accessors as part of Option C.
- ⚪ README "perf" note claims a "fixed per-frame entity Graphics rebuild" cost center;
  the code actually change-guards redraws (`render.ts:412-427`). Stale doc — fixed with
  the perf numbers after this wave's verification.

## 6. Gameplay observations (recorded, not code-fixed this wave)

Design notes surfaced by the audit — for Wave B decisions, not refactor items:

- Idle workers never fight back: spiders slaughter afk workers unless the player
  intervenes (`worker_ai` has no combat response; `attack_after` is player-only). The
  original auto-attacks in range. Likely Wave B (worker AI combat stance).
- `best_food` prioritizes protein over carbs regardless of colony state
  (`sim.rs:1517-1521`) — with 90 s starvation this can walk the colony into the red.
  Tied to the open survival-balance question.
- `Job::Fetch` delivers after exactly one harvested unit — round trips per unit;
  deliberate? (feels like a placeholder for a carry-capacity upgrade).
- Spider aggro (5 tiles) vs ant sight (8 tiles) means scouts see the spider first but
  can't warn anyone — pheromone-shaped hole, noted in the undecided list already.

## 7. What is deliberately NOT changed

- The tick system order (`worker_ai → movement → discover → digging → combat →
  predators → cleanup_deaths → queen_system → eggs → cleanup`) — any reorder changes
  every seed's future; the split preserves it exactly.
- The compact float-array transport across WASM (it becomes a *positional wire format*
  decoded at one typed boundary, which is the Phase-4 protocol seed).
- Legacy economy semantics (see B3).
- Determinism architecture, replay format v2, e2e harness — all healthy.

---

## Addendum — found while applying the fixes (2026-09-26)

Three more defects surfaced during the fix wave, all fixed in the same commits:

- **🔴 Pantry-saturation deadlock** (new A-number: A7). Once the physical pantry
  (6 units/cell) fills the nest, carriers try to dig expansions — but
  `pick_dig_target` only searched radius 3 around the queen. Past that zone the
  colony starved beside uncollected surface food, with every carrier looping on
  a 100-tick retry. The inclusive-`irange` worldgen shift exposed it on legacy
  seed 7 (`delivered` froze at the identical pantry capacity across different
  worlds — the tell). Search radius is now `DIG_EXPAND_RADIUS` (balance.rs, 8).
- **🔴 The queen dropped attack orders on successful routes** (A8). Her attack
  resolution cleared `attack_after` when the cross-layer route *succeeded*: she
  surfaced, walked to where the spider had been, and stood idle until it killed
  her. The intent now persists like the worker branch's and re-engages on
  arrival (regression: `attack_intent_survives_a_routed_chase`).
- **🟠 Underground combat could never start** (deepens A6). The AI only set
  `Fighting` for surface/surface pairs — an underground attack order routed to
  the target's tile forever without engaging; and had it engaged, the chase
  moved in straight lines through walls. Engagement is now same-layer, chases
  below ground pathfind, and `attack_after` persists through `Fighting` so
  routed chases re-engage. (No underground hostile exists yet — Wave B
  provides the live test.)

Also observed, recorded for Wave B design (not code-fixed): the e2e perf
sample right after a fast-forward batch reads low (the 0.5 s window catches
the batch tail) — a perf-note hazard, not a regression; a clean-probe reading
(30 fps live / 36 paused, 49 entities, SwiftShader) matches the old baseline.
