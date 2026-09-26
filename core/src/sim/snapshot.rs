//! The client-facing snapshot (typed shapes, one named field per fact) and
//! the canonical full-state digest used by the cross-platform determinism
//! test. The wire packing into a flat f64 array lives in `lib.rs` and is the
//! ONLY place the positional codes exist on the Rust side; `snapshot_spec()`
//! exports them so the client can assert its decoder matches at boot.

use super::Sim;
use crate::components::*;

// --- wire codes (single source of truth; exported via snapshot_spec) ---

pub const KIND_QUEEN: u8 = 0;
pub const KIND_WORKER: u8 = 1;
pub const KIND_SOLDIER: u8 = 2;
pub const KIND_EGG: u8 = 3;
pub const KIND_SPIDER: u8 = 4;
pub const KIND_FOOD: u8 = 5;
pub const KIND_SOURCE: u8 = 6;

pub const ACT_IDLE: u8 = 0;
pub const ACT_MOVING: u8 = 1;
pub const ACT_DIGGING: u8 = 2;
pub const ACT_FIGHTING: u8 = 3;
pub const ACT_FLYING: u8 = 4;

pub const CARRY_NONE: u8 = 0;
pub const CARRY_DIRT: u8 = 1;
pub const CARRY_EGG: u8 = 2;
pub const CARRY_FOOD: u8 = 3;

pub const FOOD_GREEN: u8 = 0;
pub const FOOD_SUPER: u8 = 1;
pub const FOOD_PROTEIN: u8 = 2;
pub const FOOD_CARB: u8 = 3;
pub const FOOD_WATER: u8 = 4;

pub fn food_code(kind: FoodKind) -> u8 {
    match kind {
        FoodKind::Green => FOOD_GREEN,
        FoodKind::Super => FOOD_SUPER,
        FoodKind::Protein => FOOD_PROTEIN,
        FoodKind::Carbs => FOOD_CARB,
        FoodKind::Water => FOOD_WATER,
    }
}

/// JSON description of the snapshot wire codes, for the client's boot-time
/// decoder assertion. Bump `snapshot` when the layout changes.
pub fn snapshot_spec() -> String {
    format!(
        "{{\"snapshot\":1,\"stride\":10,\"kinds\":{{\"queen\":{KIND_QUEEN},\"worker\":{KIND_WORKER},\"soldier\":{KIND_SOLDIER},\"egg\":{KIND_EGG},\"spider\":{KIND_SPIDER},\"food\":{KIND_FOOD},\"source\":{KIND_SOURCE}}},\"activity\":{{\"idle\":{ACT_IDLE},\"moving\":{ACT_MOVING},\"digging\":{ACT_DIGGING},\"fighting\":{ACT_FIGHTING},\"flying\":{ACT_FLYING}}},\"carry\":{{\"none\":{CARRY_NONE},\"dirt\":{CARRY_DIRT},\"egg\":{CARRY_EGG},\"food\":{CARRY_FOOD}}},\"food\":{{\"green\":{FOOD_GREEN},\"super\":{FOOD_SUPER},\"protein\":{FOOD_PROTEIN},\"carbs\":{FOOD_CARB},\"water\":{FOOD_WATER}}}}}"
    )
}

