use std::collections::BTreeMap;

use hecs::Entity;

use crate::components::*;
use crate::math::Vec2;
use crate::path::{chebyshev, find_path, manhattan, tile_of};
use crate::rng::Rng;
use crate::world::{tile_center, Grid, World, DIRT, EMPTY, ROCK};

pub const TPS: u32 = 20;
pub const DT: f64 = 1.0 / TPS as f64;

pub const WORKER_SPEED: f64 = 3.0;
pub const DIG_TIME: f64 = 1.2;
pub const EAT_PERIOD: f64 = 25.0;
pub const EGG_COST: u32 = 5;
pub const EGG_TIME: f64 = 45.0;
pub const LAY_COOLDOWN: f64 = 12.0;
pub const STARVE_TIME: f64 = 90.0;
pub const START_FOOD: u32 = 5;
pub const PILE_AMOUNT: u32 = 45;

#[derive(Clone)]
pub struct Config {
    pub width: u32,
    pub height: u32,
    pub start_workers: u32,
    pub food_clusters: u32,
    pub max_ants: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            width: 96,
            height: 96,
            start_workers: 3,
            food_clusters: 6,
            max_ants: 24,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    Move { ant: u32, x: f64, y: f64 },
    Dig { ant: u32, tx: u32, ty: u32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntitySnap {
    pub id: u32,
    pub kind: u8,
    pub layer: u8,
    pub x: f64,
    pub y: f64,
    pub state: u8,
    pub extra: f64,
}

pub struct Colony {
    pub food: u32,
    pub delivered: u32,
    pub eggs_laid: u32,
    pub dead: bool,
    pub queen_id: u32,
    pub ant_count: u32,
    pub starve_t: f64,
    pub eat_t: f64,
    pub lay_cooldown: f64,
}

pub struct Sim {
    pub world: World,
    pub ecs: hecs::World,
    pub rng: Rng,
    pub colony: Colony,
    pub config: Config,
    pub tick: u64,
    ids: BTreeMap<u32, Entity>,
    next_id: u32,
    dug_tiles: u32,
}

impl Sim {
    pub fn new(seed: u64, config: Config) -> Sim {
        let w = config.width;
        let h = config.height;
        let e = w / 2;
        let mut rng = Rng::new(seed);

        let surface = Grid::filled(w, h, EMPTY);
        let mut underground = Grid::filled(w, h, DIRT);
        for x in 0..w {
            underground.set(x, 0, ROCK);
            underground.set(x, h - 1, ROCK);
        }
        for y in 0..h {
            underground.set(0, y, ROCK);
            underground.set(w - 1, y, ROCK);
        }
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                if y < 8 && (x as i32 - e as i32).abs() <= 4 {
                    continue;
                }
                if rng.f64() < 0.08 {
                    underground.set(x, y, ROCK);
                }
            }
        }
        for y in 1..=4 {
            for x in e.saturating_sub(2)..=e + 2 {
                underground.set(x, y, EMPTY);
            }
        }
        underground.set(e, 0, EMPTY);

        let mut sim = Sim {
            world: World {
                surface,
                underground,
                entrance: (e, 0),
            },
            ecs: hecs::World::new(),
            rng,
            colony: Colony {
                food: START_FOOD,
                delivered: 0,
                eggs_laid: 0,
                dead: false,
                queen_id: 0,
                ant_count: 0,
                starve_t: 0.0,
                eat_t: 0.0,
                lay_cooldown: LAY_COOLDOWN,
            },
            config,
            tick: 0,
            ids: BTreeMap::new(),
            next_id: 0,
            dug_tiles: 0,
        };

        let queen_id = sim.spawn_ant(Caste::Queen, tile_center(e, 2), Layer::Underground);
        sim.colony.queen_id = queen_id;
        for i in 0..sim.config.start_workers {
            let x = e.saturating_sub(1) + (i % 3);
            let y = 3 + (i / 3);
            sim.spawn_ant(Caste::Worker, tile_center(x, y), Layer::Underground);
        }

        for _ in 0..sim.config.food_clusters {
            let cx = (e as i32 + sim.rng.irange(0, 41) as i32 - 20).clamp(2, w as i32 - 3) as u32;
            let cy = sim.rng.irange(15, 41).min(h - 3);
            let piles = sim.rng.irange(4, 8);
            for _ in 0..piles {
                let px =
                    (cx as i32 + sim.rng.irange(0, 5) as i32 - 2).clamp(1, w as i32 - 2) as u32;
                let py =
                    (cy as i32 + sim.rng.irange(0, 5) as i32 - 2).clamp(1, h as i32 - 2) as u32;
                sim.spawn_food(tile_center(px, py), PILE_AMOUNT);
            }
        }
        sim
    }

