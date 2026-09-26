use crate::math::Vec2;

pub const EMPTY: u8 = 0;
pub const DIRT: u8 = 1;
pub const MOIST: u8 = 2;
pub const DRY: u8 = 3;
pub const ROCK: u8 = 4;

/// Cell soil quality, orthogonal to tile kind: orange soil is the only place
/// eggs transform into ants; silver soil is the only place food never spoils.
/// Hidden while covered by dirt — digging reveals it.
pub const SOIL_NONE: u8 = 0;
pub const SOIL_ORANGE: u8 = 1;
pub const SOIL_SILVER: u8 = 2;

/// Origin of the aligned 2×2 dig block containing tile (x, y). Digging,
/// dirt-filling and soil seeding all work on these blocks.
pub fn block_of(x: u32, y: u32) -> (u32, u32) {
    (x & !1, y & !1)
}

pub struct Grid {
    pub w: u32,
    pub h: u32,
    pub tiles: Vec<u8>,
}

impl Grid {
    pub fn filled(w: u32, h: u32, kind: u8) -> Self {
        Grid {
            w,
            h,
            tiles: vec![kind; (w * h) as usize],
        }
    }

    pub fn get(&self, x: u32, y: u32) -> u8 {
        self.tiles[(y * self.w + x) as usize]
    }

    pub fn set(&mut self, x: u32, y: u32, kind: u8) {
        self.tiles[(y * self.w + x) as usize] = kind;
    }

    pub fn in_bounds(&self, x: u32, y: u32) -> bool {
        x < self.w && y < self.h
    }

    pub fn is_soft(kind: u8) -> bool {
        kind == DIRT || kind == MOIST || kind == DRY
    }

    pub fn pass_cost(kind: u8, soft_passable: bool) -> Option<u32> {
        // Scale: straight step on empty terrain = 1000; diagonals are
        // cost * 1414 / 1000 in find_path (integer-exact for these values).
        match kind {
            EMPTY => Some(1000),
            DIRT | MOIST | DRY if soft_passable => Some(10000),
            _ => None,
        }
    }
}

pub fn tile_center(x: u32, y: u32) -> Vec2 {
    Vec2::new(x as f64 + 0.5, y as f64 + 0.5)
}

pub struct World {
    pub surface: Grid,
    pub underground: Grid,
    /// Nest hole (2×2 block origin, same tiles on both layers). None until
    /// the founding queen creates the nest; cross-layer routing refuses
    /// while unset.
    pub entrance: Option<(u32, u32)>,
    /// Per-tile soil quality, parallel to each grid. Surface soil comes from
    /// the dust patches (indicator + founding bonus); underground soil is
    /// seeded per block and revealed by digging.
    pub soil_surface: Vec<u8>,
    pub soil_underground: Vec<u8>,
}
