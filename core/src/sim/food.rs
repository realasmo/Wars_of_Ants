//! Food logistics: piles, finite sources, dropped units, the pantry, harvest
//! progress and spoiling.

use super::{Phase, Sim};
use crate::components::{Carry, Collectible, CollectibleVariant, Food, FoodKind, Layer, Pos};
use crate::math::Vec2;
use crate::path::tile_of;
use crate::rules::SourceSpec;
use crate::world::{block_of, tile_center, EMPTY, SOIL_SILVER};
use hecs::Entity;

impl Sim {
    pub(crate) fn food_on(&self, layer: Layer, tile: (u32, u32)) -> Vec<u32> {
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

    /// Room for `kind` at a cell. Underground 2×2 storage blocks are
    /// TYPED: the block holds at most `food_block_cap` units of exactly
    /// one kind — a block holding another kind has no room. Surface cells
    /// have no blocks: per-tile mixed storage with the same number.
    pub(crate) fn block_room(&self, layer: Layer, tile: (u32, u32), kind: FoodKind) -> u32 {
        if layer == Layer::Surface {
            return self
                .rules
                .food_block_cap
                .saturating_sub(self.cell_food(layer, tile));
        }
        let (bx, by) = block_of(tile.0, tile.1);
        let mut total = 0u32;
        let mut foreign = false;
        for dy in 0..2u32 {
            for dx in 0..2u32 {
                for fid in self.food_on(Layer::Underground, (bx + dx, by + dy)) {
                    let ent = self.ids[&fid];
                    let Ok(f) = self.ecs.get::<&Food>(ent) else {
                        continue;
                    };
                    if f.amount == 0 {
                        continue;
                    }
                    if f.kind != kind {
                        foreign = true;
                    }
                    total += f.amount;
                }
            }
        }
        if foreign {
            0
        } else {
            self.rules.food_block_cap.saturating_sub(total)
        }
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
                let room = self.block_room(Layer::Surface, tile, kind);
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

    /// One unit of dropped/banked food at a cell that has room for its
    /// kind (typed blocks). Joins a same-kind pile when possible. Returns
    /// false without placing when the cell's block is full or holds
    /// another kind — callers refuse (player drop), pick another cell
    /// (AI), or spill (a falling carrier).
    pub(crate) fn spawn_unit_food(
        &mut self,
        layer: Layer,
        tile: (u32, u32),
        kind: FoodKind,
        spoil: Option<f64>,
    ) -> bool {
        if self.block_room(layer, tile, kind) == 0 {
            return false;
        }
        for fid in self.food_on(layer, tile) {
            let ent = self.ids[&fid];
            let room = self
                .ecs
                .get::<&Food>(ent)
                .map(|f| f.kind == kind && f.amount > 0)
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
                self.note_unit_food(layer, tile, kind);
                return true;
            }
        }
        self.spawn_food_entity(layer, tile, 1, kind, false, spoil);
        self.note_unit_food(layer, tile, kind);
        true
    }

    /// Side effects of a unit landing: the founding water quest counts
    /// every water unit placed on a food block (silver cell) while the
    /// quest is live. One chokepoint — banking and manual drops both pass
    /// through `spawn_unit_food`.
    fn note_unit_food(&mut self, layer: Layer, tile: (u32, u32), kind: FoodKind) {
        if kind == FoodKind::Water
            && layer == Layer::Underground
            && self.colony.phase == Phase::Founding
            && self.soil_at(Layer::Underground, tile.0, tile.1) == SOIL_SILVER
        {
            self.colony.quest_water_tally += 1;
            self.ev(format!(
                "founding: water stored on a food block ({}/{})",
                self.colony.quest_water_tally, self.rules.founding_quest_water
            ));
        }
    }

    /// Bank carried food at a cell: credits the store, leaves a visible pile.
    /// Refuses (carries on holding the unit) when the cell's typed block is
    /// full or holds another kind — callers pre-check, so this is rare.
    pub(crate) fn store_food(&mut self, ant: u32, tile: (u32, u32)) {
        let ent = self.ids[&ant];
        let kind = match self.ecs.get::<&Carry>(ent).map(|c| *c) {
            Ok(Carry::Food(kind)) => kind,
            _ => return, // nothing edible in hand — nothing to bank
        };
        // place first, credit only when the unit really landed
        if !self.spawn_unit_food(Layer::Underground, tile, kind, None) {
            return;
        }
        match kind {
            FoodKind::Green | FoodKind::Carbs => self.colony.carbs += 1,
            FoodKind::Super | FoodKind::Protein => self.colony.protein += 1,
            FoodKind::Water => self.colony.water += 1,
            FoodKind::Honeydew => self.colony.honeydew += 1,
        }
        self.colony.delivered += 1;
        let rname = match kind {
            FoodKind::Water => "water",
            FoodKind::Carbs | FoodKind::Green => "carbs",
            FoodKind::Protein | FoodKind::Super => "protein",
            FoodKind::Honeydew => "honeydew",
        };
        self.ev(format!(
            "ant #{ant} banked 1 {rname} at ({},{})",
            tile.0, tile.1
        ));
        // pantry piles never spoil and are not forage targets (same kind
        // only — typed blocks keep foreign kinds off this block anyway)
        for fid in self.food_on(Layer::Underground, tile) {
            let fent = self.ids[&fid];
            if let Ok(mut q) = self.ecs.get::<&mut Food>(fent) {
                if q.kind == kind {
                    q.stored = true;
                    q.spoil = None;
                }
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

    // --- physical queen feeding (worker-priorities wave) ---

    /// What the queen requests right now.
    pub(crate) fn craving(&self) -> FoodKind {
        self.rules.craving_cycle[self
            .colony
            .craving_i
            .min(self.rules.craving_cycle.len() - 1)]
    }

    /// The pantry pile the feeder should withdraw from: the nearest stored
    /// pile of the craved kind to the queen (tie → lowest id). None when the
    /// pantry cannot satisfy the craving.
    pub(crate) fn craving_pile(&self) -> Option<(u32, (u32, u32))> {
        let craving = self.craving();
        let q = self.queen_tile();
        let mut best: Option<(u32, u32)> = None; // (chebyshev dist, id)
        for &fid in &self.food_ids() {
            let ent = self.ids[&fid];
            let (Ok(fp), Ok(ff)) = (self.ecs.get::<&Pos>(ent), self.ecs.get::<&Food>(ent)) else {
                continue;
            };
            if !ff.stored || ff.amount == 0 || ff.kind != craving {
                continue;
            }
            if fp.layer != Layer::Underground {
                continue; // the pantry is underground by definition
            }
            let key = (crate::path::chebyshev(tile_of(fp.p), q), fid);
            if best.map(|b| key < b).unwrap_or(true) {
                best = Some(key);
            }
        }
        best.map(|(_, fid)| (fid, self.food_info(fid).map(|(t, _)| t).unwrap_or((0, 0))))
    }

    /// Total units of one kind sitting in stored pantry piles (the physical
    /// balance; `colony.*` is the ledger and the two move together).
    pub(crate) fn pantry_units(&self, kind: FoodKind) -> u32 {
        let mut total = 0;
        for &fid in &self.food_ids() {
            let ent = self.ids[&fid];
            let Ok(f) = self.ecs.get::<&Food>(ent) else {
                continue;
            };
            if f.stored && f.kind == kind {
                total += f.amount;
            }
        }
        total
    }

    /// Remove `n` stored units of one kind from the pantry piles (lowest-id
    /// piles first) and debit the ledger. Returns false without touching
    /// anything when the pantry cannot satisfy the full amount — brood costs
    /// and medic water pay atomically, all kinds or nothing.
    pub(crate) fn withdraw_pantry_units(&mut self, kind: FoodKind, n: u32) -> bool {
        if self.pantry_units(kind) < n {
            return false;
        }
        let mut left = n;
        for fid in self.food_ids() {
            if left == 0 {
                break;
            }
            let Some(&fent) = self.ids.get(&fid) else {
                continue;
            };
            let take = {
                let Ok(mut f) = self.ecs.get::<&mut Food>(fent) else {
                    continue;
                };
                if !f.stored || f.kind != kind || f.amount == 0 {
                    continue;
                }
                let take = f.amount.min(left);
                f.amount -= take;
                take
            };
            left -= take;
        }
        match kind {
            FoodKind::Green | FoodKind::Carbs => {
                self.colony.carbs = self.colony.carbs.saturating_sub(n)
            }
            FoodKind::Super | FoodKind::Protein => {
                self.colony.protein = self.colony.protein.saturating_sub(n)
            }
            FoodKind::Water => self.colony.water = self.colony.water.saturating_sub(n),
            FoodKind::Honeydew => self.colony.honeydew = self.colony.honeydew.saturating_sub(n),
        }
        true
    }

    /// Withdraw one craved unit from a pantry pile into the mandibles: pile
    /// −1 and store (ledger) −1 — the unit leaves the pantry accounting and
    /// settles when fed to the queen or re-banked.
    pub(crate) fn withdraw_pantry(&mut self, id: u32, fid: u32) -> bool {
        let Some(&fent) = self.ids.get(&fid) else {
            return false;
        };
        let (amount, kind) = match self
            .ecs
            .query_one::<&mut Food>(fent)
            .ok()
            .and_then(|mut q| q.get().map(|f| (f.amount, f.kind)))
        {
            Some(v) => v,
            None => return false,
        };
        if amount == 0 {
            return false;
        }
        match kind {
            FoodKind::Green | FoodKind::Carbs => {
                self.colony.carbs = self.colony.carbs.saturating_sub(1)
            }
            FoodKind::Super | FoodKind::Protein => {
                self.colony.protein = self.colony.protein.saturating_sub(1)
            }
            FoodKind::Water => self.colony.water = self.colony.water.saturating_sub(1),
            FoodKind::Honeydew => self.colony.honeydew = self.colony.honeydew.saturating_sub(1),
        }
        if let Ok(mut q) = self.ecs.get::<&mut Food>(fent) {
            q.amount = amount - 1;
        }
        if let Some(&aent) = self.ids.get(&id) {
            if let Ok(mut q) = self.ecs.get::<&mut Carry>(aent) {
                *q = Carry::Food(kind);
            }
        }
        self.ev(format!(
            "ant #{id} withdrew 1 {} from the pantry for the queen",
            super::snapshot::food_name(kind)
        ));
        true
    }

    /// Feed the queen from the adjacent carrier's mandibles: caller guarantees
    /// same layer, adjacency, the craved kind carried, and that she is hungry.
    pub(crate) fn feed_queen(&mut self, feeder: u32) {
        let Some(&aent) = self.ids.get(&feeder) else {
            return;
        };
        let kind = match self.ecs.get::<&Carry>(aent).map(|c| *c) {
            Ok(Carry::Food(k)) => k,
            _ => return,
        };
        if kind != self.craving() {
            return;
        }
        if let Ok(mut q) = self.ecs.get::<&mut Carry>(aent) {
            *q = Carry::None;
        }
        self.queen_fed(format!("feeder #{feeder}"), kind);
    }

    /// The queen eats one craved unit (from a feeder or her own mandibles):
    /// the unit is consumed (caller cleared the carry), hunger resets, the
    /// craving advances to the next in the cycle.
    pub(crate) fn queen_fed(&mut self, who: String, fed: FoodKind) {
        self.colony.hunger_t = 0.0;
        self.colony.craving_i = (self.colony.craving_i + 1) % self.rules.craving_cycle.len();
        let next = self.craving();
        self.ev(format!(
            "{who} fed the queen 1 {} — next: {}",
            super::snapshot::food_name(fed),
            super::snapshot::food_name(next),
        ));
    }

    /// The queen's current layer + tile (worker_ai feeds/fetches toward this).
    pub(crate) fn queen_where(&self) -> (Layer, (u32, u32)) {
        let Some(&ent) = self.ids.get(&self.colony.queen_id) else {
            return (Layer::Underground, self.queen_tile());
        };
        match self.ecs.get::<&Pos>(ent).map(|p| (p.layer, tile_of(p.p))) {
            Ok(v) => v,
            Err(_) => (Layer::Underground, self.queen_tile()),
        }
    }
}

impl Sim {
    /// Spawn a nest-building collectible on the surface.
    pub(crate) fn spawn_collectible(&mut self, p: Vec2, variant: CollectibleVariant) -> u32 {
        let id = self.fresh_id();
        let ent = self.ecs.spawn((
            Collectible { variant },
            Pos {
                p,
                layer: Layer::Surface,
            },
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
            let Ok(c) = self.ecs.get::<&Collectible>(ent) else {
                continue;
            };
            let Ok(pos) = self.ecs.get::<&Pos>(ent) else {
                continue;
            };
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
