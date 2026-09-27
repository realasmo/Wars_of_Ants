use crate::math::Vec2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layer {
    Surface = 0,
    Underground = 1,
}

impl Layer {
    pub fn other(self) -> Layer {
        match self {
            Layer::Surface => Layer::Underground,
            Layer::Underground => Layer::Surface,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Caste {
    Queen,
    Worker,
    Soldier,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FoodKind {
    /// Legacy founded worlds only (test economy).
    Green,
    /// Legacy founded worlds only (spider drops there).
    Super,
    /// Resource economy: units and sources are one of these three.
    Protein,
    Carbs,
    Water,
}

/// What an ant carries. A sum type instead of `{amount, kind}` so an empty
/// hand can never carry a stale kind — the phantom-dot / dead-branch class
/// of client bugs came exactly from there.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Carry {
    None,
    /// Excavated dirt blocks (the founding queen hauls up to DIRT_CAPACITY).
    Dirt { blocks: u32 },
    /// A brood egg; the egg entity itself rides along (`Egg::carried_by`).
    Egg,
    /// One unit of food.
    Food(FoodKind),
    /// Wet wood — place in the nest to build one food-storage (silver) block.
    Wood,
    /// Dry wool — place in the nest to build one egg-friendly (orange) block.
    Wool,
}

/// Surface collectibles hauled home for nest building (user TODO F2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectibleVariant {
    Wood,
    Wool,
}

/// A map collectible lying on the ground until an ant with free mandibles
/// picks it up by standing on it.
#[derive(Clone, Copy, Debug)]
pub struct Collectible {
    pub variant: CollectibleVariant,
}

#[derive(Clone, Debug)]
pub struct Ant {
    pub caste: Caste,
    pub speed: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Pos {
    pub p: Vec2,
    pub layer: Layer,
}

#[derive(Clone, Copy, Debug)]
pub struct Combat {
    pub hp: f64,
    pub max_hp: f64,
    pub dmg: f64,
    pub atk_cd: f64,
    pub atk_t: f64,
}

pub type MovePlan = (Vec<Vec2>, usize, bool);

#[derive(Clone, Debug, PartialEq)]
pub enum AntState {
    Idle,
    Moving {
        /// String-pulled world-space waypoints; path[0] is the ant's position
        /// at route time, the last is the goal (tile center).
        path: Vec<Vec2>,
        next: usize,
        then_swap: bool,
    },
    Digging {
        tx: u32,
        ty: u32,
        progress: f64,
        resume: Option<Box<MovePlan>>,
    },
    Fighting {
        target: u32,
    },
    /// Standing on a food source/loose pile, working it (the progress bar
    /// is shared and lives on the Food entity). Completing a unit fills
    /// the mandibles and returns the ant to Idle.
    Harvesting {
        target: u32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Job {
    Manual,
    Idle,
    Fetch(u32),
    /// Carry food to the pantry cell (x, y).
    Deliver(u32, u32),
    /// The designated feeder: withdraw the queen's craved resource from the
    /// pantry and hand it to her when she turns hungry.
    Feed,
    /// Wander a few hops near where the ant idled, then head home and rest.
    Loiter(u32),
    /// Walk back into the nest (underground, near the queen).
    GoHome,
    /// Walk out for a glance around the nest mouth (keeps near-nest sources
    /// discoverable without auto-scouting).
    GoOut,
    /// Pause in the nest between loiter cycles; ticks down, then GoOut.
    Rest(u32),
    DigTile(u32, u32),
    /// Squad follow: keep near the leader ant (X-menu, F3). `resume` is the
    /// job the follower returns to on release/leader death — busy ants
    /// (farming, feeding, manual control) resume their unfinished activity.
    /// A leader who starts farming converts followers to Fetch; a leader's
    /// attack order is shared as `attack_after` — they fight that enemy and
    /// return to following when it dies.
    Follow(u32, Option<Box<Job>>),
}

#[derive(Clone, Debug)]
pub struct WorkerAi {
    pub job: Job,
    pub retry: u32,
    pub pending: Option<(Layer, (u32, u32))>,
    pub attack_after: Option<u32>,
    /// Walk-to-dig intent (2×2 block origin): route to the block, dig on
    /// arrival — right-clicking distant dirt must send the ant there.
    pub dig_after: Option<(u32, u32)>,
    /// Walk-to-drop intent (target tile): route there, drop the carried
    /// item on arrival.
    pub drop_after: Option<(u32, u32)>,
    /// Walk-to-pick intent (egg id): route to the egg, pick it up there.
    pub pick_after: Option<u32>,
    /// Flying queen: land once the flight to the clicked spot completes.
    pub land_after: bool,
    /// Grounded queen: found the nest at this tile once the walk completes.
    pub found_after: Option<(u32, u32)>,
}

#[derive(Clone, Copy, Debug)]
pub struct Food {
    pub amount: u32,
    pub kind: FoodKind,
    /// True for food placed in the nest pantry by ants: visible and safe, but
    /// not a forage target.
    pub stored: bool,
    /// Remaining spoil seconds — None = never spoils (sources, pantry food);
    /// Some(t) ticks down while the food sits on a non-silver cell.
    pub spoil: Option<f64>,
    /// Seconds to harvest one unit; 0 = loose unit (instant pickup).
    pub harvest_t: f64,
    /// Shared harvest progress toward the next unit.
    pub progress: f64,
    /// Source visual type: 0 = not a source, 1..6 = moss, mushroom,
    /// raspberry, strawberry, cockroach, caterpillar.
    pub src: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct Egg {
    pub hatch: f64,
    pub total: f64,
    pub caste: Caste,
    /// Carrying ant, if any: position follows the carrier and hatching is
    /// suspended until placed on an orange cell.
    pub carried_by: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
pub struct Predator {
    pub hp: f64,
    pub max_hp: f64,
    pub dmg: f64,
    pub speed: f64,
    pub atk_cd: f64,
    pub atk_t: f64,
    pub home: (u32, u32),
    pub wander_t: f64,
    pub dest: Option<(f64, f64)>,
    pub target: Option<u32>,
}
