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
pub const KIND_COLLECTIBLE: u8 = 7;
pub const KIND_HONEY: u8 = 8;
pub const KIND_MEDIC: u8 = 9;

pub const ACT_IDLE: u8 = 0;
pub const ACT_MOVING: u8 = 1;
pub const ACT_DIGGING: u8 = 2;
pub const ACT_FIGHTING: u8 = 3;
pub const ACT_FLYING: u8 = 4;
pub const ACT_HARVESTING: u8 = 5;
pub const CARRY_WOOD: u8 = 4;
pub const CARRY_WOOL: u8 = 5;

pub const CARRY_NONE: u8 = 0;
pub const CARRY_DIRT: u8 = 1;
pub const CARRY_EGG: u8 = 2;
pub const CARRY_FOOD: u8 = 3;
pub const CARRY_FALLEN: u8 = 6;

pub const FOOD_GREEN: u8 = 0;
pub const FOOD_SUPER: u8 = 1;
pub const FOOD_PROTEIN: u8 = 2;
pub const FOOD_CARB: u8 = 3;
pub const FOOD_WATER: u8 = 4;
pub const FOOD_HONEYDEW: u8 = 5;

pub fn food_code(kind: FoodKind) -> u8 {
    match kind {
        FoodKind::Green => FOOD_GREEN,
        FoodKind::Super => FOOD_SUPER,
        FoodKind::Protein => FOOD_PROTEIN,
        FoodKind::Carbs => FOOD_CARB,
        FoodKind::Water => FOOD_WATER,
        FoodKind::Honeydew => FOOD_HONEYDEW,
    }
}

/// What an egg hatches into (wire p1 on egg records; layout v4).
pub const EGG_WORKER: u8 = 0;
pub const EGG_SOLDIER: u8 = 1;
pub const EGG_HONEY: u8 = 2;
pub const EGG_MEDIC: u8 = 3;

pub fn egg_caste_code(caste: Caste) -> u8 {
    match caste {
        Caste::Worker => EGG_WORKER,
        Caste::Soldier => EGG_SOLDIER,
        Caste::Honey => EGG_HONEY,
        Caste::Medic => EGG_MEDIC,
        Caste::Queen => EGG_WORKER,
    }
}

/// Request codes for p6 (the queen's craving): 0 = none/not the queen, else
/// the food code of the craved kind.
pub const REQUEST_NONE: u8 = 0;

// --- activity indicators (admin-panel wave, wire v5) ---

/// What an ant is *about* — the intent-level job, not just the body state
/// (a feeder walking to the pantry is still feeding the queen, a squad
/// follower hauling dirt is still in the squad's service). Derived live in
/// `intent_of` from job/state/carry; never stored, never canonical.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    /// Not applicable (downed ants carry the DOWN label instead).
    None,
    /// Off-duty: loiter, rest, standby, idle drift.
    Off,
    Dig,
    /// Spoil being hauled out to the surface.
    HaulDirt,
    /// Carrying something home to the nest (food to the pantry, eggs to the
    /// nursery, collectibles to build with).
    HaulHome,
    Fight,
    /// Medic business: standby, rescue, healing.
    Medic,
    /// The designated queen-feeder, whatever leg of the loop it is on.
    Feeder,
    Follow,
    /// A Honey ant producing honeydew where it stands.
    Produce,
    /// Working a food source or pile of this kind (drives the icon tint).
    Forage(FoodKind),
}

/// Why an ant is stuck — the "!" icon's reason. Derived honestly from the
/// live situation (the AI's actual blockers), never guessed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocked {
    None,
    /// The feeder's pantry cannot satisfy the queen's craving.
    EmptyPantry,
    /// A medic's healing is parked waiting for pantry water.
    NeedsWater,
    /// A loaded carrier with no pantry cell and no room to dig more nest.
    PantryFull,
    /// The founding queen's hunger fuse is burning and nothing feeds her.
    QueenStarving,
}

