//! The simulation: world state, entity bookkeeping, the tick pipeline, and
//! the dev/test operations. World generation lives in `worldgen`, player
//! commands in `commands`, worker AI in `ai`, the per-tick systems in
//! `systems`, food logistics in `food`, grid/block/soil geometry in `geom`,
//! and the client-facing snapshot + canonical digest in `snapshot`.

mod ai;
mod commands;
mod food;
mod geom;
pub(crate) mod snapshot;
mod systems;
mod worldgen;

pub use snapshot::{
    snapshot_spec, Activity, AntSnap, Blocked, EggSnap, EntitySnap, FoodRole, FoodSnap, Intent,
    SpiderSnap,
};

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::components::*;
use crate::math::Vec2;
use crate::path::tile_of;
use crate::rng::Rng;
use crate::rules::GameRules;
use crate::world::World;

pub const TPS: u32 = 20;
pub const DT: f64 = 1.0 / TPS as f64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Team {
    Red = 0,
    Blue = 1,
}

/// Game-start state machine. The founded (legacy) constructor jumps straight
/// to `Colony`; the founding constructor starts at `Flight` with a lone queen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Queen flying over the surface, choosing a nest site (state code 4).
    Flight = 0,
    /// Queen landed; walks the surface; right-click founds the nest.
    Grounded = 1,
    /// Nest founded; excavation window before the first brood (`t` = seconds left).
    Founding = 2,
    /// First brood incubating; queen may keep excavating.
    Brood = 3,
    /// Workers exist; standard colony gameplay.
    Colony = 4,
}

#[derive(Clone, Copy, Debug)]
pub struct Patch {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
    /// SOIL_ORANGE or SOIL_SILVER — founding inside grants that soil nearby.
    pub soil: u8,
}

#[derive(Clone, Debug)]
pub enum Command {
    Move {
        ant: u32,
        x: f64,
        y: f64,
    },
    /// Dig the aligned 2×2 block containing (tx, ty). If the ant is not
    /// adjacent yet it walks there first and digs on arrival.
    Dig {
        ant: u32,
        tx: u32,
        ty: u32,
    },
    Attack {
        ant: u32,
        target: u32,
    },
    UseEntrance {
        ant: u32,
    },
    /// Flying queen flies to (x, y), then lands there.
    Land {
        ant: u32,
        x: f64,
        y: f64,
    },
    /// Grounded queen walks to (x, y), then founds the nest at that tile.
    FoundNest {
        ant: u32,
        x: f64,
        y: f64,
    },
    /// Drop the carried item at (tx, ty): dirt fills the adjacent fully-empty
    /// 2×2 block (or vanishes on the surface), an egg is placed on the
    /// adjacent empty cell, food is dropped as one unit (≤ cap per cell).
    Drop {
        ant: u32,
        tx: u32,
        ty: u32,
    },
    /// Squad control (X-menu): the issuing ant is the leader. All/One
    /// recruit nearby ants, Soldiers recruits every soldier, Release
    /// disbands the leader's squad.
    Follow {
        leader: u32,
        mode: FollowMode,
    },
    /// Pick up an adjacent egg (keeps its hatch state; carried eggs ride the
    /// carrier and cannot hatch).
    PickEgg {
        ant: u32,
        egg: u32,
    },
    /// Queen X-menu brood order (F4): lay one egg of `caste`, paying the
    /// brood cost from the physical pantry; a soldier order additionally
    /// consumes one worker (metamorphosis).
    Brood {
        ant: u32,
        caste: Caste,
    },
}

