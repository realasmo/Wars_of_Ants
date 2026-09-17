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
    Green = 0,
    Super = 1,
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

pub type MovePlan = (Vec<(u32, u32)>, usize, bool);

#[derive(Clone, Debug, PartialEq)]
pub enum AntState {
    Idle,
    Moving {
        path: Vec<(u32, u32)>,
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
    Deliver,
    DigTile(u32, u32),
}

#[derive(Clone, Debug)]
pub struct WorkerAi {
    pub job: Job,
    pub retry: u32,
    pub pending: Option<(Layer, (u32, u32))>,
    pub attack_after: Option<u32>,
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
}

#[derive(Clone, Copy, Debug)]
pub struct Egg {
    pub hatch: f64,
    pub total: f64,
    pub caste: Caste,
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