    pub fn issue(&mut self, cmd: Command) -> bool {
        self.apply_command(cmd)
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        self.worker_ai();
        self.movement();
        self.digging();
        self.queen_system();
        self.eggs();
        self.cleanup();
    }

    fn apply_command(&mut self, cmd: Command) -> bool {
        match cmd {
            Command::Move { ant, x, y } => {
                if !self.is_ant(ant) {
                    return false;
                }
                let tx = x.floor().clamp(0.0, self.config.width as f64 - 1.0) as u32;
                let ty = y.floor().clamp(0.0, self.config.height as f64 - 1.0) as u32;
                let layer = self.ant_layer(ant);
                self.set_job(ant, Job::Manual);
                self.route(ant, layer, (tx, ty))
            }
            Command::Dig { ant, tx, ty } => {
                let ent = match self.ids.get(&ant) {
                    Some(&e) => e,
                    None => return false,
                };
                let (caste, pos) = match self.ecs.query_one::<(&Ant, &Pos)>(ent).unwrap().get() {
                    Some(q) => (q.0.caste, q.1.layer),
                    None => return false,
                };
                if caste != Caste::Worker
                    || pos != Layer::Underground
                    || !self.world.underground.in_bounds(tx, ty)
                {
                    return false;
                }
                let kind = self.world.underground.get(tx, ty);
                if !Grid::is_soft(kind) {
                    return false;
                }
                if chebyshev(self.ant_tile(ant), (tx, ty)) > 1 {
                    return false;
                }
                self.set_job(ant, Job::Manual);
                self.set_state(
                    ant,
                    AntState::Digging {
                        tx,
                        ty,
                        progress: 0.0,
                    },
                );
                true
            }
        }
    }

    fn set_state(&mut self, id: u32, state: AntState) {
        if let Some(&ent) = self.ids.get(&id) {
            if let Ok(mut q) = self.ecs.get::<&mut AntState>(ent) {
                *q = state;
            }
        }
    }

    fn set_job(&mut self, id: u32, job: Job) {
        if let Some(&ent) = self.ids.get(&id) {
            if let Ok(mut q) = self.ecs.get::<&mut WorkerAi>(ent) {
                q.job = job;
            }
        }
    }

    fn set_pending(&mut self, id: u32, pending: Option<(Layer, (u32, u32))>) {
        if let Some(&ent) = self.ids.get(&id) {
            if let Ok(mut q) = self.ecs.get::<&mut WorkerAi>(ent) {
                q.pending = pending;
            }
        }
    }

    fn set_retry(&mut self, id: u32, retry: u32) {
        if let Some(&ent) = self.ids.get(&id) {
            if let Ok(mut q) = self.ecs.get::<&mut WorkerAi>(ent) {
                q.retry = retry;
            }
        }
    }

    fn route(&mut self, id: u32, dest_layer: Layer, dest: (u32, u32)) -> bool {
        let ent = match self.ids.get(&id) {
            Some(&e) => e,
            None => return false,
        };
        let (is_worker, start_layer, start_tile) =
            match self.ecs.query_one::<(&Ant, &Pos)>(ent).unwrap().get() {
                Some(q) => (q.0.caste == Caste::Worker, q.1.layer, tile_of(q.1.p)),
                None => return false,
            };
        if start_layer == dest_layer {
            let grid = self.grid_of(start_layer);
            match find_path(grid, start_tile, dest, is_worker) {
                Some(path) => {
                    self.set_state(
                        id,
                        AntState::Moving {
                            path,
                            next: 0,
                            then_swap: false,
                        },
                    );
                    self.set_pending(id, None);
                    true
                }
                None => false,
            }
        } else {
            let entrance = self.world.entrance;
            let grid = self.grid_of(start_layer);
            match find_path(grid, start_tile, entrance, is_worker) {
                Some(path) => {
                    self.set_state(
                        id,
                        AntState::Moving {
                            path,
                            next: 0,
                            then_swap: true,
                        },
                    );
                    self.set_pending(id, Some((dest_layer, dest)));
                    true
                }
                None => false,
            }
        }
    }

