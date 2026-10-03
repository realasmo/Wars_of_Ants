# Wars of Ants

A browser-based ant colony game: multiplayer colony simulation + RTS + survival + ecosystem.
Design reference: AntWar.io (see `docs/BASED-ON.md`); world content draft: `docs/WORLD-DESIGN.md`.

## Repository

- **Remote (origin):** `git@github.com:realasmo/Wars_of_Ants.git`
- **SSH key:** `/root/.ssh/key_Wars_of_Ants` (configured via repo-local `core.sshCommand`)

## Current status (2026-10-03) — version 0.1.06.61-dev

**Playable:** title → team select → flying queen → found nest → **water quest**
(she farms water outside, stores it on a food block — silver soil, dug or
built from wet wood — then stands on an egg block — orange soil — to lay the
founding brood; 4-minute incubation) → first workers → excavate
(2×2 blocks, dirt hauling) → harvest sighted finite sources
(protein/carbs/water/honeydew; foragers prioritize what the queen craves) →
survive spiders → **squad play**
(X-menu: recruit/release followers, silver rings, released ants resume
their interrupted work; a leader who farms **converts the squad** — they
keep farming until re-recruited; an attack order is shared — the whole
squad fights that enemy, then returns to you; a leader under attack calls
the squad to **retaliate**) → **physical queen feeding** (one feeder worker serves her
rotating cravings; the request shows above her and in the HUD) →
**brood production** (X as the queen: order Worker/Soldier/Honey/Medic
eggs for physical pantry costs — protein + sugars; soldier orders consume
a worker; honeydew from nettle sources and Honey ants; medics rescue
downed ants for water). **Typed food blocks:** one 2×2 storage block holds
15 units of a single food type. Engine: deterministic 20 tps core (native +
WASM byte-identical), replays (v2), e2e playtest suite, in-game dev console
(`` ` ``) with a live sim event log. Every game change bumps `GAME_VERSION`
(menu + corner).

**Ants are procedural and alive** (ticket `docs/TICKET-procedural-ants.md`,
shipped): part-based rigs — planted-feet tripod gait (feet never slide),
two-bone IK legs with knees, per-caste silhouettes and per-team palettes,
idle micro-motion (antennae, breathing, head turns), carrying between the
mandibles, a mandible state machine (idle closed / alert flare + bite snap
in combat / parted to item width while carrying / dig + harvest work
rhythms), queen wings and wing scars. Live art set: **Camponotus**
(carpenter ants — worker/soldier/queen readable at a glance; intake
`docs/ant-proportions-intake-camponotus.md`; re-skinning a species =
editing `client/src/art/ants.ts` numbers).

**User TODO batch (2026-09-27) — F1–F3 shipped:**
- **F1:** harvesting is a real visible activity (mandible work + head
  peck); the queen can farm sources and bank on silver soil.
- **F2:** map collectibles — wet wood builds a food-storage block, dry
  wool an egg-friendly block (haul, place, reversible).
- **F3:** squad follow — worker/soldier X-menu (all-in-sight / one /
  soldiers / release), followers keep near you and cross layers, silver
  squad rings, replay-logged commands.
- **F4 (shipped 2026-09-28):** queen X-menu brood production — Worker 2p+1w,
  Soldier 6p+3w + consumes a worker, Honey 1p+8h, Medic 4p+3c, paid from
  the physical pantry; founding softlock fixed (re-order workers). Fourth
  resource **honeydew**: nettle map sources + Honey ants secreting 1/4min.
  **Medics**: combat downs caste ants (60s bleed-out), auto-rescue, heal
  for 2 stored water, revive at full hp. **Retaliation**: the squad turns
  on whoever hits the leader. Foundation: `GameRules` (rules.rs) — all
  tunables now one typed, validated, digest-hashed dataset on the sim (the
  admin tuning panel's substrate). Wire layout v4.

**Worker-priorities wave (ticket `docs/TICKET-worker-priorities.md`):**
the queen eats **real food** now — a rotating craving cycle
(carbs→protein→carbs→water), one designated **feeder** worker that
withdraws her craved unit from the pantry and stands by her until she is
hungry (reassigned when controlled, recruited-then-released feeders resume
feeding; a hungry queen can also eat her own farmed load). Starvation
becomes visible: her request shows above her (red + imperative when the
fuse burns) and in the HUD row. Founding reserves are a physical carb pile
in the chamber. Auto-scouting is gone: taskless workers drift where they
were left, then return to rest in the nest and periodically glance around
the nest mouth (which keeps near-nest sources discoverable) — far sources
are found by the player. Followers **resume their interrupted job** on
release.

**Playtest-fix wave:** worker digging now obeys dirt capacity and
auto-hauls spoil to the surface; eggs can be carried through the entrance;
the carb-low slowdown is gone and the founding grace also suspends eating
(the reserves now survive to meet the first workers — the queen-death
death-spiral is fixed); refused actions explain themselves in the help
bar; HUD shows carried items and the map size.

**Code-quality wave shipped (see `docs/AUDIT.md`):** `sim/` module split;
typed snapshot layer (core `Carry` enum + client `decode.ts` boundary +
boot wire-spec assertion); true full-state canonical digest; irange,
pantry-deadlock, queen retry-freeze, dropped attack intents, friendly
fire, underground chase fixes; food index + epoch-gated re-pulls.

**Admin tuning + activity indicators (ticket
`docs/TICKET-admin-indicators.md`, shipped 2026-09-28):** every tunable is
live-editable in-game — `?admin=1` or **F4** opens a Tweakpane drawer over
`GameRules`, generated from the core's field registry (groups, per-field
live vs new-game scope), committing whole-object and atomically: the core
validates everything, applies live fields immediately (entity stats
re-derived, hp fraction preserved) or lists every violation; "apply & new
game" restarts under the edited rules with the same seed. Commits are
logged (`dev-rules`) and replay byte-identically (digest in the canon).
Every player ant floats a **pixel-art intent icon** (always on, never
smaller than 16 screen px) —
per-resource harvest droplets (tinted), dig, spoil-haul, haul-home, fight,
medic, feeder crown, follow reticle, honey production, off-duty Zz — and a
red **!** with an honest core-derived reason (empty pantry, medic without
water, pantry full, queen starving); a gold pulse confirms every order.
Refused digs explain themselves (soldiers can't dig). Wire layout v5.

**Data-files wave (ticket `docs/TICKET-json-data.md`, shipped 2026-10-03):**
the shipped balance now lives in **`data/*.jsonc`** at the repo root
(`ants` / `resources` / `world` / `colony`) — JSON with comments, embedded
into the core at compile time (`include_str!`) so browser, native tests,
and replays read the same bytes and can never drift. `GameRules::default()`
delegates to the loader; the hand-written default literals are gone.
`dirt_capacity` moved from one global into each caste's stats (worker and
founding queen dig; soldiers/honey/medic carry an honest 0). The comment
stripper runs in `parse_rules` too, so the admin drawer's JSON box accepts
annotated documents; comments never reach the rules digest. Workflow:
edit files → `npm run wasm` → restart vite → hard reload (or tune live in
F4 and copy the JSON over). 96 core tests.

**Decided next:** wave C — neutral creatures (earthworm/snail/rove beetle,
see `docs/WORLD-DESIGN.md`), then D (aphid farming), E (bosses); drafts and
costs in `docs/WORLD-DESIGN.md`. The visual/lighting ticket menu (baked AO
first) follows the content waves; one effect per ticket, rendering-only.

**Open balance questions from playtesting:** founding lottery (natural
orange in the starter chamber ~17% of seeds); orange/silver density
(3.5% each); sparse-source survival sharpness (knobs: `SOURCES`,
`START_FOOD`, `STARVE_TIME`); do soldiers dig; should clicks auto-route
across layers.

## Why this project exists

AntWar.io is a great proof of concept, but it is not perfect and no longer maintained. This project is an independent alternative: same genre and core fantasy, rebuilt with a modern stack and room to fix the original's shortcomings.

## Confirmed project goals

### Vision
- 2D WebGL game running in the browser — no engine download, no Unity WebGL.
- Players control individual ants within a colony; the rest of the colony runs on autonomous AI.
- Two-layer world: surface (ecosystem, food, threats) and underground (digging, nest building, brood).
- Queen survival as the core win/loss condition.
- Ecosystem-driven strategy: strategic value emerges from the living world (predators, aphids, weather), not from placed "resource nodes".

### Target architecture (confirmed)
- **Client:** TypeScript + PixiJS (WebGL 2D renderer) + Vite (build/dev) + Howler.js (audio) + HTML/CSS overlay UI (menus/HUD not drawn in canvas).
- **Shared game core:** a Rust crate compiled twice — natively for the server, to **WASM** (`wasm-bindgen`) for the browser. TS handles rendering, input, and UI; simulation (tiles, pathfinding, combat, AI) lives in the core. This guarantees an identical sim for singleplayer and multiplayer — no logic drift between implementations.
- **Game server:** Rust, authoritative — **tokio** + **Axum** (WebSocket + HTTP, reusable for matchmaker), **hecs/bevy_ecs** for ECS, spatial hash, pathfinding, AI, combat, resource system, colony system, networking.
- **Protocol:** binary snapshot synchronization (**postcard/bincode**-style packing; original uses raw ArrayBuffer/DataView — same idea), no full JSON per tick.
- **Supporting services:** matchmaker, database, metrics.
- **Transport pattern:** one shared game core with two transports — `LocalTransport` (singleplayer: core running client-side via WASM) and `NetworkTransport` (multiplayer: authoritative server).

### Platform positioning
- Multiplayer colony simulation + RTS + survival + ecosystem.
- Browser-based 2D WebGL + WebSocket is sufficient and proven (working proof: AntWar.io itself).

### Starting content (confirmed, minimal test slice)
- **Queen** and **Worker** ants only — enough to validate the dig → gather → feed → brood loop.
- All further content is deliberately deferred and decided through Phase 3 playtesting: additional castes (Nanitic, Soldier, Major, Acid Ant, Alate), the fighting system, food types and costs, predators, pheromones.
- Goal of the test slice: prove the architecture end-to-end, not to ship content.

### Game start: founding (confirmed)
- On load: title screen → **PLAY** → **team choice (red / blue — cosmetic for now, stored in the sim/replay)** → the game starts with **only a flying queen** above the surface.
- **Flight:** hold LMB to fly (queen speed 5.0, faster than any ant); **right-click to land** where she hovers. Spiders cannot touch her while airborne.
- **Grounded:** the queen walks (speed 2.5, slower than workers); **right-click founds the nest at the tile she stands on** (refused within ~3 tiles of the map edge — the starter chamber must fit). Founding digs the entrance shaft + a 3×3 chamber and the camera follows her underground.
- **Founding window (60s, countdown in the HUD):** the queen excavates — right-click digs a dirt cell (1s each) and the spoil becomes a **carried dirt unit**: dump it on an empty underground cell to *refill* that cell, or haul it up through the entrance and drop it above ground where it vanishes. Real nest growth means hauling dirt out. She digs until the first workers hatch.
- **First brood:** when the timer ends she lays **4 eggs** (fewer if the chamber is too small — excavate!); they hatch **3 minutes** later into 4 controllable workers → standard colony gameplay. Ongoing egg production is **not** part of this wave (the founding script replaces auto-laying; production design is a later decision).
- **Founding grace:** starvation is suspended while the colony has no workers; the queen is still on her own against spiders once grounded (her death by any means ends the colony).
- All founding numbers live in `GameRules` (`core/src/rules.rs` — editable live via the F4 admin drawer).

### World content (design draft — see docs/WORLD-DESIGN.md)
Creatures (worker/soldier/honey/medic/queen castes, neutral earthworm/snail/rove
beetle, hostile bosses TBD), the resource economy (protein / carbohydrates /
water / aphid honeydew, finite map resources, aphid farming with a dedicated
chamber) and worker scouting flows are drafted in `docs/WORLD-DESIGN.md` and
gate into the roadmap through playtesting, phase by phase.

### Nest soil & food logistics (confirmed)
- **Digging is 2×2:** one dig action removes an aligned 2×2 block (all four cells must be dirt); one carried dirt block refills one fully-empty block — or vanishes dropped above ground. Right-clicking distant dirt **walks the ant there and digs on arrival** (the reported bug — fixed).
- **Soil quality, hidden under the dirt:** each underground 2×2 block has a chance (~3.5% each) of being **orange** or **silver**, revealed by digging. The surface shows 2 orange + 2 silver **dust patches** (~10×12): founding the nest inside one grants 3–6 hidden soil blocks of that color near the nest.
- **Orange = nursery.** Eggs transform into ants **only on empty orange cells** — ready eggs wait indefinitely until carried there. Any ant can pick up an adjacent egg (right-click) and place it (right-click); the hatch state is preserved.
- **Silver = pantry.** Food that ants pick up and drop spoils after 5 minutes on a non-silver cell (the timer freezes while on silver); worldgen piles never spoil. Per-cell cap: **6 food units** (any mix) — bigger piles spread across neighboring cells.
- **Physical pantry:** workers place collected food on the **nearest silver cell** to the entrance (first free cell if none exists yet) as a visible, safe pile; placement credits the food store, and when the pantry fills up the carriers dig out more nest space. The queen still eats from the abstract store (physical eating is undecided).
- All knobs live in `GameRules` (`core/src/rules.rs` — `ORANGE/SILVER_SOIL_CHANCE`, `PATCH_*`, `FOOD_CELL_CAP`, `SPOIL_TIME`, ...; F4 tunes them live). Dev: `__woa.setsoil(layer, x, y, 0|1|2)` paints a block (replayable `dev-soil` act).

### Resource economy (founding worlds)
- Three colony stores: **protein** (ant spawning — spiders drop 8; cockroach
  and caterpillar sources), **carbs** (the keep-alive store: the queen eats
  it, starvation, and below `CARB_LOW` the whole colony moves at 60% until
  fed), **water** (healing later; moss/mushroom sources).
- The map has **no green food**: six **finite source types** (moss, mushroom,
  raspberry, strawberry, cockroach, caterpillar) scattered as single finds —
  spec table in `GameRules.sources` (`core/src/rules.rs`) and `docs/WORLD-DESIGN.md`.
  Sources never respawn; depleted ones vanish.
- **Scouting:** sources count as known only after a colony ant sights them
  (8 tiles). Idle workers with nothing known wander outward from the nest
  (Job::Scout) instead of idling. Harvesting a source takes per-unit time
  (strawberry 4 s … mushroom 20 s) — ants stand and work the source.
- Pantry rules (silver preservation, 6 units/cell, visible piles, dig-to-
  expand when full) apply to all three resources; the queen eats carbs.
- Legacy founded worlds (test economy) keep green/super food semantics.

### Combat & content (confirmed, Phase 3 wave 1)
- **Combat model:** original-style melee — HP / damage / attack cooldown / speed per unit; click an enemy to lock on and auto-attack in range.
- **Stats (wave 1):** Worker 100 HP / 8 dmg / 1.0s CD / speed 3.0 · Soldier 130 / 22 / 1.0s / 2.6 · Queen 150 HP · Spider 130 / 15 / 1.2s / 2.2 (aggro 5 tiles, wanders near lair, surface only). Values live in `GameRules` (`core/src/rules.rs`) — see "Balance table".
- **Predators:** spiders (2 per map) wander the surface, attack ants in aggro range.
- **Super Food (blue):** dropped by killed spiders (8 units); Soldier eggs cost 2 Super + 3 Green; soldiers produced automatically while Super is available, capped at 1 soldier per 2 workers.
- **Fog of war:** removed for now (was implemented in wave 1) — may return later, likely server-authoritative for multiplayer.

### Roadmap (confirmed)
0. ✅ **Repo foundation** — monorepo (`core/`, `server/`, `client/`, `docs/`), toolchains (cargo + wasm-pack + Vite), CI, initial push.
1. ✅ **Core simulation** (Rust, headless) — deterministic fixed-timestep ticks (20 tps), hecs ECS, seeded PCG RNG, two-layer tilemap + digging, A* pathfinding (dig-aware costs, 8-directional with no corner cutting) smoothed into straight line-of-sight segments (string pulling; dig stops preserved), Queen/Worker economy (gather → feed → eggs → hatch), starvation/queen-death loss condition, command API (`Move`, `Dig`), WASM bindings, headless sim tests (determinism, colony growth, starvation, dig).
2. ✅ **First playable** — PixiJS client with placeholder/procedural art, WASM core in-browser (`LocalTransport`), HTML/CSS HUD, camera locked to the controlled ant (AntWar-style): **hold LMB to steer your ant toward the cursor**, left-click an ant to take control, RMB for context commands (dig / attack spider / enter-exit the marked nest hole), wheel zoom, ant cycling (C). Spectating (controlled ant dead): free camera (drag, WASD, Tab layer toggle). Queen-death game over + restart. *Milestone 1 reached: dig, gather, grow the colony, queen death = game over.* Includes AI nest expansion (workers dig new chambers when brood space runs out).
3. **Combat + ecosystem + AI** — predators, day/night, weather, fog of war; remaining content decisions are made and playtested here.
4. **Server + multiplayer** — `NetworkTransport`, authoritative Axum server, binary snapshots, rooms/lobby, co-op.
5. **Meta + release** — matchmaker, DB persistence, metrics, deployment, real art pass, balance.

Build order: singleplayer first. Art: placeholder/procedural until the gameplay is fun.

### Running it
- Prereq: build the WASM core once — `cd client && npm run wasm`
- Dev: `cd client && npm run dev` → open the printed localhost URL
- Prod: `cd client && npm run build` → static files in `client/dist/` (deploy to any static host)

### Automated playtesting (e2e)
- `cd client && npm run e2e` — boots the real client in headless Chromium (system `chromium` package), drives it through a scripted scenario via an in-page debug hook (`window.__woa`, only alongside `?e2e=1`): world-coordinate clicks, key toggles, sim fast-forward, state dumps, canvas pixel probes.
- Reproducible: `WOA_SEED=42 WOA_PORT=5199 WOA_OUT=/tmp/... npm run e2e`.
- Screenshots + JSON state dumps land in `/tmp/opencode/woa-shots` (screenshots are for humans; assertions are state/pixel-based).
- URL params: `?seed=N` fixed world seed, `?e2e=1` preserve canvas for pixel reads, `?admin=1` open the rules drawer, `?perf=1` perf overlay, `?replay=name` watch a replay.

### Input recorder (live debugging)

- The client keeps a ring buffer (last 500 events) of everything the player did: raw gestures (`click`, `wheel`, `pan`, `key` — screen + world coords and camera state) and resolved commands (`cmd`: `move`/`dig`/`attack`/`entrance`/`select`/`cycle`), plus `start`/`restart`/`view`/`death`/`mark` markers. Every event is stamped with the sim tick.
- Dump from the browser console: `__woa.log()`. A hidden DOM mirror (`#inputlog`, updated on every event) can be read where JS eval is sandboxed.
- `__woa.mark('label')` drops a labeled marker into the log (e.g. "bug here").
- A dump maps 1:1 onto `__woa.click/key/step` calls, so a recorded play session converts mechanically into an e2e regression scenario.

### Replays

- A replay is `{version, seed, cmds}` — the seed plus sim commands stamped with the tick they fire (a command applies after the first tick that reaches its `t`). Determinism makes this a complete record: same seed + same commands = same game.
- **Format v2** (founding update): adds `team` (red/blue) and the acts `land` / `found` / `dump`. The loader also accepts v1 with a console warning — v1 replays predate the founding start and no longer reproduce their original game.
- Export a live session from the console: `__woa.replay()` (built from the input recorder's command log). Exports are stamped with the core build (`core`); loading a replay recorded on a different core warns in the console — sim changes mean old replays no longer reproduce their original game.
- Watch one: put `name.json` in `client/public/replays/` and open `?replay=name` — the game runs it at normal speed with a REPLAY badge. Replays are watch-only: camera pan/zoom, layer toggle and ant selection stay live, but sim commands (move/dig/attack/entrance/land/found/dump) are blocked while the badge is up.
- `client/public/replays/determinism.json` is the fixture for the determinism test (below). It is regenerated by the e2e suite from each run's founding session.

### Cross-platform determinism test

- `Sim::canonical_state()` (core, exported to WASM) renders the whole sim state into a platform-independent string. `cargo run --example determinism_dump -- <replay.json>` prints it for the native build.
- The e2e suite runs the *same* replay file in the browser (WASM) and compares the canonical strings — any character difference is a native-vs-WASM determinism break. Run it via `npm run e2e` (see "Automated playtesting").
- Verified matching as of 2026-10-03 (full e2e session fixture, 10706 ticks, incl. a dev-rules commit; regenerated by every e2e run).

### Versioning rule

- **Every format that crosses a build, process, or network boundary gets a version number from day one.** Currently versioned: replay format (`version: 2`), the wire snapshot (layout v5 via `snapshot_spec()`), and the game itself — `GAME_VERSION` in `core/src/lib.rs` (single source; `game_version()` serves it to the client, shown in the main menu and the bottom-left in-game corner). Scheme: `MAJOR.MINOR.WAVE.BUILD-dev` — BUILD bumps on EVERY game change however slight (same commit as the change), WAVE bumps per shipped feature wave. Coming before first use: save files and the Phase-4 binary protocol.

### Balance table

- All tunable gameplay numbers live in **`data/*.jsonc`** (repo root — see `data/README.md`): JSONC files embedded into the core at compile time, one source of truth for the shipped balance. The typed, validated schema (`GameRules`, `core/src/rules.rs`) defines the fields and invariants; the loader merges the files and fails loudly on unknown/missing/duplicate keys. Sim code reads values via `stats_for(caste)`. Tune live with the F4 admin drawer for experiments, or edit the files (+ `npm run wasm`, restart vite, hard reload) for permanent changes — a data edit is a balance change, not a code change.

### Dev tools (F2, F3, F4)

- **F4 opens the admin rules drawer** (also `?admin=1` on load) — see the
  "Admin tuning" paragraph above. The whole form is generated from the
  core's field registry; `◇ng` marks fields that only apply on a new game.
  `__woa.rulesGet/rulesSet/rulesRestart` drive the same surface from
  devtools/e2e; `__woa.icons()` reads the intent-icon layer state.
- **F2 opens the dev panel**: pick an entity (worker / soldier / egg / spider / food / super food), then click the map to place it. Quick actions: food = 50, super = 5, kill spiders, pause, +10s fast-forward. Esc cancels placement. Ants and eggs spawn underground; spiders and food on the surface.
- **In-game console (`` ` ``):** a half-transparent console rolls down from the top — live event feed (colony events with causes: births, deaths and why, egg laid/ready/hatched, sources discovered/depleted, deliveries, pantry expansions, carb slow on/off, colony death) plus a command line (`help` lists everything: spawn/setfood/setsuper/setwater/soil/kill/killspiders/pause/step/coords/state/canon/seed/events/clear). Arrow-up recalls history. The event log lives in the sim (capped 400, observational only — never affects gameplay or determinism).
- **Coords tool** (F2 → coords): live x/y + tile of your ant in a corner readout; every click logs its world position into the console.
- **Console API** (devtools, with the game open): `__woa.spawn('spider', x, y)` (omit x/y for the nest entrance), `__woa.setfood(n)`, `__woa.setsuper(n)`, `__woa.kill(id)`, `__woa.killspiders()`, `__woa.pause()`.
- Dev operations live in the Rust core (`Sim::dev_spawn/dev_set_food/dev_set_super/dev_kill`) — so they are deterministic, recorded in the input log (act `dev-*`), and replayable like any command. Dev actions are blocked while a replay is running. Killing the queen via dev tools flags the colony dead (the sim dereferences the queen id every tick).
- `__woa.state()` includes `foods` (food pile count) and `paused` for test assertions.
- **F3 toggles the perf overlay** (`?perf=1` also enables it on load): fps, avg/worst frame ms (0.5s window), measured sim ticks/s, entity total vs shown-on-layer, zoom. Baseline headless (SwiftShader): ~30–36 fps with ~50 entities and flat scaling (clean-probe reading after the typed-snapshot wave; entity redraws are change-guarded, tile redraws epoch-gated). The sim holds 20 tps regardless. Caveat: a perf sample taken right after a fast-forward batch (`step(n)`) reads low — the 0.5s window catches the batch tail.

## Undecided (vs. the original game)

The following decisions are **not yet made**. `docs/BASED-ON.md` describes the original game as a reference only — nothing there is a commitment.

- [x] Fighting system design — **decided: original-style melee** (see Combat & content)
- [ ] Ant castes beyond Queen/Worker — Soldier added in wave 1; Nanitic, Major, Acid Ant, Alate still undecided
- [ ] Ongoing brood production after founding (the founding script lays exactly 4 eggs; production design — costs, castes, menu — is a later decision)
- [ ] Dirt hauling for workers (the founding queen hauls dirt; worker digging is still instant-spoil)
- [ ] Does the queen eventually eat the physical pantry food (vs the abstract store)?
- [ ] Should worker AI also seek orange cells for eggs / silver for placement proactively?
- [ ] Food types and economy details — Green + Super in wave 1; Meat and Red Food undecided
- [ ] Whether to re-enable the pheromone system the original disabled
- [x] Fog of war — was in wave 1, **removed for now** (may return, server-authoritative, with multiplayer); day/night and weather parameters still undecided
- [ ] Day/night cycle and weather parameters
- [ ] Modes to support beyond singleplayer sandbox and co-op multiplayer (teams, private games, PvP)

> Notes and decisions go into the "Ideas / Changes / Improvements over the original" section of `docs/BASED-ON.md` and, once confirmed, get promoted into this README.
