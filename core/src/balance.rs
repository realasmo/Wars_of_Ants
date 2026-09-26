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
    speed: 2.5,
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

// --- founding (game start) constants ---

/// Flight speed of the founding queen over the surface (tiles/s).
pub const QUEEN_FLY_SPEED: f64 = 5.0;
/// Seconds the founding queen has to excavate before laying the first brood.
pub const FOUNDING_TIME: f64 = 60.0;
/// Eggs laid when the founding timer ends (up to free-tile availability).
pub const FOUNDING_EGGS: u32 = 4;
/// Seconds until founding eggs hatch into the first workers.
pub const FOUNDING_EGG_HATCH: f64 = 180.0;
/// Seconds per dirt cell dug by the founding queen.
pub const QUEEN_DIG_TIME: f64 = 1.0;
/// Dirt blocks an ant may carry before having to dump (dig two, haul once).
pub const DIRT_CAPACITY: u32 = 2;

// --- resource economy (finite map sources, see docs/WORLD-DESIGN.md) ---

pub struct SourceSpec {
    /// Client visual type 1..6.
    pub src: u8,
    pub kind: crate::components::FoodKind,
    /// (min, max) units per source.
    pub amount: (u32, u32),
    /// Seconds to harvest one unit.
    pub harvest: f64,
    /// How many on the map.
    pub count: u32,
}

pub const SOURCES: [SourceSpec; 6] = [
    SourceSpec { src: 1, kind: crate::components::FoodKind::Water,   amount: (15, 20),  harvest: 10.0, count: 10 },
    SourceSpec { src: 2, kind: crate::components::FoodKind::Water,   amount: (21, 26),  harvest: 20.0, count: 7 },
    SourceSpec { src: 3, kind: crate::components::FoodKind::Carbs,   amount: (100, 110), harvest: 6.0, count: 6 },
    SourceSpec { src: 4, kind: crate::components::FoodKind::Carbs,   amount: (40, 60),  harvest: 4.0,  count: 8 },
    SourceSpec { src: 5, kind: crate::components::FoodKind::Protein, amount: (8, 12),   harvest: 15.0, count: 6 },
    SourceSpec { src: 6, kind: crate::components::FoodKind::Protein, amount: (25, 35),  harvest: 13.0, count: 5 },
];

/// Source sighting distance (chebyshev tiles).
pub const SIGHT_RANGE: u32 = 8;
/// Carb sources placed near the founding center so the first workers can
/// survive; the rest scatter wide.
pub const SOURCES_NEAR_NEST: u32 = 2;
pub const SOURCE_MIN_GAP: u32 = 8;
/// Below this many carbs the whole colony slows down (recovers when fed).
pub const CARB_LOW: u32 = 3;
pub const CARB_SLOWDOWN: f64 = 0.6;
/// Protein dropped by a killed spider.
pub const PROTEIN_PER_SPIDER: u32 = 8;

// --- soil / food logistics ---

/// Chance per 2×2 underground block of being orange (nursery) or silver
/// (pantry) soil, hidden until dug.
pub const ORANGE_SOIL_CHANCE: f64 = 0.035;
pub const SILVER_SOIL_CHANCE: f64 = 0.035;
/// Surface dust patches of each color (indicator + founding bonus source).
pub const PATCHES_PER_COLOR: u32 = 2;
pub const PATCH_W: u32 = 10;
pub const PATCH_H: u32 = 12;
pub const PATCH_WOBBLE: u32 = 2;
/// Founding inside a patch grants this many hidden soil blocks nearby.
pub const PATCH_GRANT_MIN: u32 = 3;
pub const PATCH_GRANT_MAX: u32 = 6;
/// Max food units (green + super combined) per cell.
pub const FOOD_CELL_CAP: u32 = 6;
/// Seconds before dropped food spoils on a non-silver cell.
pub const SPOIL_TIME: f64 = 300.0;

// --- economy / world constants ---

pub const DIG_TIME: f64 = 1.2;
pub const EAT_PERIOD: f64 = 25.0;
pub const EGG_COST: u32 = 5;
pub const EGG_TIME: f64 = 45.0;
pub const LAY_COOLDOWN: f64 = 3.0;
pub const STARVE_TIME: f64 = 90.0;
pub const START_FOOD: u32 = 5; // starting carbs
pub const PILE_AMOUNT: u32 = 45;

pub const SPIDER_AGGRO: f64 = 5.0;
pub const SPIDER_WANDER: f64 = 8.0;
pub const SUPER_PER_SPIDER: u32 = 8; // legacy worlds
pub const SOLDIER_COST_GREEN: u32 = 3;
pub const SOLDIER_COST_SUPER: u32 = 2;