    fn grid_of(&self, layer: Layer) -> &Grid {
        match layer {
            Layer::Surface => &self.world.surface,
            Layer::Underground => &self.world.underground,
        }
    }

    fn is_ant(&self, id: u32) -> bool {
        self.ids
            .get(&id)
            .map(|&e| self.ecs.get::<&Ant>(e).is_ok())
            .unwrap_or(false)
    }

    fn ant_layer(&self, id: u32) -> Layer {
        let ent = self.ids[&id];
        self.ecs
            .get::<&Pos>(ent)
            .map(|q| q.layer)
            .unwrap_or(Layer::Underground)
    }

    fn ant_tile(&self, id: u32) -> (u32, u32) {
        let ent = self.ids[&id];
        self.ecs
            .get::<&Pos>(ent)
            .map(|q| tile_of(q.p))
            .unwrap_or((0, 0))
    }

    fn ant_ids(&self) -> Vec<u32> {
        self.ids
            .iter()
            .filter_map(|(&id, &ent)| self.ecs.get::<&Ant>(ent).is_ok().then_some(id))
            .collect()
    }

    fn food_ids(&self) -> Vec<u32> {
        self.ids
            .iter()
            .filter_map(|(&id, &ent)| self.ecs.get::<&Food>(ent).is_ok().then_some(id))
            .collect()
    }

    fn egg_ids(&self) -> Vec<u32> {
        self.ids
            .iter()
            .filter_map(|(&id, &ent)| self.ecs.get::<&Egg>(ent).is_ok().then_some(id))
            .collect()
    }

    fn fresh_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn spawn_ant(&mut self, caste: Caste, p: Vec2, layer: Layer) -> u32 {
        let id = self.fresh_id();
        let speed = if caste == Caste::Queen {
            1.0
        } else {
            WORKER_SPEED
        };
        let ent = self.ecs.spawn((
            Ant { caste, speed },
            Pos { p, layer },
            AntState::Idle,
            Carrying { food: 0 },
            WorkerAi {
                job: Job::Idle,
                retry: 0,
                pending: None,
            },
        ));
        self.ids.insert(id, ent);
        self.colony.ant_count += 1;
        id
    }

    fn spawn_food(&mut self, p: Vec2, amount: u32) -> u32 {
        let id = self.fresh_id();
        let ent = self.ecs.spawn((
            Food { amount },
            Pos {
                p,
                layer: Layer::Surface,
            },
        ));
        self.ids.insert(id, ent);
        id
    }

    fn spawn_egg(&mut self, p: Vec2) -> u32 {
        let id = self.fresh_id();
        let ent = self.ecs.spawn((
            Egg {
                hatch: EGG_TIME,
                total: EGG_TIME,
            },
            Pos {
                p,
                layer: Layer::Underground,
            },
        ));
        self.ids.insert(id, ent);
        id
    }

    fn kill(&mut self, id: u32) {
        if let Some(ent) = self.ids.remove(&id) {
            let was_ant = self.ecs.get::<&Ant>(ent).is_ok();
            let _ = self.ecs.despawn(ent);
            if was_ant {
                self.colony.ant_count -= 1;
            }
        }
    }

    fn best_food(&self) -> Option<u32> {
        let entrance = self.world.entrance;
        let mut best: Option<(u32, u32)> = None;
        for &fid in &self.food_ids() {
            let ent = self.ids[&fid];
            let tile = match self.ecs.get::<&Pos>(ent) {
                Ok(q) => tile_of(q.p),
                Err(_) => continue,
            };
            let key = (manhattan(tile, entrance), fid);
            if best.map(|b| key < b).unwrap_or(true) {
                best = Some(key);
            }
        }
        best.map(|(_, fid)| fid)
    }

