AntWar.io — description of the original game (design reference)
A browser-based ant colony game: an RTS/MMO hybrid with colony simulation, AI, pathfinding, a physical underground map, and multiplayer.

Technical stack (confirmed by client code)
JavaScript + HTML + CSS
PIXI.js — 2D WebGL renderer (Sprite, Container, sprite sheets, filters)
Socket.IO / WebSocket (websocket-only transport)
Howler.js — audio (ambient above/below ground, rain, thunder, footsteps, gathering)
Not Unity WebGL (some portals state this incorrectly; the client code is the proof)
Singleplayer via fakeSockets (local socket emulation) — pattern: shared game core + LocalTransport/NetworkTransport

Gameplay loop
The player controls a single ant → works for the colony → the colony gathers food → the queen produces ants → the colony grows → predators and enemy colonies → combat → queen death = colony death.
The player = one unit; the rest of the colony has its own AI.
The player can issue commands: follow, stop, defend, attack, gathering, transport, feeding, carrying brood, working in the nest.
Player death ≠ game over — you can take control of another ant.
Multiple players can be ants of a single colony (e.g. A=Worker, B=Soldier, C=Scout).
Modes: multiplayer, teams, lobby, private games, Co-Op (2024), singleplayer, sandbox.

Map: two layers
Surface — food, plants, aphids, insects, spiders, centipedes, other colonies, exploration.
Underground — digging, tunnels, chambers, brood, queen, storage.

Tilemap / digging
Tile types: dirt, dry, moist, rock, empty.
Ants actually modify the terrain — the nest must be dug out.
Dynamics: tiles wear out/degrade; humidity affects brood development; twigs create dry spots, water droplets increase humidity.
Conclusion: food alone is not enough — you need a good nest.

Queen
Her death = death of the entire colony (main win/loss condition).
She eats, produces brood and eggs via an offspring selection menu (since 2021); each offspring type costs specific resources.
Can be controlled by the team leader, moves around the nest; strong inside the nest, very vulnerable outside it.

Ant castes
Caste	Role	Cost
Nanitic	fast, basic, nest	Green Food (early)
Worker	universal	1 Green + 1 Meat
Soldier	combat	1 Super + 2 Meat
Major	heavy unit	2 Super + 2 Meat
Acid Ant	ranged attack	1 Green + 1 Meat + 2 Super
Queen	reproduction	—
Alate	founding a new nest	—

Food
Types: Green Food, Meat, Super Food (Blue), Red Food.
Super Food: sources are predators, aphids, grubs; used for stronger castes.
Progression: early (green → nanitics) → mid (green+meat → workers) → late (super+meat → soldiers/majors/acids).
RTS-style economy: food → queen → eggs → brood → ants → workers (more food) / soldiers (defense) → even more food.

Ant AI
Autonomous roles: gathering food/meat, feeding, transport, working in the nest, defense, combat, reacting to threats, following the player.
A pheromone system existed but was disabled.

Combat (realtime)
Attributes: HP, damage, speed, target, attack, pathfinding.
Worker: 100 HP / 8 dmg / speed 6. Soldier: 130 HP / 22 dmg / speed 5.
Units react to damage, can panic.
Automatic lock-on after clicking an opponent.

PvE / ecosystem
Neutral/hostile: spiders, centipedes, bugs, grubs, aphids, Goliath beetle.
PvE loop: kill a spider → Super Food → Soldier → bigger enemy → more Super Food → Major/Acid (a natural "tech tree" without a tech tree).
Aphids live on plants and generate food = farms; strategic points emerge from the ecosystem, not from "resource nodes".

Day/night, weather, visibility
Cycle: day, night, rain, storm; weather affects visibility (night and rain reduce vision). Historically: 2 min day + 1 min night.
Fog of war: ~1500 px above ground, ~750 px underground. Information about the world is a gameplay element.

Networking (details from client code)
Socket.IO: io("//" + address + ":" + port, { query: { userToken, gameSocketType: "main" }, transports: ["websocket"], reconnection: false }).
Binary snapshot synchronization (ArrayBuffer + DataView): timestamp, entity id, x, y, z, rotation — no full JSON per tick.

Ideas / Changes / Improvements over the original
Phase 3 wave 1 decisions (confirmed):
- Combat: keep original-style melee (HP/dmg/cooldown/speed, click to lock on).
- Content wave 1: spiders (surface predators) drop Super Food; Soldier caste costs 2 Super + 3 Green, produced automatically (cap: 1 soldier per 2 workers).
- Fog of war: in, client-side for singleplayer (10 tiles surface vision / 5 underground); server-authoritative version deferred to multiplayer phase.
(to be filled in — further notes and decisions go here, then get promoted into README.md once confirmed)

Target architecture
Frontend: TypeScript + PixiJS (WebGL) + WebSocket + WebAudio + HTML/CSS.
Game server: Rust, authoritative — ECS, simulation, spatial hash, pathfinding, AI, combat, resource system, colony system, networking.
Additionally: matchmaker, DB, metrics.
Transport: LocalTransport (singleplayer) / NetworkTransport (multiplayer), shared game core.
Positioning: multiplayer colony simulation + RTS + survival + ecosystem. Browser-based 2D WebGL + WebSocket is sufficient (proof: the working AntWar).