/// Spawnable entities for dev tools. Natural layers: spiders/food on the
/// surface, ants/eggs underground (matching real game behavior).
/// Squad-follow recruitment mode (X-menu).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FollowMode {
    /// Every ant within sight of the leader joins the squad.
    All,
    /// The nearest ant not already in the squad joins.
    One,
    /// Every soldier joins the squad.
    Soldiers,
    /// The leader's squad disbands back to idle.
    Release,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevSpawn {
    Worker,
    Soldier,
    EggWorker,
    EggSoldier,
    Spider,
    Food,
    SuperFood,
    /// Wet wood collectible (food-storage block).
    Wood,
    /// Dry wool collectible (egg-friendly block).
    Wool,
    /// A map source by visual type 1..7.
    Source(u8),
    /// One stored pantry unit (silver-cell pile, ledger synced) — F4 testing:
    /// brood costs and medic water are paid from PHYSICAL piles, so dev tools
    /// must place real ones (dev_set_* only writes the ledger).
    PantryProtein,
    PantryCarbs,
    PantryWater,
    PantryHoneydew,
    /// F4 specialist castes (dev tools / tests).
    HoneyAnt,
    MedicAnt,
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
            "wood" => DevSpawn::Wood,
            "wool" => DevSpawn::Wool,
            "moss" => DevSpawn::Source(1),
            "mushroom" => DevSpawn::Source(2),
            "raspberry" => DevSpawn::Source(3),
            "strawberry" => DevSpawn::Source(4),
            "cockroach" => DevSpawn::Source(5),
            "caterpillar" => DevSpawn::Source(6),
            "nettle" => DevSpawn::Source(7),
            "pantry-protein" => DevSpawn::PantryProtein,
            "pantry-carbs" => DevSpawn::PantryCarbs,
            "pantry-water" => DevSpawn::PantryWater,
            "pantry-honeydew" => DevSpawn::PantryHoneydew,
            "honey" => DevSpawn::HoneyAnt,
            "medic" => DevSpawn::MedicAnt,
            _ => return None,
        })
    }
}

pub struct Colony {
    pub carbs: u32,
    pub protein: u32,
    pub water: u32,
    pub honeydew: u32,
    pub delivered: u32,
    pub eggs_laid: u32,
    pub dead: bool,
    pub queen_id: u32,
    pub ant_count: u32,
    pub starve_t: f64,
    pub eat_t: f64,
    pub lay_cooldown: f64,
    pub dig_queue: Vec<(u32, u32)>,
    /// Game-start phase (see `Phase`).
    pub phase: Phase,
    pub team: Team,
    /// True when started via `new_founding`: the founding script (4 eggs,
    /// no auto-laying) governs brood; false in the legacy founded start.
    pub founding: bool,
    /// Founding water quest: water units the queen has stored on food
    /// blocks (silver). She can lay the founding brood on an egg block
    /// once this reaches `rules.founding_quest_water`. One-way — the
    /// ritual payment is not refunded if the water is later eaten.
    pub quest_water_tally: u32,
    /// Seconds since the founding queen last ate (physical feeding model);
    /// hungry at `eat_period`, dead at `eat_period + starve_time` (rules).
    pub hunger_t: f64,
    /// Index into the rules' craving cycle — what the queen requests next.
    pub craving_i: usize,
    /// The designated feeder (founding): one worker whose job is feeding her.
    pub feeder_id: Option<u32>,
    /// Source ids discovered by the colony (within sight of any ant).
    pub known: std::collections::BTreeSet<u32>,
}

/// Cap of retained game events (drop-oldest ring via Vec drain).
pub const EVENT_CAP: usize = 400;

pub struct Sim {
    pub world: World,
    pub ecs: hecs::World,
    pub rng: Rng,
    pub colony: Colony,
    /// The complete ruleset this sim runs under (tuning data, not code —
    /// see `rules.rs`; folded into the canonical digest).
    pub rules: GameRules,
    pub tick: u64,
    /// Capped sim-side event log: everything the game did, newest last.
    pub events: VecDeque<String>,
    /// Total events ever emitted (retained = events.len()).
    pub event_total: u64,
    /// Surface dust patches (founding mode): founding inside one grants its
    /// soil color near the nest.
    pub patches: Vec<Patch>,
    pub(crate) ids: BTreeMap<u32, hecs::Entity>,
    pub(crate) next_id: u32,
    pub(crate) dug_tiles: u32,
    /// Food entities by (layer, tile) — they never move, so this index turns
    /// the per-tile pantry/cap scans from O(all food) into O(food on tile).
    /// BTree everywhere: iteration order is deterministic.
    pub(crate) food_tiles: BTreeMap<(u8, u32, u32), BTreeSet<u32>>,
    /// Bumped by every tile mutation (dig, carve, refill). Lets the client
    /// skip the per-tick grid re-pull — the same lever the Phase-4 network
    /// protocol will pull.
    pub tiles_epoch: u64,
    /// Bumped by every soil mutation (worldgen grants, dev painting).
    pub soil_epoch: u64,
}

impl Sim {
    /// Legacy founded start: pre-dug nest, queen + `start_workers` workers.
    pub fn new(seed: u64, rules: GameRules) -> Sim {
        Self::build(seed, rules, false, Team::Red)
    }

