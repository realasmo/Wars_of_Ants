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
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Job {
    Manual,
    Idle,
    Fetch(u32),
    Deliver,
}

#[derive(Clone, Debug)]
pub struct WorkerAi {
    pub job: Job,
    pub retry: u32,
    pub pending: Option<(Layer, (u32, u32))>,
}

#[derive(Clone, Copy, Debug)]
pub struct Carrying {
    pub food: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Food {
    pub amount: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Egg {
    pub hatch: f64,
    pub total: f64,
}
