use std::collections::BTreeMap;

use hecs::Entity;

use crate::balance::*;
use crate::components::*;
use crate::math::Vec2;
use crate::path::{chebyshev, find_path, manhattan, smooth_path, tile_of};
use crate::rng::Rng;
use crate::world::{tile_center, Grid, World, DIRT, EMPTY, ROCK};

pub const TPS: u32 = 20;
pub const DT: f64 = 1.0 / TPS as f64;

#[derive(Clone)]
pub struct Config {
    pub width: u32,
    pub height: u32,
    pub start_workers: u32,
    pub food_clusters: u32,
    pub max_ants: u32,
    pub spiders: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            width: 96,
            height: 96,
            start_workers: 3,
            food_clusters: 6,
            max_ants: 24,
            spiders: 2,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    Move { ant: u32, x: f64, y: f64 },
    Dig { ant: u32, tx: u32, ty: u32 },
    Attack { ant: u32, target: u32 },
    UseEntrance { ant: u32 },
}

/// Spawnable entities for dev tools. Natural layers: spiders/food on the
/// surface, ants/eggs underground (matching real game behavior).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevSpawn {
    Worker,
    Soldier,
    EggWorker,
    EggSoldier,
    Spider,
    Food,
    SuperFood,
}

impl DevSpawn {
    pub fn parse(s: &str) -> Option<DevSpawn> {
        Some(match s {
            "worker" => DevSpawn::Worker,
            "soldier" => DevSpawn::Soldier,
            "egg" => DevSpawn::EggWorker,
            "egg-soldier" => DevSpawn::EggSoldier,
            "spider" => DevSpawn::Spider,
            "food" => DevSpawn::Food,
            "super" => DevSpawn::SuperFood,
            _ => return None,
        })
    }
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
    pub hp: f64,
    pub aux: f64,
}

pub struct Colony {
    pub food: u32,
    pub food_super: u32,
    pub delivered: u32,
    pub eggs_laid: u32,
    pub dead: bool,
    pub queen_id: u32,
    pub ant_count: u32,
    pub starve_t: f64,
    pub eat_t: f64,
    pub lay_cooldown: f64,
    pub dig_queue: Vec<(u32, u32)>,
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
        for y in 1..=3 {
            for x in e.saturating_sub(1)..=e + 1 {
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
                food_super: 0,
                delivered: 0,
                eggs_laid: 0,
                dead: false,
                queen_id: 0,
                ant_count: 0,
                starve_t: 0.0,
                eat_t: 0.0,
                lay_cooldown: LAY_COOLDOWN,
                dig_queue: Vec::new(),
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
                sim.spawn_food(tile_center(px, py), PILE_AMOUNT, FoodKind::Green);
            }
        }