    /// Founding start: a lone queen flying over the surface at map center,
    /// under the default ruleset.
    pub fn new_founding(seed: u64, team: Team) -> Sim {
        Self::build(seed, GameRules::default(), true, team)
    }

    /// Founding start under explicit rules (the admin panel's "apply & new
    /// game" path). Parses and validates first: a bad ruleset must fail here,
    /// before any world exists — never silently fall back to defaults.
    pub fn new_founding_with(seed: u64, team: Team, rules: GameRules) -> Sim {
        Self::build(seed, rules, true, team)
    }

    /// Parse + validate a whole-rules JSON document (JSONC: `//` and
    /// `/* */` comments are stripped first, exactly like the shipped data
    /// files). Shared by the wasm boundary and the replay path so a commit
    /// and its replay apply the exact same bytes.
    pub fn parse_rules(json: &str) -> Result<GameRules, Vec<String>> {
        let clean = crate::data::strip_jsonc(json);
        match serde_json::from_str::<GameRules>(&clean) {
            Ok(rules) => match rules.validate() {
                Ok(()) => Ok(rules),
                Err(errs) => Err(errs),
            },
            Err(e) => Err(vec![format!("JSON: {e}")]),
        }
    }

    /// The current ruleset as JSON (serde, whole-object; pairs with
    /// `set_rules`).
    pub fn rules_json(&self) -> String {
        serde_json::to_string(&self.rules).expect("GameRules serializes")
    }

    /// Rules digest as 16 hex chars (the same value folded into the canon).
    pub fn rules_digest_hex(&self) -> String {
        format!("{:016x}", self.rules.digest())
    }

    /// Atomically replace the ruleset: parse + validate (all errors at once),
    /// then commit and re-derive the entity-copied stats (speed, combat
    /// numbers — hp fraction preserved) so "live" fields really are live for
    /// existing entities too. Returns the new digest.
    pub fn set_rules(&mut self, json: &str) -> Result<String, Vec<String>> {
        let rules = Self::parse_rules(json)?;
        let digest = self.rules_digest_hex();
        self.rules = rules;
        self.resync_stats();
        let new_digest = self.rules_digest_hex();
        self.ev(format!("rules updated ({digest} → {new_digest})"));
        Ok(new_digest)
    }

    /// After a rules commit, refresh every per-entity copy of the tunables:
    /// `Ant::speed` and `Combat` are snapshotted from the rules at spawn, so
    /// without this a live speed/hp change would only affect ants yet to be
    /// born. HP keeps its fraction (a half-dead spider stays half-dead of the
    /// new maximum).
    fn resync_stats(&mut self) {
        let ants = self.ant_ids();
        for id in ants {
            let Some(&ent) = self.ids.get(&id) else {
                continue;
            };
            let Some(caste) = self
                .ecs
                .query_one::<&Ant>(ent)
                .ok()
                .and_then(|mut q| q.get().map(|a| a.caste))
            else {
                continue;
            };
            let st = *self.rules.stats_for(caste);
            if let Ok(mut a) = self.ecs.get::<&mut Ant>(ent) {
                a.speed = st.speed;
            }
            if let Ok(mut c) = self.ecs.get::<&mut Combat>(ent) {
                let frac = (c.hp / c.max_hp).clamp(0.0, 1.0);
                c.max_hp = st.hp;
                c.hp = frac * st.hp;
                c.dmg = st.dmg;
                c.atk_cd = st.atk_cd;
            }
        }
        let spiders: Vec<u32> = self
            .ids
            .iter()
            .filter(|(_, &e)| self.ecs.get::<&Predator>(e).is_ok())
            .map(|(&id, _)| id)
            .collect();
        for id in spiders {
            let Some(&ent) = self.ids.get(&id) else {
                continue;
            };
            let st = self.rules.spider;
            if let Ok(mut p) = self.ecs.get::<&mut Predator>(ent) {
                let frac = (p.hp / p.max_hp).clamp(0.0, 1.0);
                p.max_hp = st.hp;
                p.hp = frac * st.hp;
                p.dmg = st.dmg;
                p.speed = st.speed;
                p.atk_cd = st.atk_cd;
            }
        }
    }

    pub fn issue(&mut self, cmd: Command) -> bool {
        self.apply_command(cmd)
    }

    /// Record a game event (drop-oldest cap). Purely observational — never
    /// part of sim state, replays, or canonical output.
    pub(crate) fn ev(&mut self, text: String) {
        if self.events.len() >= EVENT_CAP {
            self.events.pop_front();
        }
        self.events.push_back(format!("[{}] {}", self.tick, text));
        self.event_total += 1;
    }