// --- typed snapshot shapes ---

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Idle,
    Moving,
    Digging,
    Fighting,
    /// The founding queen is airborne (flight phase).
    Flying,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AntSnap {
    pub id: u32,
    pub caste: Caste,
    pub layer: Layer,
    pub x: f64,
    pub y: f64,
    pub activity: Activity,
    /// HP fraction 0..=1.
    pub hp: f64,
    pub carry: Carry,
    /// Starvation progress 0..=1 (queen only; 0 for everyone else).
    pub hunger: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FoodRole {
    /// Dropped unit: spoils off silver. `spoil` = remaining fraction
    /// (None = not spoiling: pantry and worldgen piles never do).
    Loose { spoil: Option<f64> },
    /// Banked pantry pile: safe, visible, not a forage target.
    Pantry,
    /// Finite map source; `src` is the visual type 1..6.
    Source { src: u8 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct FoodSnap {
    pub id: u32,
    pub layer: Layer,
    pub x: f64,
    pub y: f64,
    pub amount: u32,
    pub kind: FoodKind,
    pub role: FoodRole,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EggSnap {
    pub id: u32,
    pub layer: Layer,
    pub x: f64,
    pub y: f64,
    /// Remaining incubation fraction: 1 fresh → 0 ready to hatch.
    pub hatch_left: f64,
    /// What hatches.
    pub caste: Caste,
    /// Riding a carrier right now.
    pub carried: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpiderSnap {
    pub id: u32,
    pub layer: Layer,
    pub x: f64,
    pub y: f64,
    /// HP fraction 0..=1.
    pub hp: f64,
    /// Has a prey target.
    pub hunting: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EntitySnap {
    Ant(AntSnap),
    Food(FoodSnap),
    Egg(EggSnap),
    Spider(SpiderSnap),
}

impl EntitySnap {
    pub fn id(&self) -> u32 {
        match self {
            EntitySnap::Ant(e) => e.id,
            EntitySnap::Food(e) => e.id,
            EntitySnap::Egg(e) => e.id,
            EntitySnap::Spider(e) => e.id,
        }
    }
}

pub(crate) fn source_name(src: u8) -> &'static str {
    match src {
        1 => "moss",
        2 => "mushroom",
        3 => "raspberry",
        4 => "strawberry",
        5 => "cockroach",
        6 => "caterpillar",
        _ => "source",
    }
}

impl Sim {
    pub fn snapshot(&self) -> Vec<EntitySnap> {
        let mut rev = std::collections::HashMap::new();
        for (&id, &ent) in &self.ids {
            rev.insert(ent, id);
        }
        let mut v = Vec::new();
        for (ent, (ant, pos, state, carry, combat)) in self
            .ecs
            .query::<(&Ant, &Pos, &AntState, &Carry, &Combat)>()
            .iter()
        {
            let Some(&id) = rev.get(&ent) else { continue };
            let activity = if ant.caste == Caste::Queen
                && self.colony.phase == super::Phase::Flight
            {
                Activity::Flying
            } else {
                match state {
                    AntState::Idle => Activity::Idle,
                    AntState::Moving { .. } => Activity::Moving,
                    AntState::Digging { .. } => Activity::Digging,
                    AntState::Fighting { .. } => Activity::Fighting,
                }
            };
            v.push(EntitySnap::Ant(AntSnap {
                id,
                caste: ant.caste,
                layer: pos.layer,
                x: pos.p.x,
                y: pos.p.y,
                activity,
                hp: (combat.hp / combat.max_hp).clamp(0.0, 1.0),
                carry: *carry,
                hunger: if ant.caste == Caste::Queen {
                    self.colony.starve_t / crate::balance::STARVE_TIME
                } else {
                    0.0
                },
            }));
        }
        for (ent, (food, pos)) in self.ecs.query::<(&Food, &Pos)>().iter() {
            let Some(&id) = rev.get(&ent) else { continue };
            let role = if food.harvest_t > 0.0 {
                FoodRole::Source { src: food.src }
            } else if food.stored {
                FoodRole::Pantry
            } else {
                FoodRole::Loose {
                    spoil: food
                        .spoil
                        .map(|t| (t / crate::balance::SPOIL_TIME).clamp(0.0, 1.0)),
                }
            };
            v.push(EntitySnap::Food(FoodSnap {
                id,
                layer: pos.layer,
                x: pos.p.x,
                y: pos.p.y,
                amount: food.amount,
                kind: food.kind,
                role,
            }));
        }
        for (ent, (egg, pos)) in self.ecs.query::<(&Egg, &Pos)>().iter() {
            let Some(&id) = rev.get(&ent) else { continue };
            v.push(EntitySnap::Egg(EggSnap {
                id,
                layer: pos.layer,
                x: pos.p.x,
                y: pos.p.y,
                hatch_left: egg.hatch / egg.total,
                caste: egg.caste,
                carried: egg.carried_by.is_some(),
            }));
        }
        for (ent, (pred, pos)) in self.ecs.query::<(&Predator, &Pos)>().iter() {
            let Some(&id) = rev.get(&ent) else { continue };
            v.push(EntitySnap::Spider(SpiderSnap {
                id,
                layer: pos.layer,
                x: pos.p.x,
                y: pos.p.y,
                hp: pred.hp / pred.max_hp,
                hunting: pred.target.is_some(),
            }));
        }
        v.sort_by_key(|s| s.id());
        v
    }

    /// Canonical, platform-independent digest of the FULL sim state — every
    /// field that influences the future, not just the client view. Native and
    /// WASM builds must produce byte-identical strings for the same seed +
    /// command script; this is what the cross-platform determinism test
    /// compares. (The first canonical reused the render snapshot and missed
    /// the soil grids, known sources, harvest/dig progress and AI internals.)
    pub fn canonical_state(&self) -> String {
        let mut s = String::new();
        let c = &self.colony;
        let entrance = self
            .world
            .entrance
            .map(|(x, y)| format!("{x},{y}"))
            .unwrap_or_else(|| "-".to_string());
        let (rng_state, rng_inc) = self.rng.state_pair();
        s.push_str(&format!(
            "t={};c={} p={} w={} del={} eggs={} dead={} q={} ants={} starve={:.4} eat={:.4} lay={:.4} q_len={} phase={:?}({}) phase_t={:.4} team={:?}({}) ent={};rng={:016x}{:016x};dug={} nid={}",
            self.tick,
            c.carbs,
            c.protein,
            c.water,
            c.delivered,
            c.eggs_laid,
            c.dead,
            c.queen_id,
            c.ant_count,
            c.starve_t,
            c.eat_t,
            c.lay_cooldown,
            c.dig_queue.len(),
            c.phase,
            c.phase as u8,
            c.phase_t,
            c.team,
            c.team as u8,
            entrance,
            rng_state,
            rng_inc,
            self.dug_tiles,
            self.next_id,
        ));
        s.push_str(&format!(
            ";grid={:016x} {:016x} soil={:016x} {:016x}",
            fnv(&self.world.surface.tiles),
            fnv(&self.world.underground.tiles),
            fnv(&self.world.soil_surface),
            fnv(&self.world.soil_underground),
        ));
        s.push_str(&format!(
            ";known={:?}",
            c.known.iter().collect::<Vec<_>>()
        ));
        for e in self.snapshot() {
            match e {
                EntitySnap::Ant(a) => {
                    let (state, extra) = self.canonical_ant_state(a.id);
                    s.push_str(&format!(
                        "|A{} {:?} sp{:.4} L{} {:+.4},{:+.4} {}{} hp{:.4} {:?} h{:.4}{}",
                        a.id,
                        a.caste,
                        self.ant_speed_raw(a.id),
                        a.layer as u8,
                        a.x,
                        a.y,
                        state,
                        extra,
                        a.hp,
                        a.carry,
                        a.hunger,
                        self.canonical_ai(a.id),
                    ));
                }
                EntitySnap::Food(f) => {
                    s.push_str(&format!(
                        "|F{} L{} {:+.4},{:+.4} n{} {:?} {:?}{}",
                        f.id,
                        f.layer as u8,
                        f.x,
                        f.y,
                        f.amount,
                        f.kind,
                        f.role,
                        self.canonical_food(f.id),
                    ));
                }
                EntitySnap::Egg(e) => {
                    s.push_str(&format!(
                        "|E{} L{} {:+.4},{:+.4} h{:.4}/{:.4} {:?} by{:?}",
                        e.id,
                        e.layer as u8,
                        e.x,
                        e.y,
                        self.hatch_raw(e.id),
                        self.total_raw(e.id),
                        e.caste,
                        self.carrier_of(e.id),
                    ));
                }
                EntitySnap::Spider(p) => {
                    s.push_str(&format!(
                        "|S{} L{} {:+.4},{:+.4} hp{:.4}{}",
                        p.id,
                        p.layer as u8,
                        p.x,
                        p.y,
                        p.hp,
                        self.canonical_predator(p.id),
                    ));
                }
            }
        }
        s
    }

    fn ant_speed_raw(&self, id: u32) -> f64 {
        let ent = self.ids[&id];
        self.ecs.get::<&Ant>(ent).map(|a| a.speed).unwrap_or(0.0)
    }

    fn hatch_raw(&self, id: u32) -> f64 {
        let ent = self.ids[&id];
        self.ecs.get::<&Egg>(ent).map(|e| e.hatch).unwrap_or(0.0)
    }

    fn total_raw(&self, id: u32) -> f64 {
        let ent = self.ids[&id];
        self.ecs.get::<&Egg>(ent).map(|e| e.total).unwrap_or(0.0)
    }

    fn carrier_of(&self, id: u32) -> Option<u32> {
        let ent = self.ids[&id];
        self.ecs.get::<&Egg>(ent).ok().and_then(|e| e.carried_by)
    }

    /// Full AntState incl. path waypoints and dig progress — the render
    /// snapshot deliberately omits these; the digest must not.
    fn canonical_ant_state(&self, id: u32) -> (String, String) {
        let ent = self.ids[&id];
        let Ok(st) = self.ecs.get::<&AntState>(ent) else {
            return ("-".into(), String::new());
        };
        match &*st {
            AntState::Idle => ("idle".into(), String::new()),
            AntState::Moving {
                path,
                next,
                then_swap,
            } => {
                let wps: Vec<String> = path
                    .iter()
                    .map(|p| format!("({:.4},{:.4})", p.x, p.y))
                    .collect();
                (
                    "moving".into(),
                    format!(" n{} sw{} [{}]", next, then_swap, wps.join("")),
                )
            }
            AntState::Digging {
                tx,
                ty,
                progress,
                resume,
            } => (
                "digging".into(),
                format!(
                    " {tx},{ty} p{:.4} r{}",
                    progress,
                    resume.is_some()
                ),
            ),
            AntState::Fighting { target } => ("fighting".into(), format!(" t{target}")),
        }
    }

    /// WorkerAi internals: job, retries, walk-to intents — all of them steer
    /// the future, so all of them belong in the digest.
    fn canonical_ai(&self, id: u32) -> String {
        let ent = self.ids[&id];
        let Ok(ai) = self.ecs.get::<&WorkerAi>(ent) else {
            return String::new();
        };
        let a = &*ai;
        format!(
            " j{:?} r{} pen{:?} atk{:?} dig{:?} drp{:?} pick{:?} land{} fnd{:?}",
            a.job, a.retry, a.pending, a.attack_after, a.dig_after, a.drop_after, a.pick_after,
            a.land_after, a.found_after,
        )
    }

    fn canonical_food(&self, id: u32) -> String {
        let ent = self.ids[&id];
        let Ok(f) = self.ecs.get::<&Food>(ent) else {
            return String::new();
        };
        let f = &*f;
        format!(
            " h{:.4} p{:.4} st{} src{} sp{:?}",
            f.harvest_t, f.progress, f.stored as u8, f.src, f.spoil
        )
    }

    fn canonical_predator(&self, id: u32) -> String {
        let ent = self.ids[&id];
        let Ok(p) = self.ecs.get::<&Predator>(ent) else {
            return String::new();
        };
        let p = &*p;
        format!(
            " hp{:.4}/{:.4} d{} cd{} t{} home{},{} w{:.4} d{:?} t{:?}",
            p.hp, p.max_hp, p.dmg, p.atk_cd, p.atk_t, p.home.0, p.home.1, p.wander_t, p.dest,
            p.target,
        )
    }
}

/// FNV-1a over raw bytes — stable across platforms and toolchains, unlike
/// std's DefaultHasher. Folds the big tile/soil grids into the digest.
fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}
