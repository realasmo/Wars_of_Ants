mod balance;
mod components;
mod math;
mod path;
mod rng;
mod sim;
mod world;

pub use balance::{SourceSpec, UnitStats, SOURCES};
pub use components::{AntState, Caste, Layer};
pub use sim::{Colony, Command, Config, DevSpawn, EntitySnap, Phase, Sim, Team, DT, TPS};
pub use world::{DIRT, DRY, EMPTY, MOIST, ROCK};

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn core_version() -> String {
    format!("woa-core {}", env!("CARGO_PKG_VERSION"))
}

#[wasm_bindgen]
pub struct WoaSim {
    inner: Sim,
}

#[wasm_bindgen]
impl WoaSim {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u64, workers: u32, clusters: u32) -> WoaSim {
        let config = Config {
            start_workers: workers,
            food_clusters: clusters,
            ..Config::default()
        };
        WoaSim {
            inner: Sim::new(seed, config),
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

    pub fn canonical(&self) -> String {
        self.inner.canonical_state()
    }

    pub fn dims(&self) -> Vec<u32> {
        vec![self.inner.config.width, self.inner.config.height]
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

    pub fn snapshot(&self) -> Vec<f64> {
        let snaps = self.inner.snapshot();
        let mut v = Vec::with_capacity(4 + snaps.len() * 9);
        v.push(self.inner.tick as f64);
        v.push(self.inner.colony.dead as u8 as f64);
        v.push(self.inner.colony.carbs as f64);
        v.push(snaps.len() as f64);
        for s in snaps {
            v.extend([
                s.id as f64,
                s.kind as f64,
                s.layer as f64,
                s.x,
                s.y,
                s.state as f64,
                s.extra,
                s.hp,
                s.aux,
            ]);
        }
        v
    }
}