    fn tile_occupied(&self, tile: (u32, u32), layer: Layer) -> bool {
        for (_e, (_egg, pos)) in self.ecs.query::<(&Egg, &Pos)>().iter() {
            if pos.layer == layer && tile_of(pos.p) == tile {
                return true;
            }
        }
        false
    }

    fn food_info(&self, fid: u32) -> Option<((u32, u32), u32)> {
        let ent = *self.ids.get(&fid)?;
        let mut qo = self.ecs.query_one::<(&Food, &Pos)>(ent).ok()?;
        let q = qo.get()?;
        Some((tile_of(q.1.p), q.0.amount))
    }

    fn queen_tile(&self) -> (u32, u32) {
        self.ant_tile(self.colony.queen_id)
    }

    fn worker_ai(&mut self) {
        if self.colony.dead {
            return;
        }
        let ids = self.ant_ids();
        for id in ids {
            let ent = match self.ids.get(&id) {
                Some(&e) => e,
                None => continue,
            };
            let (pos, state, carrying, job, pending, retry) = {
                let mut qo = match self
                    .ecs
                    .query_one::<(&Pos, &AntState, &Carrying, &WorkerAi)>(ent)
                {
                    Ok(q) => q,
                    Err(_) => continue,
                };
                let q = match qo.get() {
                    Some(q) => q,
                    None => continue,
                };
                (
                    *q.0,
                    (*q.1).clone(),
                    q.2.food,
                    q.3.job,
                    q.3.pending,
                    q.3.retry,
                )
            };
            if !matches!(state, AntState::Idle) {
                continue;
            }
            if retry > 0 {
                self.set_retry(id, retry - 1);
                continue;
            }
            if let Some((layer, tile)) = pending {
                self.set_pending(id, None);
                if !self.route(id, layer, tile) {
                    self.set_retry(id, 60);
                }
                continue;
            }
            match job {
                Job::Manual => {}
                Job::Idle => {
                    if carrying > 0 {
                        self.set_job(id, Job::Deliver);
                    } else if let Some(fid) = self.best_food() {
                        self.set_job(id, Job::Fetch(fid));
                    } else {
                        self.set_retry(id, 50);
                    }
                }
                Job::Fetch(fid) => {
                    let fent = match self.ids.get(&fid) {
                        Some(&e) => e,
                        None => {
                            self.set_job(id, Job::Idle);
                            continue;
                        }
                    };
                    let Some((ftile, amount)) = self.food_info(fid) else {
                        self.set_job(id, Job::Idle);
                        continue;
                    };
                    if pos.layer == Layer::Surface {
                        if tile_of(pos.p) == ftile {
                            if amount > 0 {
                                if let Ok(mut q) = self.ecs.get::<&mut Food>(fent) {
                                    q.amount -= 1;
                                }
                                if let Some(&aent) = self.ids.get(&id) {
                                    if let Ok(mut q) = self.ecs.get::<&mut Carrying>(aent) {
                                        q.food = 1;
                                    }
                                }
                                self.set_job(id, Job::Deliver);
                            } else {
                                self.set_job(id, Job::Idle);
                            }
                        } else if !self.route(id, Layer::Surface, ftile) {
                            self.set_retry(id, 60);
                            self.set_job(id, Job::Idle);
                        }
                    } else if !self.route(id, Layer::Surface, ftile) {
                        self.set_retry(id, 60);
                    }
                }
                Job::Deliver => {
                    if carrying == 0 {
                        self.set_job(id, Job::Idle);
                        continue;
                    }
                    let qtile = self.queen_tile();
                    if pos.layer == Layer::Underground {
                        if chebyshev(tile_of(pos.p), qtile) <= 1 {
                            self.colony.food += 1;
                            self.colony.delivered += 1;
                            if let Some(&aent) = self.ids.get(&id) {
                                if let Ok(mut q) = self.ecs.get::<&mut Carrying>(aent) {
                                    q.food = 0;
                                }
                            }
                            self.set_job(id, Job::Idle);
                        } else if !self.route(id, Layer::Underground, qtile) {
                            self.set_retry(id, 60);
                        }
                    } else if !self.route(id, Layer::Underground, qtile) {
                        self.set_retry(id, 60);
                    }
                }
            }
        }
    }

