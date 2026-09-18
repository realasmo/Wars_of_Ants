# Wars of Ants

A browser-based ant colony game: multiplayer colony simulation + RTS + survival + ecosystem.
Design reference: AntWar.io (see `docs/BASED-ON.md`).

## Repository

- **Remote (origin):** `git@github.com:realasmo/Wars_of_Ants.git`
- **SSH key:** `/root/.ssh/key_Wars_of_Ants` (configured via repo-local `core.sshCommand`)

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

### Combat & content (confirmed, Phase 3 wave 1)
- **Combat model:** original-style melee — HP / damage / attack cooldown / speed per unit; click an enemy to lock on and auto-attack in range.
- **Stats (wave 1):** Worker 100 HP / 8 dmg / 1.0s CD / speed 3.0 · Soldier 130 / 22 / 1.0s / 2.6 · Queen 150 HP · Spider 130 / 15 / 1.2s / 2.2 (aggro 5 tiles, wanders near lair, surface only).
- **Predators:** spiders (2 per map) wander the surface, attack ants in aggro range.
- **Super Food (blue):** dropped by killed spiders (8 units); Soldier eggs cost 2 Super + 3 Green; soldiers produced automatically while Super is available, capped at 1 soldier per 2 workers.
- **Fog of war:** removed for now (was implemented in wave 1) — may return later, likely server-authoritative for multiplayer.

### Roadmap (confirmed)
0. ✅ **Repo foundation** — monorepo (`core/`, `server/`, `client/`, `docs/`), toolchains (cargo + wasm-pack + Vite), CI, initial push.
1. ✅ **Core simulation** (Rust, headless) — deterministic fixed-timestep ticks (20 tps), hecs ECS, seeded PCG RNG, two-layer tilemap + digging, A* pathfinding (dig-aware costs, 8-directional with no corner cutting), Queen/Worker economy (gather → feed → eggs → hatch), starvation/queen-death loss condition, command API (`Move`, `Dig`), WASM bindings, headless sim tests (determinism, colony growth, starvation, dig).
2. ✅ **First playable** — PixiJS client with placeholder/procedural art, WASM core in-browser (`LocalTransport`), HTML/CSS HUD, camera (drag/WASD/zoom), layer toggle (Tab), click-to-command (LMB move/select, RMB dig), ant takeover/cycling (C), queen-death game over + restart. *Milestone 1 reached: dig, gather, grow the colony, queen death = game over.* Includes AI nest expansion (workers dig new chambers when brood space runs out).
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
- URL params: `?seed=N` fixed world seed, `?e2e=1` preserve canvas for pixel reads.

### Input recorder (live debugging)

- The client keeps a ring buffer (last 500 events) of everything the player did: raw gestures (`click`, `wheel`, `pan`, `key` — screen + world coords and camera state) and resolved commands (`cmd`: `move`/`dig`/`attack`/`entrance`/`select`/`cycle`), plus `start`/`restart`/`view`/`death`/`mark` markers. Every event is stamped with the sim tick.
- Dump from the browser console: `__woa.log()`. A hidden DOM mirror (`#inputlog`, updated on every event) can be read where JS eval is sandboxed.
- `__woa.mark('label')` drops a labeled marker into the log (e.g. "bug here").
- A dump maps 1:1 onto `__woa.click/key/step` calls, so a recorded play session converts mechanically into an e2e regression scenario.

### Replays

- A replay is `{version, seed, cmds}` — the seed plus sim commands stamped with the tick they fire (a command applies after the first tick that reaches its `t`). Determinism makes this a complete record: same seed + same commands = same game.
- Export a live session from the console: `__woa.replay()` (built from the input recorder's command log). Exports are stamped with the core build (`core`); loading a replay recorded on a different core warns in the console — sim changes (e.g. the 8-directional movement change) mean old replays no longer reproduce their original game.
- Watch one: put `name.json` in `client/public/replays/` and open `?replay=name` — the game runs it at normal speed with a REPLAY badge. Replays are watch-only: camera pan/zoom, layer toggle and ant selection stay live, but sim commands (move/dig/attack/entrance) are blocked while the badge is up.
- `client/public/replays/determinism.json` is the fixture for the determinism test (below).

### Cross-platform determinism test

- `Sim::canonical_state()` (core, exported to WASM) renders the whole sim state into a platform-independent string. `cargo run --example determinism_dump -- <replay.json>` prints it for the native build.
- The e2e suite runs the *same* replay file in the browser (WASM) and compares the canonical strings — any character difference is a native-vs-WASM determinism break. Run it via `npm run e2e` (see "Automated playtesting").
- Verified matching as of 2026-09-18 (seed 42, 3000 ticks, mixed move/dig/entrance/attack commands).

### Versioning rule

- **Every format that crosses a build, process, or network boundary gets a version number from day one.** Currently versioned: replay format (`version: 1`). Coming before first use: the binary snapshot/protocol (Phase 4), save files, and the WASM↔JS interface (`core_version()` already reports the crate version — surface it in the client HUD/debug output when it matters).

### Balance table

- All tunable gameplay numbers (unit HP/damage/cooldown/speed/range per caste and predator, economy constants) live in `core/src/balance.rs` — data only, no logic. Sim code reads them via `stats_for(caste)`; changing a number there is a balance change, not a code change.

## Undecided (vs. the original game)

The following decisions are **not yet made**. `docs/BASED-ON.md` describes the original game as a reference only — nothing there is a commitment.

- [x] Fighting system design — **decided: original-style melee** (see Combat & content)
- [ ] Ant castes beyond Queen/Worker — Soldier added in wave 1; Nanitic, Major, Acid Ant, Alate still undecided
- [ ] Food types and economy details — Green + Super in wave 1; Meat and Red Food undecided
- [ ] Whether to re-enable the pheromone system the original disabled
- [x] Fog of war — was in wave 1, **removed for now** (may return, server-authoritative, with multiplayer); day/night and weather parameters still undecided
- [ ] Day/night cycle and weather parameters
- [ ] Modes to support beyond singleplayer sandbox and co-op multiplayer (teams, private games, PvP)

> Notes and decisions go into the "Ideas / Changes / Improvements over the original" section of `docs/BASED-ON.md` and, once confirmed, get promoted into this README.