    /// Retained events, oldest first (capped at EVENT_CAP).
    pub fn event_lines(&self) -> Vec<String> {
        self.events.iter().cloned().collect()
    }

    pub fn event_total(&self) -> u64 {
        self.event_total
    }

    /// Dev/test operations. They mutate the sim directly (no command gating)
    /// and are recorded with ticks by the client, so a dev action stream is
    /// as replayable as any other command stream.
    pub fn dev_spawn(&mut self, what: DevSpawn, x: f64, y: f64) -> u32 {
        let p = Vec2::new(x, y);
        match what {
            DevSpawn::Worker => self.spawn_ant(Caste::Worker, p, Layer::Underground),
            DevSpawn::Soldier => self.spawn_ant(Caste::Soldier, p, Layer::Underground),
            DevSpawn::EggWorker => self.spawn_egg(p, Caste::Worker, self.rules.egg_time),
            DevSpawn::EggSoldier => self.spawn_egg(p, Caste::Soldier, self.rules.egg_time),
            DevSpawn::Spider => self.spawn_spider(p),
            DevSpawn::Food => self.spawn_food(p, self.rules.pile_amount, FoodKind::Green),
            DevSpawn::SuperFood => self.spawn_food(p, self.rules.super_per_spider, FoodKind::Super),
            DevSpawn::Wood => self.spawn_collectible(Vec2::new(x, y), CollectibleVariant::Wood),
            DevSpawn::Wool => self.spawn_collectible(Vec2::new(x, y), CollectibleVariant::Wool),
            DevSpawn::PantryProtein => self.dev_pantry_unit(x, y, FoodKind::Protein),
            DevSpawn::PantryCarbs => self.dev_pantry_unit(x, y, FoodKind::Carbs),
            DevSpawn::PantryWater => self.dev_pantry_unit(x, y, FoodKind::Water),
            DevSpawn::PantryHoneydew => self.dev_pantry_unit(x, y, FoodKind::Honeydew),
            DevSpawn::HoneyAnt => self.spawn_ant(Caste::Honey, p, Layer::Underground),
            DevSpawn::MedicAnt => self.spawn_ant(Caste::Medic, p, Layer::Underground),
            DevSpawn::Source(src) => {
                let spec = self.rules.sources.iter().find(|s| s.src == src).copied();
                match spec {
                    Some(spec) => {
                        let amount = (spec.amount.0 + spec.amount.1) / 2;
                        self.spawn_source(p, &spec, amount)
                    }
                    None => self.spawn_food(p, self.rules.pile_amount, FoodKind::Green),
                }
            }
        }
    }

    pub fn dev_set_food(&mut self, n: u32) {
        self.colony.carbs = n;
    }

    pub fn dev_set_super(&mut self, n: u32) {
        self.colony.protein = n;
    }

    pub fn dev_set_water(&mut self, n: u32) {
        self.colony.water = n;
    }

    pub fn dev_set_honeydew(&mut self, n: u32) {
        self.colony.honeydew = n;
    }

    /// One stored pantry unit at (x, y) with the ledger synced — brood costs
    /// and medic water are paid physically, so tests/tools place real piles.
    fn dev_pantry_unit(&mut self, x: f64, y: f64, kind: FoodKind) -> u32 {
        let tile = (x.floor() as u32, y.floor() as u32);
        let id = self.spawn_food_entity(Layer::Underground, tile, 1, kind, true, None);
        match kind {
            FoodKind::Green | FoodKind::Carbs => self.colony.carbs += 1,
            FoodKind::Super | FoodKind::Protein => self.colony.protein += 1,
            FoodKind::Water => self.colony.water += 1,
            FoodKind::Honeydew => self.colony.honeydew += 1,
        }
        id
    }

    /// Paint a 2×2 soil block (dev/test op, deterministic + replayable).
    /// `soil`: 0 none, 1 orange, 2 silver.
    pub fn dev_set_soil(&mut self, layer_num: u32, x: u32, y: u32, soil: u32) {
        let layer = if layer_num == 0 {
            Layer::Surface
        } else {
            Layer::Underground
        };
        let (bx, by) = crate::world::block_of(x, y);
        if bx + 1 < self.rules.width && by + 1 < self.rules.height {
            let soil = match soil {
                1 => crate::world::SOIL_ORANGE,
                2 => crate::world::SOIL_SILVER,
                _ => crate::world::SOIL_NONE,
            };
            self.set_soil_block(layer, bx, by, soil);
        }
    }

