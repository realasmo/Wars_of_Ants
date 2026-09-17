mod components;
mod math;
mod path;
mod rng;
mod sim;
mod world;

pub use components::{AntState, Caste, Layer};
pub use sim::{Colony, Command, Config, EntitySnap, Sim, DT, TPS};
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

    pub fn colony_dead(&self) -> bool {
        self.inner.colony.dead
    }

    pub fn food_store(&self) -> u32 {
        self.inner.colony.food
    }

    pub fn food_super(&self) -> u32 {
        self.inner.colony.food_super
    }

    pub fn ants_alive(&self) -> u32 {
        self.inner.colony.ant_count
    }

    pub fn tiles_dug(&self) -> u32 {
        self.inner.tiles_dug()
    }

    pub fn dims(&self) -> Vec<u32> {
        vec![self.inner.config.width, self.inner.config.height]
    }

    pub fn entrance(&self) -> Vec<u32> {
        let (x, y) = self.inner.world.entrance;
        vec![x, y]
    }

    pub fn tiles_underground(&self) -> Vec<u8> {
        self.inner.tiles(Layer::Underground).to_vec()
    }

    pub fn tiles_surface(&self) -> Vec<u8> {
        self.inner.tiles(Layer::Surface).to_vec()
    }

    pub fn snapshot(&self) -> Vec<f64> {
        let snaps = self.inner.snapshot();
        let mut v = Vec::with_capacity(4 + snaps.len() * 9);
        v.push(self.inner.tick as f64);
        v.push(self.inner.colony.dead as u8 as f64);
        v.push(self.inner.colony.food as f64);
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
