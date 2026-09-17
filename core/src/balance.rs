//! Balance table: every tunable gameplay number lives here.
//! Data only — no logic — so balancing never touches sim code.
//! When a number starts being tuned weekly, this file graduates to a data file.

use crate::components::Caste;

/// Combat/movement stats per unit type. Times are in seconds, distances in tiles.
pub struct UnitStats {
    pub hp: f64,
    pub dmg: f64,
    pub atk_cd: f64,
    pub speed: f64,
    pub range: f64,
}

pub const QUEEN: UnitStats = UnitStats {
    hp: 150.0,
    dmg: 0.0,
    atk_cd: 1.0,
    speed: 1.0,
    range: ANT_RANGE,
};

pub const WORKER: UnitStats = UnitStats {
    hp: 100.0,
    dmg: 8.0,
    atk_cd: 1.0,
    speed: 3.0,
    range: ANT_RANGE,
};

pub const SOLDIER: UnitStats = UnitStats {
    hp: 130.0,
    dmg: 22.0,
    atk_cd: 1.0,
    speed: 2.6,
    range: ANT_RANGE,
};

pub const SPIDER: UnitStats = UnitStats {
    hp: 130.0,
    dmg: 15.0,
    atk_cd: 1.2,
    speed: 2.2,
    range: 0.8,
};

/// Melee reach shared by all ant castes (kept uniform for now).
pub const ANT_RANGE: f64 = 0.9;

pub fn stats_for(caste: Caste) -> &'static UnitStats {
    match caste {
        Caste::Queen => &QUEEN,
        Caste::Worker => &WORKER,
        Caste::Soldier => &SOLDIER,
    }
}

// --- economy / world constants ---

pub const DIG_TIME: f64 = 1.2;
pub const EAT_PERIOD: f64 = 25.0;
pub const EGG_COST: u32 = 5;
pub const EGG_TIME: f64 = 45.0;
pub const LAY_COOLDOWN: f64 = 3.0;
pub const STARVE_TIME: f64 = 90.0;
pub const START_FOOD: u32 = 5;
pub const PILE_AMOUNT: u32 = 45;

pub const SPIDER_AGGRO: f64 = 5.0;
pub const SPIDER_WANDER: f64 = 8.0;
pub const SUPER_PER_SPIDER: u32 = 8;
pub const SOLDIER_COST_GREEN: u32 = 3;
pub const SOLDIER_COST_SUPER: u32 = 2;