        for _ in 0..sim.config.spiders {
            let dx = sim.rng.range(-1.0, 1.0);
            let dy = sim.rng.range(-1.0, 1.0);
            let len = (dx * dx + dy * dy).sqrt().max(0.001);
            let dist = sim.rng.range(25.0, 45.0);
            let sx = ((e as f64 + dx / len * dist) as i32).clamp(2, w as i32 - 3) as f64 + 0.5;
            let sy = ((dy / len * dist) as i32).clamp(2, h as i32 - 3) as f64 + 0.5;
            sim.spawn_spider(Vec2::new(sx, sy));
        }
        sim
    }

    pub fn issue(&mut self, cmd: Command) -> bool {
        self.apply_command(cmd)
    }

    /// Dev/test operations. They mutate the sim directly (no command gating)
    /// and are recorded with ticks by the client, so a dev action stream is
    /// as replayable as any other command stream.
    pub fn dev_spawn(&mut self, what: DevSpawn, x: f64, y: f64) -> u32 {
        let p = Vec2::new(x, y);
        match what {
            DevSpawn::Worker => self.spawn_ant(Caste::Worker, p, Layer::Underground),
            DevSpawn::Soldier => self.spawn_ant(Caste::Soldier, p, Layer::Underground),
            DevSpawn::EggWorker => self.spawn_egg(p, Caste::Worker),
            DevSpawn::EggSoldier => self.spawn_egg(p, Caste::Soldier),
            DevSpawn::Spider => self.spawn_spider(p),
            DevSpawn::Food => self.spawn_food(p, PILE_AMOUNT, FoodKind::Green),
            DevSpawn::SuperFood => self.spawn_food(p, SUPER_PER_SPIDER, FoodKind::Super),
        }
    }

    pub fn dev_set_food(&mut self, n: u32) {
        self.colony.food = n;
    }

    pub fn dev_set_super(&mut self, n: u32) {
        self.colony.food_super = n;
    }

    pub fn dev_kill(&mut self, id: u32) -> bool {
        if !self.ids.contains_key(&id) {
            return false;
        }
        if id == self.colony.queen_id {
            // the colony system dereferences queen_id every tick — killing
            // the queen outside starvation must flag the colony dead too
            self.colony.dead = true;
        }
        self.kill(id);
        true
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        self.worker_ai();
        self.movement();
        self.digging();
        self.combat();
        self.predators();
        self.cleanup_deaths();
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
                        resume: None,
                    },
                );
                true
            }
            Command::Attack { ant, target } => {
                if !self.is_ant(ant) {
                    return false;
                }
                let Some(&tent) = self.ids.get(&target) else {
                    return false;
                };
                let attackable =
                    self.ecs.get::<&Predator>(tent).is_ok() || self.ecs.get::<&Ant>(tent).is_ok();
                if !attackable {
                    return false;
                }
                self.set_job(ant, Job::Manual);
                self.set_attack_after(ant, Some(target));
                true
            }
            Command::UseEntrance { ant } => {
                if !self.is_ant(ant) {
                    return false;
                }
                let layer = self.ant_layer(ant);
                self.set_job(ant, Job::Manual);
                self.route(ant, layer.other(), self.world.entrance)
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

    fn set_attack_after(&mut self, id: u32, target: Option<u32>) {
        if let Some(&ent) = self.ids.get(&id) {
            if let Ok(mut q) = self.ecs.get::<&mut WorkerAi>(ent) {
                q.attack_after = target;
            }
        }
    }

    fn route(&mut self, id: u32, dest_layer: Layer, dest: (u32, u32)) -> bool {
        let ent = match self.ids.get(&id) {
            Some(&e) => e,
            None => return false,
        };
        let (is_worker, start_layer, start_p) =
            match self.ecs.query_one::<(&Ant, &Pos)>(ent).unwrap().get() {
                Some(q) => (q.0.caste == Caste::Worker, q.1.layer, q.1.p),
                None => return false,
            };
        let start_tile = tile_of(start_p);
        if start_layer == dest_layer {
            let grid = self.grid_of(start_layer);
            match find_path(grid, start_tile, dest, is_worker) {
                Some(path) => {
                    let smooth = smooth_path(grid, start_p, &path);
                    self.set_state(
                        id,
                        AntState::Moving {
                            path: smooth,
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
                    let smooth = smooth_path(grid, start_p, &path);
                    self.set_state(
                        id,
                        AntState::Moving {
                            path: smooth,
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
        let st = stats_for(caste);
        let ent = self.ecs.spawn((
            Ant {
                caste,
                speed: st.speed,
            },
            Pos { p, layer },
            AntState::Idle,
            Combat {
                hp: st.hp,
                max_hp: st.hp,
                dmg: st.dmg,
                atk_cd: st.atk_cd,
                atk_t: 0.0,
            },
            Carrying {
                amount: 0,
                kind: FoodKind::Green,
            },
            WorkerAi {
                job: if caste == Caste::Soldier {
                    Job::Manual
                } else {
                    Job::Idle
                },
                retry: 0,
                pending: None,
                attack_after: None,
            },
        ));
        self.ids.insert(id, ent);
        self.colony.ant_count += 1;
        id
    }

    fn spawn_food(&mut self, p: Vec2, amount: u32, kind: FoodKind) -> u32 {
        let id = self.fresh_id();
        let ent = self.ecs.spawn((
            Food { amount, kind },
            Pos {
                p,
                layer: Layer::Surface,
            },
        ));
        self.ids.insert(id, ent);
        id
    }

    fn spawn_egg(&mut self, p: Vec2, caste: Caste) -> u32 {
        let id = self.fresh_id();
        let ent = self.ecs.spawn((
            Egg {
                hatch: EGG_TIME,
                total: EGG_TIME,
                caste,
            },
            Pos {
                p,
                layer: Layer::Underground,
            },
        ));
        self.ids.insert(id, ent);
        id
    }

    fn spawn_spider(&mut self, p: Vec2) -> u32 {
        let id = self.fresh_id();
        let home = (p.x.floor() as u32, p.y.floor() as u32);
        let ent = self.ecs.spawn((
            Predator {
                hp: SPIDER.hp,
                max_hp: SPIDER.hp,
                dmg: SPIDER.dmg,
                speed: SPIDER.speed,
                atk_cd: SPIDER.atk_cd,
                atk_t: 0.0,
                home,
                wander_t: 0.0,
                dest: None,
                target: None,
            },
            Pos {
                p,
                layer: Layer::Surface,
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
        let mut best: Option<(u32, u32, u32)> = None;
        for &fid in &self.food_ids() {
            let ent = self.ids[&fid];
            let (Ok(fp), Ok(ff)) = (self.ecs.get::<&Pos>(ent), self.ecs.get::<&Food>(ent)) else {
                continue;
            };
            let tile = tile_of(fp.p);
            let super_first = if ff.kind == FoodKind::Super { 0 } else { 1 };
            let key = (super_first, manhattan(tile, entrance), fid);
            if best.map(|b| key < b).unwrap_or(true) {
                best = Some(key);
            }
        }
        best.map(|(_, _, fid)| fid)
    }

    fn tile_occupied(&self, tile: (u32, u32), layer: Layer) -> bool {
        for (_e, (_egg, pos)) in self.ecs.query::<(&Egg, &Pos)>().iter() {
            if pos.layer == layer && tile_of(pos.p) == tile {
                return true;
            }
        }
        false
    }

    fn food_on_tile(&self, tile: (u32, u32)) -> Option<u32> {
        for &fid in &self.food_ids() {
            let ent = self.ids[&fid];
            let Ok(fp) = self.ecs.get::<&Pos>(ent) else {
                continue;
            };
            if tile_of(fp.p) == tile {
                return Some(fid);
            }
        }
        None
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
            let (pos, state, carrying, job, pending, retry, attack_after) = {
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
                    *q.2,
                    q.3.job,
                    q.3.pending,
                    q.3.retry,
                    q.3.attack_after,
                )
            };
            if !matches!(state, AntState::Idle) {
                continue;
            }
            if retry > 0 {
                self.set_retry(id, retry - 1);
                continue;
            }
            if let Some(tid) = attack_after {
                let tinfo = self.ids.get(&tid).and_then(|&tent| {
                    self.ecs
                        .get::<&Pos>(tent)
                        .ok()
                        .map(|q| (q.layer, tile_of(q.p)))
                });
                match tinfo {
                    Some((Layer::Surface, _ttile)) if pos.layer == Layer::Surface => {
                        self.set_pending(id, None);
                        self.set_attack_after(id, None);
                        self.set_state(id, AntState::Fighting { target: tid });
                    }
                    Some((tlayer, ttile)) => {
                        if !self.route(id, tlayer, ttile) {
                            self.set_attack_after(id, None);
                            self.set_retry(id, 60);
                        }
                    }
                    None => {
                        self.set_attack_after(id, None);
                    }
                }
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
                Job::Manual => {
                    if pos.layer == Layer::Surface && carrying.amount == 0 {
                        if let Some(fid) = self.food_on_tile(tile_of(pos.p)) {
                            let mut picked: Option<FoodKind> = None;
                            if let Some(&fent) = self.ids.get(&fid) {
                                if let Ok(mut q) = self.ecs.get::<&mut Food>(fent) {
                                    if q.amount > 0 {
                                        q.amount -= 1;
                                        picked = Some(q.kind);
                                    }
                                }
                            }
                            if let Some(kind) = picked {
                                if let Some(&aent) = self.ids.get(&id) {
                                    if let Ok(mut q) = self.ecs.get::<&mut Carrying>(aent) {
                                        q.amount = 1;
                                        q.kind = kind;
                                    }
                                }
                            }
                        }
                    } else if pos.layer == Layer::Underground && carrying.amount > 0 {
                        let qtile = self.queen_tile();
                        if chebyshev(tile_of(pos.p), qtile) <= 1 {
                            match carrying.kind {
                                FoodKind::Green => self.colony.food += 1,
                                FoodKind::Super => self.colony.food_super += 1,
                            }
                            self.colony.delivered += 1;
                            if let Some(&aent) = self.ids.get(&id) {
                                if let Ok(mut q) = self.ecs.get::<&mut Carrying>(aent) {
                                    q.amount = 0;
                                }
                            }
                        }
                    }
                }
                Job::Idle => {
                    if carrying.amount > 0 {
                        self.set_job(id, Job::Deliver);
                    } else if let Some(t) = self.colony.dig_queue.pop() {
                        self.set_job(id, Job::DigTile(t.0, t.1));
                    } else if let Some(fid) = self.best_food() {
                        self.set_job(id, Job::Fetch(fid));
                    } else {
                        self.set_retry(id, 50);
                    }
                }
                Job::DigTile(tx, ty) => {
                    let kind = self.world.underground.get(tx, ty);
                    if !Grid::is_soft(kind) {
                        self.set_job(id, Job::Idle);
                    } else if pos.layer == Layer::Underground
                        && chebyshev(tile_of(pos.p), (tx, ty)) <= 1
                    {
                        self.set_state(
                            id,
                            AntState::Digging {
                                tx,
                                ty,
                                progress: 0.0,
                                resume: None,
                            },
                        );
                    } else if !self.route(id, Layer::Underground, (tx, ty)) {
                        self.set_job(id, Job::Idle);
                        self.set_retry(id, 60);
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
                                let mut kind = FoodKind::Green;
                                if let Ok(mut q) = self.ecs.get::<&mut Food>(fent) {
                                    q.amount -= 1;
                                    kind = q.kind;
                                }
                                if let Some(&aent) = self.ids.get(&id) {
                                    if let Ok(mut q) = self.ecs.get::<&mut Carrying>(aent) {
                                        q.amount = 1;
                                        q.kind = kind;
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
                    if carrying.amount == 0 {
                        self.set_job(id, Job::Idle);
                        continue;
                    }
                    let qtile = self.queen_tile();
                    if pos.layer == Layer::Underground {
                        if chebyshev(tile_of(pos.p), qtile) <= 1 {
                            match carrying.kind {
                                FoodKind::Green => self.colony.food += 1,
                                FoodKind::Super => self.colony.food_super += 1,
                            }
                            self.colony.delivered += 1;
                            if let Some(&aent) = self.ids.get(&id) {
                                if let Ok(mut q) = self.ecs.get::<&mut Carrying>(aent) {
                                    q.amount = 0;
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
                let target = path[next];
                let (tx, ty) = tile_of(target);
                if pos.layer == Layer::Underground {
                    let kind = self.world.underground.get(tx, ty);
                    if Grid::is_soft(kind) {
                        dig_target = Some((tx, ty));
                        break;
                    }
                }
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
                    resume: Some(Box::new((path, next, then_swap))),
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
            let (tx, ty, mut progress, resume) = match state {
                AntState::Digging {
                    tx,
                    ty,
                    progress,
                    resume,
                } => (tx, ty, progress, resume),
                _ => continue,
            };
            let kind = self.world.underground.get(tx, ty);
            if !Grid::is_soft(kind) {
                match resume {
                    Some(b) => {
                        self.set_state(
                            id,
                            AntState::Moving {
                                path: b.0,
                                next: b.1,
                                then_swap: b.2,
                            },
                        );
                    }
                    None => self.set_state(id, AntState::Idle),
                }
                continue;
            }
            progress += DT;
            if progress >= DIG_TIME {
                self.world.underground.set(tx, ty, EMPTY);
                self.dug_tiles += 1;
                match resume {
                    Some(b) => {
                        self.set_state(
                            id,
                            AntState::Moving {
                                path: b.0,
                                next: b.1,
                                then_swap: b.2,
                            },
                        );
                    }
                    None => self.set_state(id, AntState::Idle),
                }
            } else {
                self.set_state(
                    id,
                    AntState::Digging {
                        tx,
                        ty,
                        progress,
                        resume,
                    },
                );
            }
        }
    }

    fn combat(&mut self) {
        let ids = self.ant_ids();
        for id in ids {
            let ent = match self.ids.get(&id) {
                Some(&e) => e,
                None => continue,
            };
            let (pos, state, speed, dmg, atk_cd, atk_t) = {
                let mut qo = match self.ecs.query_one::<(&Pos, &AntState, &Ant, &Combat)>(ent) {
                    Ok(q) => q,
                    Err(_) => continue,
                };
                let q = match qo.get() {
                    Some(q) => q,
                    None => continue,
                };
                (*q.0, q.1.clone(), q.2.speed, q.3.dmg, q.3.atk_cd, q.3.atk_t)
            };
            let target = match state {
                AntState::Fighting { target } => target,
                _ => continue,
            };
            let Some(&tent) = self.ids.get(&target) else {
                self.set_state(id, AntState::Idle);
                self.set_job(id, Job::Idle);
                continue;
            };
            let tpos = self.ecs.get::<&Pos>(tent).map(|q| *q);
            let tpos = match tpos {
                Ok(t) => t,
                Err(_) => {
                    self.set_state(id, AntState::Idle);
                    self.set_job(id, Job::Idle);
                    continue;
                }
            };
            if tpos.layer != pos.layer {
                self.set_state(id, AntState::Idle);
                continue;
            }
            let mut new_pos = pos;
            let mut new_t = atk_t;
            let d = tpos.p - new_pos.p;
            let dist = d.len();
            if dist > ANT_RANGE {
                let step = speed * DT;
                if dist > step {
                    new_pos.p = new_pos.p + d * (step / dist);
                } else {
                    new_pos.p = tpos.p;
                }
                new_t = (new_t - DT).max(0.0);
            } else {
                new_t -= DT;
                if new_t <= 0.0 {
                    new_t = atk_cd;
                    if let Ok(mut c) = self.ecs.get::<&mut Combat>(tent) {
                        c.hp -= dmg;
                    }
                    if let Ok(mut p) = self.ecs.get::<&mut Predator>(tent) {
                        p.hp -= dmg;
                    }
                }
            }
            if let Some(q) = self
                .ecs
                .query_one::<(&mut Pos, &mut Combat)>(ent)
                .unwrap()
                .get()
            {
                q.0.p = new_pos.p;
                q.1.atk_t = new_t;
            }
        }
    }

    fn predators(&mut self) {
        let pids = self
            .ids
            .iter()
            .filter_map(|(&id, &ent)| self.ecs.get::<&Predator>(ent).is_ok().then_some(id))
            .collect::<Vec<u32>>();
        for pid in pids {
            let ent = match self.ids.get(&pid) {
                Some(&e) => e,
                None => continue,
            };
            let (mut pred, pos) = {
                let mut qo = match self.ecs.query_one::<(&mut Predator, &Pos)>(ent) {
                    Ok(q) => q,
                    Err(_) => continue,
                };
                let q = match qo.get() {
                    Some(q) => q,
                    None => continue,
                };
                (*q.0, *q.1)
            };
            if let Some(t) = pred.target {
                let valid = self
                    .ids
                    .get(&t)
                    .and_then(|&te| self.ecs.get::<&Pos>(te).ok())
                    .map(|p| p.layer == Layer::Surface)
                    .unwrap_or(false);
                if !valid {
                    pred.target = None;
                }
            }
            if pred.target.is_none() {
                let mut best: Option<(f64, u32)> = None;
                for aid in self.ant_ids() {
                    let aent = self.ids[&aid];
                    let Ok(ap) = self.ecs.get::<&Pos>(aent) else {
                        continue;
                    };
                    if ap.layer != Layer::Surface {
                        continue;
                    }
                    let d = (ap.p - pos.p).len();
                    if d <= SPIDER_AGGRO && best.map(|(bd, _)| d < bd).unwrap_or(true) {
                        best = Some((d, aid));
                    }
                }
                pred.target = best.map(|(_, id)| id);
            }
            let mut new_pos = pos;
            match pred.target {
                Some(t) => {
                    let tent = self.ids[&t];
                    let tp = match self.ecs.get::<&Pos>(tent) {
                        Ok(q) => *q,
                        Err(_) => {
                            continue;
                        }
                    };
                    let d = tp.p - new_pos.p;
                    let dist = d.len();
                    if dist > SPIDER.range {
                        let step = pred.speed * DT;
                        if dist > step {
                            new_pos.p = new_pos.p + d * (step / dist);
                        } else {
                            new_pos.p = tp.p;
                        }
                    } else {
                        pred.atk_t -= DT;
                        if pred.atk_t <= 0.0 {
                            pred.atk_t = pred.atk_cd;
                            if let Ok(mut c) = self.ecs.get::<&mut Combat>(tent) {
                                c.hp -= pred.dmg;
                            }
                        }
                    }
                }
                None => {
                    pred.wander_t -= DT;
                    let need_dest = pred
                        .dest
                        .map(|d| {
                            (d.0 - new_pos.p.x) * (d.0 - new_pos.p.x)
                                + (d.1 - new_pos.p.y) * (d.1 - new_pos.p.y)
                                < 0.05
                        })
                        .unwrap_or(true);
                    if need_dest || pred.wander_t <= 0.0 {
                        let dx = self.rng.range(-1.0, 1.0);
                        let dy = self.rng.range(-1.0, 1.0);
                        let len = (dx * dx + dy * dy).sqrt().max(0.001);
                        let dist = self.rng.range(0.0, SPIDER_WANDER);
                        let hx = pred.home.0 as f64 + 0.5 + dx / len * dist;
                        let hy = pred.home.1 as f64 + 0.5 + dy / len * dist;
                        pred.dest = Some((
                            hx.clamp(1.0, self.config.width as f64 - 2.0),
                            hy.clamp(1.0, self.config.height as f64 - 2.0),
                        ));
                        pred.wander_t = self.rng.range(3.0, 6.0);
                    }
                    if let Some((dx_, dy_)) = pred.dest {
                        let d = Vec2::new(dx_, dy_) - new_pos.p;
                        let dist = d.len();
                        let step = pred.speed * DT;
                        if dist > step {
                            new_pos.p = new_pos.p + d * (step / dist);
                        } else {
                            new_pos.p = Vec2::new(dx_, dy_);
                        }
                    }
                }
            }
            if let Ok(mut q) = self.ecs.query_one::<(&mut Predator, &mut Pos)>(ent) {
                if let Some(q) = q.get() {
                    *q.0 = pred;
                    q.1.p = new_pos.p;
                }
            }
        }
    }

    fn cleanup_deaths(&mut self) {
        let dead_ants: Vec<u32> = self
            .ant_ids()
            .into_iter()
            .filter(|&id| {
                self.ids
                    .get(&id)
                    .and_then(|&e| self.ecs.get::<&Combat>(e).ok())
                    .map(|c| c.hp <= 0.0)
                    .unwrap_or(false)
            })
            .collect();
        for id in dead_ants {
            self.kill(id);
        }
        let dead_predators: Vec<u32> = self
            .ids
            .iter()
            .filter_map(|(&id, &ent)| {
                self.ecs
                    .get::<&Predator>(ent)
                    .ok()
                    .filter(|p| p.hp <= 0.0)
                    .map(|_| id)
            })
            .collect();
        for pid in dead_predators {
            let ent = self.ids[&pid];
            let p = self.ecs.get::<&Pos>(ent).map(|q| q.p).unwrap_or_default();
            self.kill(pid);
            self.spawn_food(p, SUPER_PER_SPIDER, FoodKind::Super);
        }
    }

    fn caste_counts(&self) -> (u32, u32) {
        let mut workers = 0;
        let mut soldiers = 0;
        for id in self.ant_ids() {
            let ent = self.ids[&id];
            if let Ok(q) = self.ecs.get::<&Ant>(ent) {
                match q.caste {
                    Caste::Worker => workers += 1,
                    Caste::Soldier => soldiers += 1,
                    Caste::Queen => {}
                }
            }
        }
        (workers, soldiers)
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
        if self.colony.lay_cooldown <= 0.0 && self.colony.ant_count < self.config.max_ants {
            let (workers, soldiers) = self.caste_counts();
            let want = if self.colony.food >= SOLDIER_COST_GREEN
                && self.colony.food_super >= SOLDIER_COST_SUPER
                && soldiers * 2 < workers
            {
                Some(Caste::Soldier)
            } else if self.colony.food >= EGG_COST {
                Some(Caste::Worker)
            } else {
                None
            };
            if let Some(caste) = want {
                match self.free_egg_tile() {
                    Some(tile) => {
                        match caste {
                            Caste::Soldier => {
                                self.colony.food -= SOLDIER_COST_GREEN;
                                self.colony.food_super -= SOLDIER_COST_SUPER;
                            }
                            Caste::Worker => self.colony.food -= EGG_COST,
                            Caste::Queen => {}
                        }
                        self.colony.eggs_laid += 1;
                        self.colony.lay_cooldown = LAY_COOLDOWN;
                        self.spawn_egg(tile_center(tile.0, tile.1), caste);
                    }
                    None => {
                        if self.colony.dig_queue.len() < 3 {
                            if let Some(t) = self.pick_dig_target() {
                                self.colony.dig_queue.push(t);
                            }
                        }
                    }
                }
            }
        }
    }

    fn pick_dig_target(&self) -> Option<(u32, u32)> {
        let (qx, qy) = self.queen_tile();
        for r in 1u32..=3 {
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
                    if Grid::is_soft(self.world.underground.get(x, y))
                        && self.has_empty_neighbor(x, y)
                    {
                        return Some((x, y));
                    }
                }
            }
        }
        None
    }

    fn has_empty_neighbor(&self, x: u32, y: u32) -> bool {
        [(1u32, 0u32), (u32::MAX, 0), (0, 1), (0, u32::MAX)]
            .iter()
            .any(|&(dx, dy)| {
                let nx = x.wrapping_add(dx);
                let ny = y.wrapping_add(dy);
                self.world.underground.in_bounds(nx, ny)
                    && self.world.underground.get(nx, ny) == EMPTY
            })
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
            let (hatch, caste, pos) = match self.ecs.query_one::<(&Egg, &Pos)>(ent).unwrap().get() {
                Some(q) => (q.0.hatch, q.0.caste, *q.1),
                None => continue,
            };
            let hatch = hatch - DT;
            if hatch <= 0.0 && self.colony.ant_count < self.config.max_ants {
                let p = pos.p;
                self.kill(id);
                self.spawn_ant(caste, p, Layer::Underground);
            } else if let Ok(mut q) = self.ecs.get::<&mut Egg>(ent) {
                q.hatch = hatch.max(0.0);
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

    /// Canonical, platform-independent digest of the full sim state.
    /// Native and WASM builds must produce byte-identical strings for the
    /// same seed + command script — this is what the cross-platform
    /// determinism test compares.
    pub fn canonical_state(&self) -> String {
        let mut s = format!(
            "t={};food={};super={};dead={};dug={};next_id={}",
            self.tick,
            self.colony.food,
            self.colony.food_super,
            self.colony.dead,
            self.dug_tiles,
            self.next_id
        );
        for e in self.snapshot() {
            s.push_str(&format!(
                "|{},{},{},{},{:.4},{:.4},{:.4},{:.4},{:.4}",
                e.id, e.kind, e.layer, e.state, e.x, e.y, e.extra, e.hp, e.aux
            ));
        }
        s
    }

    pub fn snapshot(&self) -> Vec<EntitySnap> {
        let mut rev = std::collections::HashMap::new();
        for (&id, &ent) in &self.ids {
            rev.insert(ent, id);
        }
        let mut v = Vec::new();
        for (ent, (ant, pos, state, carry, combat)) in self
            .ecs
            .query::<(&Ant, &Pos, &AntState, &Carrying, &Combat)>()
            .iter()
        {
            let Some(&id) = rev.get(&ent) else { continue };
            let kind = match ant.caste {
                Caste::Queen => 0,
                Caste::Worker => 1,
                Caste::Soldier => 5,
            };
            let state = match state {
                AntState::Idle => 0u8,
                AntState::Moving { .. } => 1,
                AntState::Digging { .. } => 2,
                AntState::Fighting { .. } => 3,
            };
            let extra = if ant.caste == Caste::Queen {
                self.colony.starve_t / STARVE_TIME
            } else {
                carry.amount as f64
            };
            v.push(EntitySnap {
                id,
                kind,
                layer: pos.layer as u8,
                x: pos.p.x,
                y: pos.p.y,
                state,
                extra,
                hp: (combat.hp / combat.max_hp).clamp(0.0, 1.0),
                aux: carry.kind as u8 as f64,
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
                hp: 1.0,
                aux: food.kind as u8 as f64,
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
                hp: 1.0,
                aux: if egg.caste == Caste::Soldier {
                    1.0
                } else {
                    0.0
                },
            });
        }
        for (ent, (pred, pos)) in self.ecs.query::<(&Predator, &Pos)>().iter() {
            let Some(&id) = rev.get(&ent) else { continue };
            v.push(EntitySnap {
                id,
                kind: 4,
                layer: pos.layer as u8,
                x: pos.p.x,
                y: pos.p.y,
                state: if pred.target.is_some() { 1 } else { 0 },
                extra: pred.hp / pred.max_hp,
                hp: pred.hp / pred.max_hp,
                aux: 0.0,
            });
        }
        v.sort_by_key(|s| s.id);
        v
    }
}