pub const INTENT_NONE: u8 = 0;
pub const INTENT_OFF: u8 = 1;
pub const INTENT_DIG: u8 = 2;
pub const INTENT_HAUL_DIRT: u8 = 3;
pub const INTENT_HAUL_HOME: u8 = 4;
pub const INTENT_FIGHT: u8 = 5;
pub const INTENT_MEDIC: u8 = 6;
pub const INTENT_FEEDER: u8 = 7;
pub const INTENT_FOLLOW: u8 = 8;
pub const INTENT_PRODUCE: u8 = 9;
/// Forage intents are 20 + the food code (per-resource icon tint).
pub const INTENT_FORAGE: u8 = 20;

pub const BLOCKED_NONE: u8 = 0;
pub const BLOCKED_EMPTY_PANTRY: u8 = 1;
pub const BLOCKED_NEEDS_WATER: u8 = 2;
pub const BLOCKED_PANTRY_FULL: u8 = 3;
pub const BLOCKED_QUEEN_STARVING: u8 = 4;

pub fn intent_code(i: Intent) -> u8 {
    match i {
        Intent::None => INTENT_NONE,
        Intent::Off => INTENT_OFF,
        Intent::Dig => INTENT_DIG,
        Intent::HaulDirt => INTENT_HAUL_DIRT,
        Intent::HaulHome => INTENT_HAUL_HOME,
        Intent::Fight => INTENT_FIGHT,
        Intent::Medic => INTENT_MEDIC,
        Intent::Feeder => INTENT_FEEDER,
        Intent::Follow => INTENT_FOLLOW,
        Intent::Produce => INTENT_PRODUCE,
        Intent::Forage(k) => INTENT_FORAGE + food_code(k),
    }
}

pub fn blocked_code(b: Blocked) -> u8 {
    match b {
        Blocked::None => BLOCKED_NONE,
        Blocked::EmptyPantry => BLOCKED_EMPTY_PANTRY,
        Blocked::NeedsWater => BLOCKED_NEEDS_WATER,
        Blocked::PantryFull => BLOCKED_PANTRY_FULL,
        Blocked::QueenStarving => BLOCKED_QUEEN_STARVING,
    }
}

