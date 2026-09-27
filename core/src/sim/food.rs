//! Food logistics: piles, finite sources, dropped units, the pantry, harvest
//! progress and spoiling.

use super::Sim;
use crate::balance::{FOOD_CELL_CAP, SourceSpec};
use crate::components::{Carry, Collectible, CollectibleVariant, Food, FoodKind, Layer, Pos};
use crate::math::Vec2;
use crate::path::tile_of;
use crate::world::{tile_center, EMPTY};
use hecs::Entity;

impl Sim {
    fn food_on(&self, layer: Layer, tile: (u32, u32)) -> Vec<u32> {
        self.food_tiles
            .get(&(layer as u8, tile.0, tile.1))
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Total loose+stored food units on a cell (any kind).
    pub(crate) fn cell_food(&self, layer: Layer, tile: (u32, u32)) -> u32 {
        let mut total = 0;
        for fid in self.food_on(layer, tile) {
            let ent = self.ids[&fid];
            total += self.ecs.get::<&Food>(ent).map(|f| f.amount).unwrap_or(0);
        }
        total
    }

    pub(crate) fn spawn_food_entity(
        &mut self,
        layer: Layer,
        tile: (u32, u32),
        amount: u32,
        kind: FoodKind,
        stored: bool,
        spoil: Option<f64>,
    ) -> u32 {
        let id = self.fresh_id();
        let ent = self.ecs.spawn((
            Food {
                amount,
                kind,
                stored,
                spoil,
                harvest_t: 0.0,
                progress: 0.0,
                src: 0,
            },
            Pos {
                p: tile_center(tile.0, tile.1),
                layer,
            },
        ));
        self.ids.insert(id, ent);
        self.food_tiles
            .entry((layer as u8, tile.0, tile.1))
            .or_default()
            .insert(id);
        id
    }

    /// Loose food pile (forage target, never spoils), split across cells to
    /// respect the per-cell cap.
    pub(crate) fn spawn_food(&mut self, p: Vec2, amount: u32, kind: FoodKind) -> u32 {
        let mut first = u32::MAX;
        let mut left = amount;
        let start = tile_of(p);
        let mut r = 0u32;
        while left > 0 && r < 8 {
            for tile in self.ring_tiles(start, r) {
                if left == 0 {
                    break;
                }
                if self.grid_of(Layer::Surface).get(tile.0, tile.1) != EMPTY {
                    continue;
                }
                let room = FOOD_CELL_CAP.saturating_sub(self.cell_food(Layer::Surface, tile));
                if room == 0 {
                    continue;
                }
                let put = room.min(left);
                let id = self.spawn_food_entity(Layer::Surface, tile, put, kind, false, None);
                if first == u32::MAX {
                    first = id;
                }
                left -= put;
            }
            r += 1;
        }
        first
    }

    /// A finite map source (harvest takes time, despawns when depleted).
    pub(crate) fn spawn_source(&mut self, p: Vec2, spec: &SourceSpec, amount: u32) -> u32 {
        let id = self.fresh_id();
        let tile = tile_of(p);
        let ent = self.ecs.spawn((
            Food {
                amount,
                kind: spec.kind,
                stored: false,
                spoil: None,
                harvest_t: spec.harvest,
                progress: 0.0,
                src: spec.src,
            },
            Pos {
                p,
                layer: Layer::Surface,
            },
        ));
        self.ids.insert(id, ent);
        // sources are food for every tile-indexed lookup (cell caps, manual
        // pickup) — forgetting this silently broke standing harvests once
        self.food_tiles
            .entry((Layer::Surface as u8, tile.0, tile.1))
            .or_default()
            .insert(id);
        id
    }

    /// One unit of loose (spoiling) dropped food on a cell with room.
    pub(crate) fn spawn_unit_food(
        &mut self,
        layer: Layer,
        tile: (u32, u32),
        kind: FoodKind,
        spoil: Option<f64>,
    ) {
        for fid in self.food_on(layer, tile) {
            let ent = self.ids[&fid];
            let room = self
                .ecs
                .get::<&Food>(ent)
                .map(|f| f.kind == kind && f.amount < FOOD_CELL_CAP)
                .unwrap_or(false);
            if room {
                if let Ok(mut q) = self.ecs.get::<&mut Food>(ent) {
                    q.amount += 1;
                    if q.stored {
                        // pantry piles stay safe; a dropped unit joining one
                        // is stored too
                        q.stored = true;
                        q.spoil = None;
                    } else if q.spoil.is_none() {
                        q.spoil = spoil;
                    }
                }
                return;
            }
        }
        self.spawn_food_entity(layer, tile, 1, kind, false, spoil);
    }

    /// Bank carried food at a cell: credits the store, leaves a visible pile.
    pub(crate) fn store_food(&mut self, ant: u32, tile: (u32, u32)) {
        let ent = self.ids[&ant];
        let kind = match self.ecs.get::<&Carry>(ent).map(|c| *c) {
            Ok(Carry::Food(kind)) => kind,
            _ => return, // nothing edible in hand — nothing to bank
        };
        match kind {
            FoodKind::Green | FoodKind::Carbs => self.colony.carbs += 1,
            FoodKind::Super | FoodKind::Protein => self.colony.protein += 1,
            FoodKind::Water => self.colony.water += 1,
        }
        self.colony.delivered += 1;
        let rname = match kind {
            FoodKind::Water => "water",
            FoodKind::Carbs | FoodKind::Green => "carbs",
            FoodKind::Protein | FoodKind::Super => "protein",
        };
        self.ev(format!("ant #{ant} banked 1 {rname} at ({},{})", tile.0, tile.1));
        self.spawn_unit_food(Layer::Underground, tile, kind, None);
        // pantry piles never spoil and are not forage targets
        for fid in self.food_on(Layer::Underground, tile) {
            let fent = self.ids[&fid];
            if let Ok(mut q) = self.ecs.get::<&mut Food>(fent) {
                q.stored = true;
                q.spoil = None;
            }
        }
        if let Ok(mut q) = self.ecs.get::<&mut Carry>(ent) {
            *q = Carry::None;
        }
    }

    /// Advance the harvest bar on a food entity by one tick. Loose units
    /// (harvest_t == 0) are picked instantly; sources need their full
    /// pickup time per unit. Returns the carried kind when a unit is ready.
    pub(crate) fn advance_harvest(&mut self, fent: Entity) -> Option<FoodKind> {
        let (amount, kind, harvest_t, progress) = {
            let q = self.ecs.get::<&Food>(fent).ok()?;
            (q.amount, q.kind, q.harvest_t, q.progress)
        };
        if amount == 0 {
            return None;
        }
        if harvest_t <= 0.0 {
            if let Ok(mut q) = self.ecs.get::<&mut Food>(fent) {
                q.amount -= 1;
            }
            return Some(kind);
        }
        let progress = progress + crate::sim::DT;
        if progress >= harvest_t {
            if let Ok(mut q) = self.ecs.get::<&mut Food>(fent) {
                if q.amount > 0 {
                    q.amount -= 1;
                }
                q.progress = 0.0;
            }
            Some(kind)
        } else {
            if let Ok(mut q) = self.ecs.get::<&mut Food>(fent) {
                q.progress = progress;
            }
            None
        }
    }

    /// First food entity on a surface tile (the manual pickup path is
    /// surface-only; the old scan was layer-blind by accident).
    pub(crate) fn food_on_tile(&self, tile: (u32, u32)) -> Option<u32> {
        self.food_on(Layer::Surface, tile).first().copied()
    }

    pub(crate) fn food_info(&self, fid: u32) -> Option<((u32, u32), u32)> {
        let ent = *self.ids.get(&fid)?;
        let mut qo = self.ecs.query_one::<(&Food, &Pos)>(ent).ok()?;
        let q = qo.get()?;
        Some((tile_of(q.1.p), q.0.amount))
    }
}

impl Sim {
    /// Spawn a nest-building collectible on the surface.
    pub(crate) fn spawn_collectible(&mut self, p: Vec2, variant: CollectibleVariant) -> u32 {
        let id = self.fresh_id();
        let ent = self.ecs.spawn((
            Collectible { variant },
            Pos { p, layer: Layer::Surface },
        ));
        self.ids.insert(id, ent);
        id
    }

    /// A collectible lying on this surface tile, if any.
    pub(crate) fn collectible_on_tile(
        &self,
        tile: (u32, u32),
    ) -> Option<(u32, CollectibleVariant)> {
        for (&id, &ent) in self.ids.iter() {
            let Ok(c) = self.ecs.get::<&Collectible>(ent) else { continue };
            let Ok(pos) = self.ecs.get::<&Pos>(ent) else { continue };
            if pos.layer == Layer::Surface && tile_of(pos.p) == tile {
                return Some((id, c.variant));
            }
        }
        None
    }

    /// Instant pickup: the collectible leaves the map and fills mandibles.
    pub(crate) fn take_collectible(&mut self, id: u32, variant: CollectibleVariant) -> Carry {
        if let Some(&ent) = self.ids.get(&id) {
            let _ = self.ecs.despawn(ent);
            self.ids.remove(&id);
        }
        match variant {
            CollectibleVariant::Wood => Carry::Wood,
            CollectibleVariant::Wool => Carry::Wool,
        }
    }
}
