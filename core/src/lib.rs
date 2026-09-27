mod components;
mod math;
mod path;
mod rng;
mod rules;
mod sim;
mod world;

pub use components::{AntState, Carry, Caste, FoodKind, Layer, WorkerAi};
pub use rules::{GameRules, SourceSpec, UnitStats};
pub use sim::{
    Activity, AntSnap, Colony, Command, DevSpawn, EggSnap, EntitySnap, FollowMode, FoodRole,
    FoodSnap, Phase, Sim, SpiderSnap, Team, DT, TPS,
};
pub use world::{DIRT, DRY, EMPTY, MOIST, ROCK, SOIL_NONE, SOIL_ORANGE, SOIL_SILVER};

use wasm_bindgen::prelude::*;

/// The game version, displayed in the main menu and the in-game corner.
/// Scheme: MAJOR.MINOR.WAVE.BUILD-dev — BUILD bumps on EVERY game change
/// (however slight), WAVE bumps per shipped feature wave, -dev is constant
/// while the game is in development. Single source of truth: edit this one
/// line in the same commit as any game change.
pub const GAME_VERSION: &str = "0.1.03.55-dev";

#[wasm_bindgen]
pub fn game_version() -> String {
    GAME_VERSION.to_string()
}

#[wasm_bindgen]
pub fn core_version() -> String {
    format!("woa-core {GAME_VERSION}")
}

/// Wire-code spec for the snapshot transport — the client asserts its
/// decoder against this at boot, before any sim exists.
#[wasm_bindgen]
pub fn snapshot_spec() -> String {
    sim::snapshot_spec()
}

#[wasm_bindgen]
pub struct WoaSim {
    inner: Sim,
}