    fn movement(&mut self) {
        let ids = self.ant_ids();
        for id in ids {
            let ent = match self.ids.get(&id) {
                Some(&e) => e,
                None => continue,
            };
            let (speed, caste, mut pos, state) = match self
                .ecs
                .query_one::<(&Ant, &Pos, &AntState)>(ent)
                .unwrap()
                .get()
            {
                Some(q) => (q.0.speed, q.0.caste, *q.1, (*q.2).clone()),
                None => continue,
            };
            if caste == Caste::Queen {
                continue;
            }
            let (path, mut next, then_swap) = match state {
                AntState::Moving {
                    path,
                    next,
                    then_swap,
                } => (path, next, then_swap),
                _ => continue,
            };
            let mut p = pos.p;
            let mut budget = speed * DT;
            let mut dig_target: Option<(u32, u32)> = None;
            let mut finished = false;
            while budget > 0.0 {
                if next >= path.len() {
                    finished = true;
                    break;
                }
                let (tx, ty) = path[next];
                if pos.layer == Layer::Underground {
                    let kind = self.world.underground.get(tx, ty);
                    if Grid::is_soft(kind) {
                        dig_target = Some((tx, ty));
                        break;
                    }
                }
                let target = tile_center(tx, ty);
                let d = target - p;
                let dist = d.len();
                if dist <= budget {
                    p = target;
                    next += 1;
                    budget -= dist;
                } else {
                    p = p + d * (budget / dist);
                    budget = 0.0;
                }
            }
            if next >= path.len() && dig_target.is_none() {
                finished = true;
            }
            let new_state = if let Some((tx, ty)) = dig_target {
                AntState::Digging {
                    tx,
                    ty,
                    progress: 0.0,
                }
            } else if finished {
                if then_swap {
                    pos.layer = pos.layer.other();
                    p = tile_center(self.world.entrance.0, self.world.entrance.1);
                }
                AntState::Idle
            } else {
                AntState::Moving {
                    path,
                    next,
                    then_swap,
                }
            };
            if let Some(q) = self
                .ecs
                .query_one::<(&mut Pos, &mut AntState)>(ent)
                .unwrap()
                .get()
            {
                q.0.p = p;
                q.0.layer = pos.layer;
                *q.1 = new_state;
            }
        }
    }

    fn digging(&mut self) {
        let ids = self.ant_ids();
        for id in ids {
            let ent = match self.ids.get(&id) {
                Some(&e) => e,
                None => continue,
            };
            let state = match self.ecs.get::<&AntState>(ent) {
                Ok(q) => (*q).clone(),
                Err(_) => continue,
            };
            let (tx, ty, mut progress) = match state {
                AntState::Digging { tx, ty, progress } => (tx, ty, progress),
                _ => continue,
            };
            let kind = self.world.underground.get(tx, ty);
            if !Grid::is_soft(kind) {
                self.set_state(id, AntState::Idle);
                continue;
            }
            progress += DT;
            if progress >= DIG_TIME {
                self.world.underground.set(tx, ty, EMPTY);
                self.dug_tiles += 1;
                self.set_state(id, AntState::Idle);
            } else {
                self.set_state(id, AntState::Digging { tx, ty, progress });
            }
        }
    }

    fn queen_system(&mut self) {
        if self.colony.dead {
            return;
        }
        self.colony.lay_cooldown -= DT;
        if self.colony.food > 0 {
            self.colony.eat_t += DT;
            if self.colony.eat_t >= EAT_PERIOD {
                self.colony.food -= 1;
                self.colony.eat_t = 0.0;
            }
            self.colony.starve_t = 0.0;
        } else {
            self.colony.eat_t = 0.0;
            self.colony.starve_t += DT;
            if self.colony.starve_t >= STARVE_TIME {
                self.colony.dead = true;
                let q = self.colony.queen_id;
                self.kill(q);
                return;
            }
        }
        if self.colony.lay_cooldown <= 0.0
            && self.colony.food >= EGG_COST
            && self.colony.ant_count < self.config.max_ants
        {
            if let Some(tile) = self.free_egg_tile() {
                self.colony.food -= EGG_COST;
                self.colony.eggs_laid += 1;
                self.colony.lay_cooldown = LAY_COOLDOWN;
                self.spawn_egg(tile_center(tile.0, tile.1));
            }
        }
    }

