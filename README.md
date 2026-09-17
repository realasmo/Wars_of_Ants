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

### Roadmap (confirmed)
0. **Repo foundation** — monorepo (`core/`, `server/`, `client/`, `docs/`), toolchains (cargo + wasm-pack + Vite), CI, initial push.
1. **Core simulation** (Rust, headless) — deterministic fixed-timestep ticks, ECS, tilemap + digging, Queen/Worker economy, A* pathfinding, command API, WASM bindings, sim tests.
2. **First playable** — PixiJS client with placeholder/procedural art, `LocalTransport` → WASM core, HUD, basic audio. *Milestone 1: dig, gather, grow the colony, queen death = game over.*
3. **Combat + ecosystem + AI** — predators, day/night, weather, fog of war; remaining content decisions are made and playtested here.
4. **Server + multiplayer** — `NetworkTransport`, authoritative Axum server, binary snapshots, rooms/lobby, co-op.
5. **Meta + release** — matchmaker, DB persistence, metrics, deployment, real art pass, balance.

Build order: singleplayer first. Art: placeholder/procedural until the gameplay is fun.

## Undecided (vs. the original game)

The following decisions are **not yet made**. `docs/BASED-ON.md` describes the original game as a reference only — nothing there is a commitment.

- [ ] Fighting system design (melee/ranged model, damage reaction, panic, targeting)
- [ ] Ant castes beyond Queen/Worker (Nanitic, Soldier, Major, Acid Ant, Alate) — which to keep, their stats and costs
- [ ] Food types and economy details (Green/Meat/Super/Red — keep as-is or change?)
- [ ] Whether to re-enable the pheromone system the original disabled
- [ ] Fog of war, day/night, weather parameters (radii, cycle lengths)
- [ ] Modes to support beyond singleplayer sandbox and co-op multiplayer (teams, private games, PvP)

> Notes and decisions go into the "Ideas / Changes / Improvements over the original" section of `docs/BASED-ON.md` and, once confirmed, get promoted into this README.