/// JSON description of the snapshot wire codes, for the client's boot-time
/// decoder assertion. Bump `snapshot` when the layout changes.
pub fn snapshot_spec() -> String {
    format!(
        "{{\"snapshot\":5,\"stride\":15,\"kinds\":{{\"queen\":{KIND_QUEEN},\"worker\":{KIND_WORKER},\"soldier\":{KIND_SOLDIER},\"egg\":{KIND_EGG},\"spider\":{KIND_SPIDER},\"food\":{KIND_FOOD},\"source\":{KIND_SOURCE},\"collectible\":{KIND_COLLECTIBLE},\"honey\":{KIND_HONEY},\"medic\":{KIND_MEDIC}}},\"activity\":{{\"idle\":{ACT_IDLE},\"moving\":{ACT_MOVING},\"digging\":{ACT_DIGGING},\"fighting\":{ACT_FIGHTING},\"flying\":{ACT_FLYING},\"harvesting\":{ACT_HARVESTING}}},\"carry\":{{\"none\":{CARRY_NONE},\"dirt\":{CARRY_DIRT},\"egg\":{CARRY_EGG},\"food\":{CARRY_FOOD},\"wood\":{CARRY_WOOD},\"wool\":{CARRY_WOOL},\"fallen\":{CARRY_FALLEN}}},\"food\":{{\"green\":{FOOD_GREEN},\"super\":{FOOD_SUPER},\"protein\":{FOOD_PROTEIN},\"carbs\":{FOOD_CARB},\"water\":{FOOD_WATER},\"honeydew\":{FOOD_HONEYDEW}}},\"request\":{{\"none\":{REQUEST_NONE},\"protein\":{FOOD_PROTEIN},\"carbs\":{FOOD_CARB},\"water\":{FOOD_WATER}}},\"eggCaste\":{{\"worker\":{EGG_WORKER},\"soldier\":{EGG_SOLDIER},\"honey\":{EGG_HONEY},\"medic\":{EGG_MEDIC}}},\"intent\":{{\"none\":{INTENT_NONE},\"off\":{INTENT_OFF},\"dig\":{INTENT_DIG},\"haulDirt\":{INTENT_HAUL_DIRT},\"haulHome\":{INTENT_HAUL_HOME},\"fight\":{INTENT_FIGHT},\"medic\":{INTENT_MEDIC},\"feeder\":{INTENT_FEEDER},\"follow\":{INTENT_FOLLOW},\"produce\":{INTENT_PRODUCE},\"forageGreen\":{},\"forageSuper\":{},\"forageProtein\":{},\"forageCarbs\":{},\"forageWater\":{},\"forageHoneydew\":{}}},\"blocked\":{{\"none\":{BLOCKED_NONE},\"emptyPantry\":{BLOCKED_EMPTY_PANTRY},\"needsWater\":{BLOCKED_NEEDS_WATER},\"pantryFull\":{BLOCKED_PANTRY_FULL},\"queenStarving\":{BLOCKED_QUEEN_STARVING}}}}}",
        INTENT_FORAGE + FOOD_GREEN,
        INTENT_FORAGE + FOOD_SUPER,
        INTENT_FORAGE + FOOD_PROTEIN,
        INTENT_FORAGE + FOOD_CARB,
        INTENT_FORAGE + FOOD_WATER,
        INTENT_FORAGE + FOOD_HONEYDEW,
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
    /// Working a food source or picking up a loose pile.
    Harvesting,
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
    /// Starvation progress 0..=1 (queen only; 0 for everyone else). In
    /// founding mode this is the physical-hunger readout: 0 while satisfied,
    /// then 0→1 from the moment she turns hungry to her death.
    pub hunger: f64,
    /// What the queen requests next (founding mode, queen only): the craved
    /// resource the feeder should bring.
    pub request: Option<FoodKind>,
    /// Squad leader this ant follows, if any (X-menu, F3).
    pub following: Option<u32>,
    /// Downed (F4): remaining bleed fraction 1→0; None = standing.
    pub downed: Option<f64>,
    /// Intent-level activity for the far-zoom icon layer (wire v5).
    pub intent: Intent,
    /// Why this ant is stuck (the "!" icon's reason), if it is.
    pub blocked: Blocked,
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
    Collectible(CollectibleSnap),
}

impl EntitySnap {
    pub fn id(&self) -> u32 {
        match self {
            EntitySnap::Ant(e) => e.id,
            EntitySnap::Food(e) => e.id,
            EntitySnap::Egg(e) => e.id,
            EntitySnap::Spider(e) => e.id,
            EntitySnap::Collectible(e) => e.id,
        }
    }

    pub fn layer(&self) -> Layer {
        match self {
            EntitySnap::Ant(e) => e.layer,
            EntitySnap::Food(e) => e.layer,
            EntitySnap::Egg(e) => e.layer,
            EntitySnap::Spider(e) => e.layer,
            EntitySnap::Collectible(e) => e.layer,
        }
    }

    pub fn pos(&self) -> (f64, f64) {
        match self {
            EntitySnap::Ant(e) => (e.x, e.y),
            EntitySnap::Food(e) => (e.x, e.y),
            EntitySnap::Egg(e) => (e.x, e.y),
            EntitySnap::Spider(e) => (e.x, e.y),
            EntitySnap::Collectible(e) => (e.x, e.y),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollectibleSnap {
    pub id: u32,
    pub layer: crate::components::Layer,
    pub x: f64,
    pub y: f64,
    /// 0 = wet wood (food-storage block), 1 = dry wool (egg-friendly block).
    pub variant: u8,
}

pub(crate) fn source_name(src: u8) -> &'static str {
    match src {
        1 => "moss",
        2 => "mushroom",
        3 => "raspberry",
        4 => "strawberry",
        5 => "cockroach",
        6 => "caterpillar",
        7 => "nettle",
        _ => "source",
    }
}

pub(crate) fn food_name(kind: FoodKind) -> &'static str {
    match kind {
        FoodKind::Green => "green",
        FoodKind::Super => "super",
        FoodKind::Protein => "protein",
        FoodKind::Carbs => "carbs",
        FoodKind::Water => "water",
        FoodKind::Honeydew => "honeydew",
    }
}

/// The WorkerAi fields the intent projection needs, copied out of the ECS
/// borrow (the Ref guard can't leave the query scope).
struct AiView {
    job: Job,
    attack_after: bool,
    dig_after: bool,
}

impl Sim {
    /// Intent + blocked derivation for the icon layer: a pure, honest
    /// projection of (job, body state, mandibles, colony context). Order:
    /// what the body is doing right now beats the job; the job beats the
    /// idle defaults. Blocked is only ever reported where the AI genuinely
    /// cannot proceed — never a guess.
    fn intent_of(
        &self,
        caste: Caste,
        state: &AntState,
        carry: Carry,
        ai: Option<&AiView>,
        downed: bool,
    ) -> (Intent, Blocked) {
        if downed {
            // the red DOWN label is the downed ant's whole story
            return (Intent::None, Blocked::None);
        }
        let job = ai.map(|a| &a.job);
        // the blocked checks all index through the queen/pantry (queen_tile
        // panics on a despawned queen) — a dead colony has no economy left
        let blocked = if self.colony.dead || self.world.entrance.is_none() {
            Blocked::None
        } else {
            self.blocked_of(caste, carry, job)
        };
        // body states first — they are what is visibly happening
        match state {
            AntState::Digging { .. } => return (Intent::Dig, blocked),
            AntState::Fighting { .. } => return (Intent::Fight, blocked),
            AntState::Harvesting { target } => {
                return (
                    self.food_kind_of(*target)
                        .map_or(Intent::Off, Intent::Forage),
                    blocked,
                )
            }
            _ => {}
        }
        if let Some(ai) = ai {
            if ai.attack_after {
                return (Intent::Fight, blocked);
            }
            if ai.dig_after {
                return (Intent::Dig, blocked);
            }
        }
        // role jobs beat the generic carry read: a feeder with the craved
        // unit in its mandibles is walking it to the QUEEN, not to the
        // pantry — the crown must not flicker to a haul icon mid-loop
        match job {
            Some(Job::Feed) => return (Intent::Feeder, blocked),
            Some(Job::Rescue(_)) | Some(Job::Heal(_)) => return (Intent::Medic, blocked),
            _ => {}
        }
        match carry {
            Carry::Dirt { .. } => return (Intent::HaulDirt, blocked),
            // a medic's patient is handled by the job below; anyone else
            // hauling a fallen ant is doing medic work
            Carry::Fallen => return (Intent::Medic, blocked),
            // food banking on silver, eggs to the nursery, wood/wool home
            Carry::Food(_) | Carry::Egg | Carry::Wood | Carry::Wool => {
                return (Intent::HaulHome, blocked)
            }
            Carry::None => {}
        }
        match job {
            Some(Job::Hold) => (Intent::Medic, blocked),
            Some(Job::Follow(_, _)) => (Intent::Follow, blocked),
            Some(Job::DigTile(_, _)) => (Intent::Dig, blocked),
            Some(Job::Deliver(_, _)) => (Intent::HaulHome, blocked),
            Some(Job::Fetch(fid)) => (
                self.food_kind_of(*fid).map_or(Intent::Off, Intent::Forage),
                blocked,
            ),
            // a Honey ant produces wherever it stands — even while loitering
            // between pantry cells (the secretion timer always runs)
            _ if caste == Caste::Honey => (Intent::Produce, blocked),
            Some(Job::Loiter(_)) | Some(Job::GoHome) | Some(Job::GoOut) | Some(Job::Rest(_)) => {
                (Intent::Off, blocked)
            }
            _ => (Intent::Off, blocked),
        }
    }

    /// The stuck-reason half of the icon layer. Every case mirrors an actual
    /// dead-end branch in the AI (see worker_ai) so the "!" never lies.
    fn blocked_of(&self, caste: Caste, carry: Carry, job: Option<&Job>) -> Blocked {
        // feeder with empty hands and a pantry that cannot satisfy the
        // craving (worker_ai's Feed → craving_pile() None branch)
        if matches!(job, Some(Job::Feed))
            && !matches!(carry, Carry::Food(_))
            && self.craving_pile().is_none()
        {
            return Blocked::EmptyPantry;
        }
        // medic parked at its patient because the pantry can't pay the water
        // (worker_ai's Heal phase 2 → withdraw fails branch)
        if let Some(Job::Heal(fid)) = job {
            let healing = self
                .ids
                .get(fid)
                .and_then(|&e| self.ecs.get::<&Fallen>(e).ok())
                .and_then(|f| f.heal_t)
                .is_some();
            if !matches!(carry, Carry::Fallen)
                && !healing
                && self.pantry_units(FoodKind::Water) < self.rules.water_per_heal
            {
                return Blocked::NeedsWater;
            }
        }
        // loaded carrier: no pantry cell with room and nowhere soft left to
        // dig within the expansion radius (worker_ai's pantry-full branch)
        if matches!(carry, Carry::Food(_))
            && matches!(job, Some(Job::Idle) | Some(Job::Deliver(_, _)) | None)
            && self.pantry_tile().is_none()
            && self.pick_dig_target().is_none()
        {
            return Blocked::PantryFull;
        }
        // founding queen: hunger fuse burning, pantry can't cover the
        // craving, and she isn't holding the craved unit herself
        if caste == Caste::Queen
            && self.colony.founding
            && self.colony.hunger_t >= self.rules.eat_period
            && self.craving_pile().is_none()
            && !matches!(carry, Carry::Food(k) if k == self.craving())
        {
            return Blocked::QueenStarving;
        }
        Blocked::None
    }

    fn food_kind_of(&self, fid: u32) -> Option<FoodKind> {
        let &ent = self.ids.get(&fid)?;
        self.ecs.get::<&Food>(ent).ok().map(|f| f.kind)
    }

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
            let activity = if ant.caste == Caste::Queen && self.colony.phase == super::Phase::Flight
            {
                Activity::Flying
            } else {
                match state {
                    AntState::Idle => Activity::Idle,
                    AntState::Moving { .. } => Activity::Moving,
                    AntState::Digging { .. } => Activity::Digging,
                    AntState::Fighting { .. } => Activity::Fighting,
                    AntState::Harvesting { .. } => Activity::Harvesting,
                }
            };
            let following = self
                .ecs
                .get::<&WorkerAi>(ent)
                .ok()
                .and_then(|ai| match &ai.job {
                    Job::Follow(leader, _) => Some(*leader),
                    _ => None,
                });
            let is_founding_queen = ant.caste == Caste::Queen
                && self.colony.founding
                && !self.colony.dead
                // the request matters once there is a pantry to feed her from
                && self.world.entrance.is_some();
            let request = is_founding_queen.then(|| {
                self.rules.craving_cycle[self
                    .colony
                    .craving_i
                    .min(self.rules.craving_cycle.len() - 1)]
            });
            let hunger = if ant.caste == Caste::Queen {
                if self.colony.founding {
                    // physical model: 0 until hungry, then hungry→death
                    ((self.colony.hunger_t - self.rules.eat_period) / self.rules.starve_time)
                        .clamp(0.0, 1.0)
                } else {
                    self.colony.starve_t / self.rules.starve_time
                }
            } else {
                0.0
            };
            let downed = self
                .ecs
                .get::<&Fallen>(ent)
                .ok()
                .map(|f| (f.bleed_t / self.rules.bleed_time).clamp(0.0, 1.0));
            let (intent, blocked) = self.intent_of(
                ant.caste,
                state,
                *carry,
                self.ecs
                    .get::<&WorkerAi>(ent)
                    .ok()
                    .map(|q| AiView {
                        job: q.job.clone(),
                        attack_after: q.attack_after.is_some(),
                        dig_after: q.dig_after.is_some(),
                    })
                    .as_ref(),
                downed.is_some(),
            );
            v.push(EntitySnap::Ant(AntSnap {
                id,
                caste: ant.caste,
                layer: pos.layer,
                x: pos.p.x,
                y: pos.p.y,
                activity,
                hp: (combat.hp / combat.max_hp).clamp(0.0, 1.0),
                carry: *carry,
                hunger,
                request,
                following,
                downed,
                intent,
                blocked,
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
                        .map(|t| (t / self.rules.spoil_time).clamp(0.0, 1.0)),
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
        for (ent, (coll, pos)) in self.ecs.query::<(&Collectible, &Pos)>().iter() {
            let Some(&id) = rev.get(&ent) else { continue };
            v.push(EntitySnap::Collectible(CollectibleSnap {
                id,
                layer: pos.layer,
                x: pos.p.x,
                y: pos.p.y,
                variant: match coll.variant {
                    CollectibleVariant::Wood => 0,
                    CollectibleVariant::Wool => 1,
                },
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
            "t={};c={} p={} w={} del={} eggs={} dead={} q={} ants={} starve={:.4} eat={:.4} lay={:.4} q_len={} phase={:?}({}) phase_t={:.4} team={:?}({}) ent={} cr={:?} hu={:.4} fd={:?};rng={:016x}{:016x};dug={} nid={} rules={:016x}",
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
            self.rules.craving_cycle
                [c.craving_i.min(self.rules.craving_cycle.len() - 1)],
            c.hunger_t,
            c.feeder_id,
            rng_state,
            rng_inc,
            self.dug_tiles,
            self.next_id,
            self.rules.digest(),
        ));
        s.push_str(&format!(
            ";grid={:016x} {:016x} soil={:016x} {:016x}",
            fnv(&self.world.surface.tiles),
            fnv(&self.world.underground.tiles),
            fnv(&self.world.soil_surface),
            fnv(&self.world.soil_underground),
        ));
        s.push_str(&format!(";known={:?}", c.known.iter().collect::<Vec<_>>()));
        for e in self.snapshot() {
            match e {
                EntitySnap::Ant(a) => {
                    let (state, extra) = self.canonical_ant_state(a.id);
                    s.push_str(&format!(
                        "|A{} {:?} sp{:.4} g{:.4} L{} {:+.4},{:+.4} {}{} hp{:.4} {:?} h{:.4}{}{}",
                        a.id,
                        a.caste,
                        self.ant_speed_raw(a.id),
                        self.ant_gen_raw(a.id),
                        a.layer as u8,
                        a.x,
                        a.y,
                        state,
                        extra,
                        a.hp,
                        a.carry,
                        a.hunger,
                        self.canonical_ai(a.id),
                        self.canonical_fallen(a.id),
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
                EntitySnap::Collectible(c) => {
                    s.push_str(&format!(
                        "|K{} L{} {:+.4},{:+.4} v{}",
                        c.id, c.layer as u8, c.x, c.y, c.variant
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

    fn ant_gen_raw(&self, id: u32) -> f64 {
        let ent = self.ids[&id];
        self.ecs.get::<&Ant>(ent).map(|a| a.gen_t).unwrap_or(0.0)
    }

    /// Downed-ant state for the digest: bleed/heal timers and carrier —
    /// all of them steer the future.
    fn canonical_fallen(&self, id: u32) -> String {
        let ent = self.ids[&id];
        match self.ecs.get::<&Fallen>(ent) {
            Ok(f) => format!(" fl{:.4} h{:?} by{:?}", f.bleed_t, f.heal_t, f.carried_by),
            Err(_) => String::new(),
        }
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
            AntState::Harvesting { target } => ("harv".into(), format!("#{target}")),
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
                format!(" {tx},{ty} p{:.4} r{}", progress, resume.is_some()),
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
            a.job,
            a.retry,
            a.pending,
            a.attack_after,
            a.dig_after,
            a.drop_after,
            a.pick_after,
            a.land_after,
            a.found_after,
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
            p.hp,
            p.max_hp,
            p.dmg,
            p.atk_cd,
            p.atk_t,
            p.home.0,
            p.home.1,
            p.wander_t,
            p.dest,
            p.target,
        )
    }
}

/// FNV-1a over raw bytes — stable across platforms and toolchains, unlike
/// std's DefaultHasher. Folds the big tile/soil grids into the digest.
pub(crate) fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}