#[wasm_bindgen]
impl WoaSim {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u64, workers: u32, clusters: u32) -> WoaSim {
        let rules = GameRules {
            start_workers: workers,
            food_clusters: clusters,
            ..GameRules::default()
        };
        WoaSim {
            inner: Sim::new(seed, rules),
        }
    }

    /// Founding start: lone flying queen, no workers, no nest yet.
    /// `team`: 0 = red, 1 = blue.
    pub fn new_founding(seed: u64, team: u32) -> WoaSim {
        let team = if team == 1 { Team::Blue } else { Team::Red };
        WoaSim {
            inner: Sim::new_founding(seed, team),
        }
    }

    pub fn tick(&mut self, n: u32) {
        for _ in 0..n {
            self.inner.tick();
        }
    }

    pub fn tick_count(&self) -> u64 {
        self.inner.tick
    }

    pub fn cmd_move(&mut self, ant: u32, x: f64, y: f64) -> bool {
        self.inner.issue(Command::Move { ant, x, y })
    }

    pub fn cmd_dig(&mut self, ant: u32, tx: u32, ty: u32) -> bool {
        self.inner.issue(Command::Dig { ant, tx, ty })
    }

    pub fn cmd_attack(&mut self, ant: u32, target: u32) -> bool {
        self.inner.issue(Command::Attack { ant, target })
    }

    pub fn cmd_entrance(&mut self, ant: u32) -> bool {
        self.inner.issue(Command::UseEntrance { ant })
    }

    pub fn cmd_land(&mut self, ant: u32, x: f64, y: f64) -> bool {
        self.inner.issue(Command::Land { ant, x, y })
    }

    pub fn cmd_found(&mut self, ant: u32, x: f64, y: f64) -> bool {
        self.inner.issue(Command::FoundNest { ant, x, y })
    }

    pub fn cmd_dump(&mut self, ant: u32, tx: u32, ty: u32) -> bool {
        // legacy name for dropping the carried item (dirt semantics kept)
        self.inner.issue(Command::Drop { ant, tx, ty })
    }

    /// Squad control (X-menu): `mode` 0 all-in-sight, 1 nearest one,
    /// 2 all soldiers, 3 release. The issuing ant leads.
    pub fn cmd_follow(&mut self, leader: u32, mode: u32) -> bool {
        let mode = match mode {
            0 => FollowMode::All,
            1 => FollowMode::One,
            2 => FollowMode::Soldiers,
            _ => FollowMode::Release,
        };
        self.inner.issue(Command::Follow { leader, mode })
    }

    pub fn cmd_drop(&mut self, ant: u32, tx: u32, ty: u32) -> bool {
        self.inner.issue(Command::Drop { ant, tx, ty })
    }

    pub fn cmd_pick_egg(&mut self, ant: u32, egg: u32) -> bool {
        self.inner.issue(Command::PickEgg { ant, egg })
    }

    /// Returns the new entity id, or u32::MAX for an unknown kind.
    pub fn dev_spawn(&mut self, kind: String, x: f64, y: f64) -> u32 {
        match DevSpawn::parse(&kind) {
            Some(what) => self.inner.dev_spawn(what, x, y),
            None => u32::MAX,
        }
    }

    pub fn dev_set_food(&mut self, n: u32) {
        self.inner.dev_set_food(n);
    }

    pub fn dev_set_super(&mut self, n: u32) {
        self.inner.dev_set_super(n);
    }

    pub fn dev_set_water(&mut self, n: u32) {
        self.inner.dev_set_water(n);
    }

    pub fn dev_kill(&mut self, id: u32) -> bool {
        self.inner.dev_kill(id)
    }

    /// Paint a 2×2 soil block: `soil` 0 none, 1 orange, 2 silver.
    pub fn dev_set_soil(&mut self, layer: u32, x: u32, y: u32, soil: u32) {
        self.inner.dev_set_soil(layer, x, y, soil);
    }

    pub fn colony_dead(&self) -> bool {
        self.inner.colony.dead
    }

    pub fn store_carbs(&self) -> u32 {
        self.inner.colony.carbs
    }

    pub fn store_protein(&self) -> u32 {
        self.inner.colony.protein
    }

    pub fn store_water(&self) -> u32 {
        self.inner.colony.water
    }

    pub fn ants_alive(&self) -> u32 {
        self.inner.colony.ant_count
    }

    pub fn tiles_dug(&self) -> u32 {
        self.inner.tiles_dug()
    }

    /// Retained game events (capped), oldest first. Observational only.
    pub fn events(&self) -> Vec<String> {
        self.inner.event_lines().to_vec()
    }

    /// Total events ever emitted (for cheap change polling).
    pub fn event_total(&self) -> u64 {
        self.inner.event_total()
    }

    pub fn canonical(&self) -> String {
        self.inner.canonical_state()
    }

    pub fn dims(&self) -> Vec<u32> {
        vec![self.inner.rules.width, self.inner.rules.height]
    }

    pub fn entrance(&self) -> Vec<u32> {
        // empty until the founding queen creates the nest
        self.inner
            .world
            .entrance
            .map(|(x, y)| vec![x, y])
            .unwrap_or_default()
    }

    /// Colony start phase: 0 flight, 1 grounded, 2 founding, 3 brood, 4 colony.
    pub fn phase(&self) -> u8 {
        self.inner.colony.phase as u8
    }

    /// Seconds left in the founding excavation window (0.0 otherwise).
    pub fn phase_time(&self) -> f64 {
        if self.inner.colony.phase == Phase::Founding {
            self.inner.colony.phase_t.max(0.0)
        } else {
            0.0
        }
    }

    /// Team color: 0 red, 1 blue.
    pub fn team(&self) -> u8 {
        self.inner.colony.team as u8
    }

    /// The designated feeder's id (u32::MAX when there is none) — debug/UX
    /// readout; the gameplay path lives in the snapshot.
    pub fn feeder_id(&self) -> u32 {
        self.inner.colony.feeder_id.unwrap_or(u32::MAX)
    }

    pub fn tiles_underground(&self) -> Vec<u8> {
        self.inner.tiles(Layer::Underground).to_vec()
    }

    pub fn tiles_surface(&self) -> Vec<u8> {
        self.inner.tiles(Layer::Surface).to_vec()
    }

    /// Per-tile soil quality (0 none, 1 orange, 2 silver) per layer.
    pub fn soil_underground(&self) -> Vec<u8> {
        self.inner.world.soil_underground.clone()
    }

    pub fn soil_surface(&self) -> Vec<u8> {
        self.inner.world.soil_surface.clone()
    }

    /// Flat snapshot transport: `[tick, dead, carbs, count]` header, then
    /// stride-12 records `[id, kind, layer, x, y, p0..p6]`. The positional
    /// codes are single-sourced in `sim::snapshot` and exported through
    /// `snapshot_spec()` — the client asserts its decoder against that spec
    /// at boot. Layout version 3 (see spec).
    pub fn snapshot(&self) -> Vec<f64> {
        use sim::snapshot as wire; // the wire-code module (crate-private)
        let snaps = self.inner.snapshot();
        let mut v = Vec::with_capacity(4 + snaps.len() * 12);
        v.push(self.inner.tick as f64);
        v.push(self.inner.colony.dead as u8 as f64);
        v.push(self.inner.colony.carbs as f64);
        v.push(snaps.len() as f64);
        let mut rec = |id: u32, kind: u8, layer: u8, x: f64, y: f64, p: [f64; 7]| {
            v.extend([
                id as f64,
                kind as f64,
                layer as f64,
                x,
                y,
                p[0],
                p[1],
                p[2],
                p[3],
                p[4],
                p[5],
                p[6],
            ]);
        };
        for s in snaps {
            match s {
                EntitySnap::Ant(a) => {
                    let kind = match a.caste {
                        Caste::Queen => wire::KIND_QUEEN,
                        Caste::Worker => wire::KIND_WORKER,
                        Caste::Soldier => wire::KIND_SOLDIER,
                    };
                    let act = match a.activity {
                        Activity::Idle => wire::ACT_IDLE,
                        Activity::Moving => wire::ACT_MOVING,
                        Activity::Digging => wire::ACT_DIGGING,
                        Activity::Fighting => wire::ACT_FIGHTING,
                        Activity::Flying => wire::ACT_FLYING,
                        Activity::Harvesting => wire::ACT_HARVESTING,
                    };
                    let (tag, data) = match a.carry {
                        Carry::None => (wire::CARRY_NONE, 0.0),
                        Carry::Dirt { blocks } => (wire::CARRY_DIRT, blocks as f64),
                        Carry::Egg => (wire::CARRY_EGG, 0.0),
                        Carry::Food(f) => (wire::CARRY_FOOD, wire::food_code(f) as f64),
                        Carry::Wood => (wire::CARRY_WOOD, 0.0),
                        Carry::Wool => (wire::CARRY_WOOL, 0.0),
                    };
                    rec(
                        a.id,
                        kind,
                        a.layer as u8,
                        a.x,
                        a.y,
                        [
                            act as f64,
                            a.hp,
                            tag as f64,
                            data,
                            a.hunger,
                            a.following.map(|l| l as f64 + 1.0).unwrap_or(0.0),
                            // p6: the queen's craved resource (food code);
                            // 0 for everyone else / no craving
                            a.request.map(wire::food_code).unwrap_or(wire::REQUEST_NONE) as f64,
                        ],
                    );
                }
                EntitySnap::Food(f) => match f.role {
                    FoodRole::Source { src } => rec(
                        f.id,
                        wire::KIND_SOURCE,
                        f.layer as u8,
                        f.x,
                        f.y,
                        [
                            f.amount as f64,
                            wire::food_code(f.kind) as f64,
                            src as f64,
                            0.0,
                            0.0,
                            0.0,
                            0.0,
                        ],
                    ),
                    role => {
                        // Loose dropped unit (p2 = remaining spoil fraction,
                        // -1 = not spoiling) or banked pantry pile (p2 = -1).
                        let spoil = match role {
                            FoodRole::Loose { spoil } => spoil.unwrap_or(-1.0),
                            _ => -1.0,
                        };
                        rec(
                            f.id,
                            wire::KIND_FOOD,
                            f.layer as u8,
                            f.x,
                            f.y,
                            [
                                f.amount as f64,
                                wire::food_code(f.kind) as f64,
                                spoil,
                                0.0,
                                0.0,
                                0.0,
                                0.0,
                            ],
                        );
                    }
                },
                EntitySnap::Egg(e) => rec(
                    e.id,
                    wire::KIND_EGG,
                    e.layer as u8,
                    e.x,
                    e.y,
                    [
                        e.hatch_left,
                        if e.caste == Caste::Soldier { 1.0 } else { 0.0 },
                        if e.carried { 1.0 } else { 0.0 },
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                    ],
                ),
                EntitySnap::Spider(p) => rec(
                    p.id,
                    wire::KIND_SPIDER,
                    p.layer as u8,
                    p.x,
                    p.y,
                    [
                        p.hp,
                        if p.hunting { 1.0 } else { 0.0 },
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                    ],
                ),
                EntitySnap::Collectible(c) => rec(
                    c.id,
                    wire::KIND_COLLECTIBLE,
                    c.layer as u8,
                    c.x,
                    c.y,
                    [c.variant as f64, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                ),
            }
        }
        v
    }

    /// Bumped by every tile mutation — re-pull the grids only when it moves.
    pub fn tiles_epoch(&self) -> u64 {
        self.inner.tiles_epoch
    }

    /// Bumped by every soil mutation (grants, dev painting).
    pub fn soil_epoch(&self) -> u64 {
        self.inner.soil_epoch
    }
}