    pub fn dev_kill(&mut self, id: u32) -> bool {
        if !self.ids.contains_key(&id) {
            return false;
        }
        self.kill_cause(id, "dev");
        true
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        self.worker_ai();
        self.movement();
        self.discover();
        self.digging();
        self.harvesting();
        self.combat();
        self.predators();
        self.cleanup_deaths();
        self.queen_system();
        self.specialists();
        self.eggs();
        self.cleanup();
    }

    // --- entity bookkeeping ---

    pub(crate) fn fresh_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub(crate) fn spawn_ant(&mut self, caste: Caste, p: Vec2, layer: Layer) -> u32 {
        let id = self.fresh_id();
        let st = self.rules.stats_for(caste);
        let ent = self.ecs.spawn((
            Ant {
                caste,
                speed: st.speed,
                gen_t: 0.0,
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
            Carry::None,
            WorkerAi {
                job: if caste == Caste::Soldier {
                    Job::Manual
                } else {
                    Job::Idle
                },
                retry: 0,
                pending: None,
                attack_after: None,
                dig_after: None,
                drop_after: None,
                pick_after: None,
                land_after: false,
                found_after: None,
            },
        ));
        self.ids.insert(id, ent);
        self.colony.ant_count += 1;
        if self.tick > 0 {
            let name = match caste {
                Caste::Queen => "queen",
                Caste::Worker => "worker",
                Caste::Soldier => "soldier",
                Caste::Honey => "honey ant",
                Caste::Medic => "medic",
            };
            self.ev(format!("{name} #{id} born"));
        }
        id
    }

    pub(crate) fn spawn_egg(&mut self, p: Vec2, caste: Caste, total: f64) -> u32 {
        let id = self.fresh_id();
        if self.tick > 0 {
            self.ev(format!("egg #{id} laid at ({:.0},{:.0})", p.x, p.y));
        }
        let ent = self.ecs.spawn((
            Egg {
                hatch: total,
                total,
                caste,
                carried_by: None,
            },
            Pos {
                p,
                layer: Layer::Underground,
            },
        ));
        self.ids.insert(id, ent);
        id
    }

    pub(crate) fn spawn_spider(&mut self, p: Vec2) -> u32 {
        let id = self.fresh_id();
        let home = (p.x.floor() as u32, p.y.floor() as u32);
        let ent = self.ecs.spawn((
            Predator {
                hp: self.rules.spider.hp,
                max_hp: self.rules.spider.hp,
                dmg: self.rules.spider.dmg,
                speed: self.rules.spider.speed,
                atk_cd: self.rules.spider.atk_cd,
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

    pub(crate) fn kill(&mut self, id: u32) {
        self.kill_cause(id, "?");
    }

    pub(crate) fn kill_cause(&mut self, id: u32, cause: &str) {
        if let Some(ent) = self.ids.remove(&id) {
            let was_ant = self.ecs.get::<&Ant>(ent).is_ok();
            // a downed ant dying (bleed-out, dev kill) releases its medic's
            // mandibles — the carrier holds no id, so free them explicitly
            if let Some(carrier) = self.ecs.get::<&Fallen>(ent).ok().and_then(|f| f.carried_by) {
                if let Some(&cent) = self.ids.get(&carrier) {
                    if let Ok(mut q) = self.ecs.get::<&mut Carry>(cent) {
                        *q = Carry::None;
                    }
                }
            }
            if let (Ok(pos), true) = (
                self.ecs.get::<&Pos>(ent),
                self.ecs.get::<&Food>(ent).is_ok(),
            ) {
                let key = (
                    pos.layer as u8,
                    pos.p.x.floor() as u32,
                    pos.p.y.floor() as u32,
                );
                if let Some(set) = self.food_tiles.get_mut(&key) {
                    set.remove(&id);
                    if set.is_empty() {
                        self.food_tiles.remove(&key);
                    }
                }
            }
            let _ = self.ecs.despawn(ent);
            if was_ant {
                self.colony.ant_count -= 1;
                // the queen's death by any means (starvation, combat) ends the
                // colony — the systems below dereference queen_id every tick
                if id == self.colony.queen_id {
                    self.colony.dead = true;
                    self.ev(format!("COLONY DIED — queen #{id} ({cause})"));
                } else {
                    self.ev(format!("ant #{id} died ({cause})"));
                }
            }
        }
    }

    pub(crate) fn ant_ids(&self) -> Vec<u32> {
        self.ids
            .iter()
            .filter_map(|(&id, &ent)| self.ecs.get::<&Ant>(ent).is_ok().then_some(id))
            .collect()
    }

    pub(crate) fn food_ids(&self) -> Vec<u32> {
        self.ids
            .iter()
            .filter_map(|(&id, &ent)| self.ecs.get::<&Food>(ent).is_ok().then_some(id))
            .collect()
    }

    pub(crate) fn egg_ids(&self) -> Vec<u32> {
        self.ids
            .iter()
            .filter_map(|(&id, &ent)| self.ecs.get::<&Egg>(ent).is_ok().then_some(id))
            .collect()
    }

    pub(crate) fn is_ant(&self, id: u32) -> bool {
        self.ids
            .get(&id)
            .map(|&e| self.ecs.get::<&Ant>(e).is_ok())
            .unwrap_or(false)
    }

    pub(crate) fn ant_layer(&self, id: u32) -> Layer {
        let ent = self.ids[&id];
        self.ecs
            .get::<&Pos>(ent)
            .map(|q| q.layer)
            .unwrap_or(Layer::Underground)
    }

    pub(crate) fn ant_pos(&self, id: u32) -> Vec2 {
        let ent = self.ids[&id];
        self.ecs.get::<&Pos>(ent).map(|q| q.p).unwrap_or_default()
    }

    pub(crate) fn ant_tile(&self, id: u32) -> (u32, u32) {
        let ent = self.ids[&id];
        self.ecs
            .get::<&Pos>(ent)
            .map(|q| tile_of(q.p))
            .unwrap_or((0, 0))
    }

    // --- component mutators (single place per field) ---

    pub(crate) fn set_state(&mut self, id: u32, state: AntState) {
        if let Some(&ent) = self.ids.get(&id) {
            if let Ok(mut q) = self.ecs.get::<&mut AntState>(ent) {
                *q = state;
            }
        }
    }

    pub(crate) fn with_ai(&mut self, id: u32, f: impl FnOnce(&mut WorkerAi)) {
        if let Some(&ent) = self.ids.get(&id) {
            if let Ok(mut q) = self.ecs.get::<&mut WorkerAi>(ent) {
                f(&mut q);
            }
        }
    }

    pub(crate) fn set_job(&mut self, id: u32, job: Job) {
        self.with_ai(id, |ai| ai.job = job);
    }

    /// A fight ended for this ant: idle movement now. Player-commanded
    /// (Manual) ants are released back to autonomy — an attack order
    /// shouldn't leave war parties as statues forever. Squad followers
    /// keep their Follow job (they return to the leader).
    pub(crate) fn end_fight(&mut self, id: u32) {
        self.set_state(id, AntState::Idle);
        let manual = self
            .ids
            .get(&id)
            .and_then(|&e| self.ecs.get::<&WorkerAi>(e).ok())
            .map(|ai| matches!(ai.job, Job::Manual))
            .unwrap_or(false);
        if manual {
            self.set_job(id, Job::Idle);
        }
    }

    pub(crate) fn set_pending(&mut self, id: u32, pending: Option<(Layer, (u32, u32))>) {
        self.with_ai(id, |ai| ai.pending = pending);
    }

    pub(crate) fn set_retry(&mut self, id: u32, retry: u32) {
        self.with_ai(id, |ai| ai.retry = retry);
    }

    pub(crate) fn set_attack_after(&mut self, id: u32, target: Option<u32>) {
        self.with_ai(id, |ai| ai.attack_after = target);
    }

    pub(crate) fn set_dig_after(&mut self, id: u32, target: Option<(u32, u32)>) {
        self.with_ai(id, |ai| ai.dig_after = target);
    }

    pub(crate) fn set_drop_after(&mut self, id: u32, target: Option<(u32, u32)>) {
        self.with_ai(id, |ai| ai.drop_after = target);
    }

    pub(crate) fn set_pick_after(&mut self, id: u32, egg: Option<u32>) {
        self.with_ai(id, |ai| ai.pick_after = egg);
    }

    pub(crate) fn set_land_after(&mut self, id: u32, on: bool) {
        self.with_ai(id, |ai| ai.land_after = on);
    }

    pub(crate) fn set_found_after(&mut self, id: u32, tile: Option<(u32, u32)>) {
        self.with_ai(id, |ai| ai.found_after = tile);
    }

    // --- world accessors ---

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
}
