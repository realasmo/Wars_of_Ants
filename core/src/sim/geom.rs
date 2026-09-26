//! Grid, 2×2-block and soil geometry helpers shared by commands, AI and
//! systems.

use super::Sim;
use crate::components::Layer;
use crate::world::{Grid, EMPTY, ROCK};

impl Sim {
    pub(crate) fn grid_of(&self, layer: Layer) -> &Grid {
        match layer {
            Layer::Surface => &self.world.surface,
            Layer::Underground => &self.world.underground,
        }
    }

    /// A copy of the underground grid for worker pathing: soft tiles inside a
    /// not-fully-soft block count as rock — blocks are dug all-or-nothing.
    pub(crate) fn worker_grid(&self) -> Grid {
        let g = &self.world.underground;
        let mut tiles = g.tiles.clone();
        for by in (0..g.h - 1).step_by(2) {
            for bx in (0..g.w - 1).step_by(2) {
                let full = (0..2u32).all(|dy| {
                    (0..2u32).all(|dx| Grid::is_soft(g.get(bx + dx, by + dy)))
                });
                if !full {
                    for dy in 0..2u32 {
                        for dx in 0..2u32 {
                            let i = ((by + dy) * g.w + bx + dx) as usize;
                            if Grid::is_soft(tiles[i]) {
                                tiles[i] = ROCK;
                            }
                        }
                    }
                }
            }
        }
        Grid {
            w: g.w,
            h: g.h,
            tiles,
        }
    }

    pub(crate) fn block_in_bounds(&self, bx: u32, by: u32) -> bool {
        bx + 1 < self.config.width && by + 1 < self.config.height
    }

    pub(crate) fn block_soft(&self, bx: u32, by: u32) -> bool {
        self.block_in_bounds(bx, by)
            && (0..2u32).all(|dy| {
                (0..2u32).all(|dx| Grid::is_soft(self.world.underground.get(bx + dx, by + dy)))
            })
    }

    pub(crate) fn block_empty(&self, bx: u32, by: u32) -> bool {
        self.block_in_bounds(bx, by)
            && (0..2u32).all(|dy| {
                (0..2u32).all(|dx| self.world.underground.get(bx + dx, by + dy) == EMPTY)
            })
    }

    /// Chebyshev distance from the ant's tile to the 2×2 block, tile-granular.
    /// Diagonal-only contact is NOT enough: like pathfinding's no-corner-cut
    /// rule, one of the two tiles flanking the corner must be open.
    pub(crate) fn block_adjacent(&self, ant: u32, bx: u32, by: u32) -> bool {
        let (tx, ty) = self.ant_tile(ant);
        let dx = if tx < bx {
            (bx - tx) as i32
        } else if tx > bx + 1 {
            (tx - bx - 1) as i32
        } else {
            0
        };
        let dy = if ty < by {
            (by - ty) as i32
        } else if ty > by + 1 {
            (ty - by - 1) as i32
        } else {
            0
        };
        if dx.max(dy) > 1 {
            return false;
        }
        if dx > 0 && dy > 0 {
            // corner-to-corner: the block column/row nearest the ant plus the
            // ant's own row/column form the two flanking tiles
            let fx = if tx < bx { bx } else { bx + 1 };
            let fy = if ty < by { by } else { by + 1 };
            let open_x = self.world.underground.get(fx, ty) == EMPTY;
            let open_y = self.world.underground.get(tx, fy) == EMPTY;
            return open_x || open_y;
        }
        true
    }

    pub(crate) fn soil_at(&self, layer: Layer, x: u32, y: u32) -> u8 {
        let v = match layer {
            Layer::Surface => &self.world.soil_surface,
            Layer::Underground => &self.world.soil_underground,
        };
        v[(y * self.config.width + x) as usize]
    }

    pub(crate) fn set_soil_block(&mut self, layer: Layer, bx: u32, by: u32, soil: u8) {
        let w = self.config.width;
        let v = match layer {
            Layer::Surface => &mut self.world.soil_surface,
            Layer::Underground => &mut self.world.soil_underground,
        };
        for dy in 0..2 {
            for dx in 0..2 {
                v[((by + dy) * w + bx + dx) as usize] = soil;
            }
        }
    }

    /// All tiles on the chebyshev ring of radius `r` around `c`, clamped away
    /// from the rock border. Deterministic scan order (row-major).
    pub(crate) fn ring_tiles(&self, c: (u32, u32), r: u32) -> Vec<(u32, u32)> {
        if r == 0 {
            return vec![c];
        }
        let mut v = Vec::new();
        for dy in -(r as i32)..=(r as i32) {
            for dx in -(r as i32)..=(r as i32) {
                if dx.abs() != r as i32 && dy.abs() != r as i32 {
                    continue;
                }
                let x = c.0 as i32 + dx;
                let y = c.1 as i32 + dy;
                if x < 1 || y < 1 || x >= self.config.width as i32 - 1 || y >= self.config.height as i32 - 1 {
                    continue;
                }
                v.push((x as u32, y as u32));
            }
        }
        v
    }

}
