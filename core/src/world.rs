use crate::math::Vec2;

pub const EMPTY: u8 = 0;
pub const DIRT: u8 = 1;
pub const MOIST: u8 = 2;
pub const DRY: u8 = 3;
pub const ROCK: u8 = 4;

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
    pub entrance: (u32, u32),
}
