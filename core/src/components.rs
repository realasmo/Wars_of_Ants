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
    Green = 0,
    /// Legacy founded worlds only (spider drops there).
    Super = 1,
    /// Carried-resource flags (not world entities): dirt = 2, egg = 3 kept
    /// for snapshot-code stability.
    Dirt = 2,
    Egg = 3,
    /// Resource economy: units and sources are one of these three.
    Protein = 4,
    Carbs = 5,
    Water = 6,
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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Job {
    Manual,
    Idle,
    Fetch(u32),
    /// Carry food to the pantry cell (x, y).
    Deliver(u32, u32),
    /// Wander toward (x, y) looking for undiscovered sources.
    Scout(u32, u32),
    DigTile(u32, u32),
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
pub struct Carrying {
    pub amount: u32,
    pub kind: FoodKind,
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