    fn free_egg_tile(&self) -> Option<(u32, u32)> {
        let (qx, qy) = self.queen_tile();
        for r in 0u32..=3 {
            for dy in -(r as i32)..=(r as i32) {
                for dx in -(r as i32)..=(r as i32) {
                    if dx.abs() != r as i32 && dy.abs() != r as i32 {
                        continue;
                    }
                    let x = qx as i32 + dx;
                    let y = qy as i32 + dy;
                    if x < 1
                        || y < 1
                        || x >= self.config.width as i32 - 1
                        || y >= self.config.height as i32 - 1
                    {
                        continue;
                    }
                    let (x, y) = (x as u32, y as u32);
                    if self.world.underground.get(x, y) == EMPTY
                        && !self.tile_occupied((x, y), Layer::Underground)
                    {
                        return Some((x, y));
                    }
                }
            }
        }
        None
    }

    fn eggs(&mut self) {
        let ids = self.egg_ids();
        for id in ids {
            let ent = match self.ids.get(&id) {
                Some(&e) => e,
                None => continue,
            };
            let (hatch, pos) = match self.ecs.query_one::<(&Egg, &Pos)>(ent).unwrap().get() {
                Some(q) => (q.0.hatch, *q.1),
                None => continue,
            };
            let hatch = hatch - DT;
            if hatch <= 0.0 {
                let p = pos.p;
                self.kill(id);
                self.spawn_ant(Caste::Worker, p, Layer::Underground);
            } else if let Ok(mut q) = self.ecs.get::<&mut Egg>(ent) {
                q.hatch = hatch;
            }
        }
    }

    fn cleanup(&mut self) {
        let ids = self.food_ids();
        for id in ids {
            let ent = self.ids[&id];
            let amount = self.ecs.get::<&Food>(ent).map(|q| q.amount).unwrap_or(0);
            if amount == 0 {
                self.kill(id);
            }
        }
    }

    pub fn tiles(&self, layer: Layer) -> &[u8] {
        match layer {
            Layer::Surface => &self.world.surface.tiles,
            Layer::Underground => &self.world.underground.tiles,
        }
    }

    pub fn tile_at(&self, layer: Layer, x: u32, y: u32) -> u8 {
        self.grid_of(layer).get(x, y)
    }

    pub fn tiles_dug(&self) -> u32 {
        self.dug_tiles
    }

    pub fn snapshot(&self) -> Vec<EntitySnap> {
        let mut rev = std::collections::HashMap::new();
        for (&id, &ent) in &self.ids {
            rev.insert(ent, id);
        }
        let mut v = Vec::new();
        for (ent, (ant, pos, state, carry)) in self
            .ecs
            .query::<(&Ant, &Pos, &AntState, &Carrying)>()
            .iter()
        {
            let Some(&id) = rev.get(&ent) else { continue };
            let kind = if ant.caste == Caste::Queen { 0 } else { 1 };
            let state = match state {
                AntState::Idle => 0u8,
                AntState::Moving { .. } => 1,
                AntState::Digging { .. } => 2,
            };
            let extra = if ant.caste == Caste::Queen {
                self.colony.starve_t / STARVE_TIME
            } else {
                carry.food as f64
            };
            v.push(EntitySnap {
                id,
                kind,
                layer: pos.layer as u8,
                x: pos.p.x,
                y: pos.p.y,
                state,
                extra,
            });
        }
        for (ent, (food, pos)) in self.ecs.query::<(&Food, &Pos)>().iter() {
            let Some(&id) = rev.get(&ent) else { continue };
            v.push(EntitySnap {
                id,
                kind: 2,
                layer: pos.layer as u8,
                x: pos.p.x,
                y: pos.p.y,
                state: 0,
                extra: food.amount as f64,
            });
        }
        for (ent, (egg, pos)) in self.ecs.query::<(&Egg, &Pos)>().iter() {
            let Some(&id) = rev.get(&ent) else { continue };
            v.push(EntitySnap {
                id,
                kind: 3,
                layer: pos.layer as u8,
                x: pos.p.x,
                y: pos.p.y,
                state: 0,
                extra: egg.hatch / egg.total,
            });
        }
        v.sort_by_key(|s| s.id);
        v
    }
}
