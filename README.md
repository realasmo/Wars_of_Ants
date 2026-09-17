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
- **Fog of war:** yes — vision radius ~10 tiles on surface, ~5 underground; rendered client-side for now (moves server-side for multiplayer later).

### Roadmap (confirmed)
0. ✅ **Repo foundation** — monorepo (`core/`, `server/`, `client/`, `docs/`), toolchains (cargo + wasm-pack + Vite), CI, initial push.
1. ✅ **Core simulation** (Rust, headless) — deterministic fixed-timestep ticks (20 tps), hecs ECS, seeded PCG RNG, two-layer tilemap + digging, A* pathfinding (dig-aware costs), Queen/Worker economy (gather → feed → eggs → hatch), starvation/queen-death loss condition, command API (`Move`, `Dig`), WASM bindings, headless sim tests (determinism, colony growth, starvation, dig).
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

## Undecided (vs. the original game)

The following decisions are **not yet made**. `docs/BASED-ON.md` describes the original game as a reference only — nothing there is a commitment.

- [x] Fighting system design — **decided: original-style melee** (see Combat & content)
- [ ] Ant castes beyond Queen/Worker — Soldier added in wave 1; Nanitic, Major, Acid Ant, Alate still undecided
- [ ] Food types and economy details — Green + Super in wave 1; Meat and Red Food undecided
- [ ] Whether to re-enable the pheromone system the original disabled
- [x] Fog of war — **decided: yes, client-side in wave 1** (radii: 10 surface / 5 underground); day/night and weather parameters still undecided
- [ ] Day/night cycle and weather parameters
- [ ] Modes to support beyond singleplayer sandbox and co-op multiplayer (teams, private games, PvP)

> Notes and decisions go into the "Ideas / Changes / Improvements over the original" section of `docs/BASED-ON.md` and, once confirmed, get promoted into this README.
