//! Player commands: validation, walk-to-act intents, routing, and the
//! founding ritual.

use super::{Command, Phase, Sim};
use crate::balance::*;
use crate::components::*;
use crate::math::Vec2;
use crate::path::{chebyshev, find_path, smooth_path, tile_of};
use crate::world::{block_of, tile_center, EMPTY};

impl Sim {
    pub(crate) fn apply_command(&mut self, cmd: Command) -> bool {
        match cmd {
            Command::Move { ant, x, y } => {
                if !self.is_ant(ant) {
                    return false;
                }
                let tx = x.floor().clamp(0.0, self.config.width as f64 - 1.0) as u32;
                let ty = y.floor().clamp(0.0, self.config.height as f64 - 1.0) as u32;
                let layer = self.ant_layer(ant);
                self.set_job(ant, Job::Manual);
                self.route(ant, layer, (tx, ty))
            }
            Command::Dig { ant, tx, ty } => {
                let ent = match self.ids.get(&ant) {
                    Some(&e) => e,
                    None => return false,
                };
                let (caste, pos, carrying) = match self
                    .ecs
                    .query_one::<(&Ant, &Pos, &Carrying)>(ent)
                    .unwrap()
                    .get()
                {
                    Some(q) => (q.0.caste, q.1.layer, *q.2),
                    None => return false,
                };
                let queen_may_dig = caste == Caste::Queen
                    && self.colony.founding
                    && matches!(
                        self.colony.phase,
                        Phase::Founding | Phase::Brood
                    )
                    && (carrying.amount == 0
                        || (carrying.kind == FoodKind::Dirt
                            && carrying.amount < DIRT_CAPACITY));
                if (caste != Caste::Worker && !queen_may_dig)
                    || pos != Layer::Underground
                    || !self.world.underground.in_bounds(tx, ty)
                {
                    return false;
                }
                let (bx, by) = block_of(tx, ty);
                if !self.block_in_bounds(bx, by) || !self.block_soft(bx, by) {
                    return false;
                }
                self.set_job(ant, Job::Manual);
                if self.block_adjacent(ant, bx, by) {
                    self.set_dig_after(ant, None);
                    self.set_state(
                        ant,
                        AntState::Digging {
                            tx: bx,
                            ty: by,
                            progress: 0.0,
                            resume: None,
                        },
                    );
                } else {
                    // walk to the block first, dig on arrival
                    self.set_dig_after(ant, Some((bx, by)));
                    if !self.route_to_block(ant, bx, by) {
                        self.set_dig_after(ant, None);
                        return false;
                    }
                }
                true
            }
            Command::Attack { ant, target } => {
                if !self.is_ant(ant) {
                    return false;
                }
                let Some(&tent) = self.ids.get(&target) else {
                    return false;
                };
                let attackable =
                    self.ecs.get::<&Predator>(tent).is_ok() || self.ecs.get::<&Ant>(tent).is_ok();
                if !attackable {
                    return false;
                }
                self.set_job(ant, Job::Manual);
                self.set_attack_after(ant, Some(target));
                true
            }
            Command::UseEntrance { ant } => {
                if !self.is_ant(ant) {
                    return false;
                }
                let layer = self.ant_layer(ant);
                self.set_job(ant, Job::Manual);
                let Some(entrance) = self.world.entrance else {
                    return false;
                };
                self.route(ant, layer.other(), entrance)
            }
            Command::Land { ant, x, y } => {
                if ant != self.colony.queen_id || self.colony.phase != Phase::Flight {
                    return false;
                }
                let tx = x.floor().clamp(0.0, self.config.width as f64 - 1.0) as u32;
                let ty = y.floor().clamp(0.0, self.config.height as f64 - 1.0) as u32;
                self.set_job(ant, Job::Manual);
                if self.ant_tile(ant) == (tx, ty) {
                    self.colony.phase = Phase::Grounded;
                    self.ev(format!("queen landed at ({tx},{ty})"));
                    return true;
                }
                // fly to the destination, land on arrival
                self.set_land_after(ant, true);
                if !self.route(ant, Layer::Surface, (tx, ty)) {
                    self.set_land_after(ant, false);
                    return false;
                }
                true
            }
            Command::FoundNest { ant, x, y } => {
                if ant != self.colony.queen_id || self.colony.phase != Phase::Grounded {
                    return false;
                }
                if self.ant_layer(ant) != Layer::Surface {
                    return false;
                }
                let tx = x.floor().clamp(0.0, self.config.width as f64 - 1.0) as u32;
                let ty = y.floor().clamp(0.0, self.config.height as f64 - 1.0) as u32;
                self.set_job(ant, Job::Manual);
                if self.ant_tile(ant) == (tx, ty) {
                    return self.try_found_nest(ant, tx, ty);
                }
                // walk to the chosen ground, found the nest on arrival
                self.set_found_after(ant, Some((tx, ty)));
                if !self.route(ant, Layer::Surface, (tx, ty)) {
                    self.set_found_after(ant, None);
                    return false;
                }
                true
            }
            Command::Drop { ant, tx, ty } => {
                if !self.is_ant(ant) {
                    return false;
                }
                if self.drop_in_range(ant, tx, ty) {
                    return self.try_drop(ant, tx, ty);
                }
                // walk to the target and drop on arrival
                self.set_job(ant, Job::Manual);
                self.set_drop_after(ant, Some((tx, ty)));
                if !self.route_for_drop(ant, tx, ty) {
                    self.set_drop_after(ant, None);
                    return false;
                }
                true
            }
            Command::PickEgg { ant, egg } => {
                if !self.is_ant(ant) {
                    return false;
                }
                let Some(&eent) = self.ids.get(&egg) else {
                    return false;
                };
                let (egg_carried, etile, elayer) = match self
                    .ecs
                    .query_one::<(&Egg, &Pos)>(eent)
                    .unwrap()
                    .get()
                {
                    Some(q) => (q.0.carried_by, tile_of(q.1.p), q.1.layer),
                    None => return false,
                };
                let aent = self.ids[&ant];
                let hands_free = self
                    .ecs
                    .get::<&Carrying>(aent)
                    .map(|c| c.amount == 0)
                    .unwrap_or(false);
                if egg_carried.is_some() || !hands_free {
                    return false;
                }
                if chebyshev(self.ant_tile(ant), etile) > 1 {
                    // walk to the egg and pick it up on arrival
                    self.set_job(ant, Job::Manual);
                    self.set_pick_after(ant, Some(egg));
                    if !self.route(ant, elayer, etile) {
                        self.set_pick_after(ant, None);
                        return false;
                    }
                    return true;
                }
                self.ev(format!("ant #{ant} picked up egg #{egg}"));
                if let Ok(mut q) = self.ecs.get::<&mut Egg>(eent) {
                    q.carried_by = Some(ant);
                }
                if let Ok(mut q) = self.ecs.get::<&mut Carrying>(aent) {
                    q.amount = 1;
                    q.kind = FoodKind::Egg;
                }
                self.set_job(ant, Job::Manual);
                true
            }
        }
    }

    /// Found the nest with its hole block at the founding tile. Caller
    /// guarantees the queen stands there, grounded, on the surface.
    pub(crate) fn try_found_nest(&mut self, ant: u32, x: u32, y: u32) -> bool {
        // the 2×2 entrance hole plus a 4×4 starter chamber below it
        // must fit inside the rock border ring
        let (bx, by) = block_of(x, y);
        if bx < 2 || by < 2 || bx + 3 > self.config.width - 3 || by + 5 > self.config.height - 3 {
            return false;
        }
        let mut carved = 0u32;
        for dy in 0..2u32 {
            for dx in 0..2u32 {
                if self.world.underground.get(bx + dx, by + dy) != EMPTY {
                    carved += 1;
                }
                self.world.underground.set(bx + dx, by + dy, EMPTY);
            }
        }
        for cy in by + 2..=by + 5 {
            for cx in bx..=bx + 3 {
                if self.world.underground.get(cx, cy) != EMPTY {
                    carved += 1;
                }
                self.world.underground.set(cx, cy, EMPTY);
            }
        }
        self.dug_tiles += carved;
        self.world.entrance = Some((bx, by));
        let qent = self.ids[&ant];
        if let Some(q) = self
            .ecs
            .query_one::<(&mut Pos, &mut AntState)>(qent)
            .unwrap()
            .get()
        {
            q.0.layer = Layer::Underground;
            q.0.p = Vec2::new(bx as f64 + 2.0, by as f64 + 4.0);
            *q.1 = AntState::Idle;
        }
        self.set_job(ant, Job::Manual);
        // founding inside a dust patch grants hidden soil blocks of
        // that color near the nest — dig them out
        let patch_soil = self
            .patches
            .iter()
            .find(|p| x >= p.x0 && x < p.x1 && y >= p.y0 && y < p.y1)
            .map(|p| p.soil);
        if let Some(soil) = patch_soil {
            let grants = self.rng.irange(PATCH_GRANT_MIN, PATCH_GRANT_MAX);
            let mut placed = 0u32;
            for _ in 0..80 {
                if placed >= grants {
                    break;
                }
                let dx = self.rng.irange(0, 16) as i32 - 8;
                let dy = self.rng.irange(0, 16) as i32 - 8;
                let gx = (bx as i32 + dx) & !1;
                let gy = (by as i32 + dy) & !1;
                if gx < 2 || gy < 2 || gx + 1 >= self.config.width as i32 - 2 || gy + 1 >= self.config.height as i32 - 2 {
                    continue;
                }
                let (gx, gy) = (gx as u32, gy as u32);
                if !self.block_soft(gx, gy) {
                    continue;
                }
                if self.soil_at(Layer::Underground, gx, gy) != crate::world::SOIL_NONE {
                    continue;
                }
                self.set_soil_block(Layer::Underground, gx, gy, soil);
                placed += 1;
            }
        }
        self.colony.phase = Phase::Founding;
        self.colony.phase_t = FOUNDING_TIME;
        self.ev(format!("nest founded at ({bx},{by}) — 60s excavation"));
        true
    }

    pub(crate) fn route(&mut self, id: u32, dest_layer: Layer, dest: (u32, u32)) -> bool {
        let ent = match self.ids.get(&id) {
            Some(&e) => e,
            None => return false,
        };
        let (is_worker, start_layer, start_p) =
            match self.ecs.query_one::<(&Ant, &Pos)>(ent).unwrap().get() {
                Some(q) => (q.0.caste == Caste::Worker, q.1.layer, q.1.p),
                None => return false,
            };
        let start_tile = tile_of(start_p);
        if start_layer == dest_layer {
            let owned;
            let grid = if is_worker && start_layer == Layer::Underground {
                owned = self.worker_grid();
                &owned
            } else {
                self.grid_of(start_layer)
            };
            match find_path(grid, start_tile, dest, is_worker) {
                Some(path) => {
                    let smooth = smooth_path(grid, start_p, &path);
                    self.set_state(
                        id,
                        AntState::Moving {
                            path: smooth,
                            next: 0,
                            then_swap: false,
                        },
                    );
                    self.set_pending(id, None);
                    true
                }
                None => false,
            }
        } else {
            let Some(entrance) = self.world.entrance else {
                return false;
            };
            let grid = self.grid_of(start_layer);
            match find_path(grid, start_tile, entrance, is_worker) {
                Some(path) => {
                    let smooth = smooth_path(grid, start_p, &path);
                    self.set_state(
                        id,
                        AntState::Moving {
                            path: smooth,
                            next: 0,
                            then_swap: true,
                        },
                    );
                    self.set_pending(id, Some((dest_layer, dest)));
                    true
                }
                None => false,
            }
        }
    }

    /// Route the ant to a stand tile next to the block. Queens only cross
    /// empty cells; workers accept soft tiles (they dig through).
    pub(crate) fn route_to_block(&mut self, ant: u32, bx: u32, by: u32) -> bool {
        let is_worker = self
            .ids
            .get(&ant)
            .map(|&e| {
                self.ecs
                    .get::<&Ant>(e)
                    .map(|a| a.caste == Caste::Worker)
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        let at = self.ant_tile(ant);
        let mut cands: Vec<((u32, u32), u32)> = Vec::new();
        for dy in -1i32..=2 {
            for dx in -1i32..=2 {
                if dx >= 0 && dx <= 1 && dy >= 0 && dy <= 1 {
                    continue; // inside the block
                }
                let x = bx as i32 + dx;
                let y = by as i32 + dy;
                if x < 1 || y < 1 || x >= self.config.width as i32 - 1 || y >= self.config.height as i32 - 1 {
                    continue;
                }
                let (x, y) = (x as u32, y as u32);
                let kind = self.world.underground.get(x, y);
                if kind == crate::world::ROCK {
                    continue;
                }
                if !is_worker && kind != EMPTY {
                    continue;
                }
                // diagonal-only stands obey the no-corner-cut rule too
                let diag = (x as i32 + 1 == bx as i32 || x as i32 == bx as i32 + 2)
                    && (y as i32 + 1 == by as i32 || y as i32 == by as i32 + 2);
                if diag {
                    let fx = if x < bx { bx } else { bx + 1 };
                    let fy = if y < by { by } else { by + 1 };
                    let open = self.world.underground.get(fx, y) == EMPTY
                        || self.world.underground.get(x, fy) == EMPTY;
                    if !open {
                        continue;
                    }
                }
                let d = (x.max(at.0) - x.min(at.0)) + (y.max(at.1) - y.min(at.1));
                cands.push(((x, y), d));
            }
        }
        cands.sort_by_key(|(_, d)| *d);
        for ((x, y), _) in cands {
            if self.route(ant, Layer::Underground, (x, y)) {
                return true;
            }
        }
        false
    }

    /// True when the ant stands where this drop can happen right now.
    pub(crate) fn drop_in_range(&self, ant: u32, tx: u32, ty: u32) -> bool {
        let Some(&ent) = self.ids.get(&ant) else {
            return false;
        };
        let (carrying, layer) = match self.ecs.query_one::<(&Carrying, &Pos)>(ent)
            .unwrap()
            .get()
        {
            Some(q) => (*q.0, q.1.layer),
            None => return false,
        };
        if carrying.amount == 0 {
            return false;
        }
        match carrying.kind {
            FoodKind::Dirt => match layer {
                Layer::Surface => true,
                Layer::Underground => {
                    let (bx, by) = block_of(tx, ty);
                    self.block_in_bounds(bx, by)
                        && self.block_empty(bx, by)
                        && self.block_adjacent(ant, bx, by)
                }
            },
            _ => chebyshev(self.ant_tile(ant), (tx, ty)) <= 1,
        }
    }

    /// Route toward a drop target: dirt refills walk to the block, other
    /// carried items walk to the tile itself.
    pub(crate) fn route_for_drop(&mut self, ant: u32, tx: u32, ty: u32) -> bool {
        let layer = self.ant_layer(ant);
        let dirt = self
            .ids
            .get(&ant)
            .and_then(|&e| self.ecs.get::<&Carrying>(e).ok())
            .map(|c| c.kind == FoodKind::Dirt && c.amount > 0)
            .unwrap_or(false);
        if layer == Layer::Underground && dirt {
            let (bx, by) = block_of(tx, ty);
            self.route_to_block(ant, bx, by)
        } else {
            self.route(ant, layer, (tx, ty))
        }
    }

    /// Attempt the drop right now (callers check drop_in_range first).
    pub(crate) fn try_drop(&mut self, ant: u32, tx: u32, ty: u32) -> bool {
        let ent = match self.ids.get(&ant) {
            Some(&e) => e,
            None => return false,
        };
        let (carrying, layer) = match self.ecs.query_one::<(&Carrying, &Pos)>(ent)
            .unwrap()
            .get()
        {
            Some(q) => (*q.0, q.1.layer),
            None => return false,
        };
        if carrying.amount == 0 {
            return false;
        }
        match carrying.kind {
            FoodKind::Dirt => match layer {
                Layer::Surface => {
                    // dumped above ground: all carried dirt disappears
                    if let Ok(mut q) = self.ecs.get::<&mut Carrying>(ent) {
                        q.amount = 0;
                    }
                    true
                }
                Layer::Underground => {
                    // dumped below: the dirt refills the fully-empty block
                    let (bx, by) = block_of(tx, ty);
                    if !self.block_in_bounds(bx, by)
                        || !self.block_empty(bx, by)
                        || !self.block_adjacent(ant, bx, by)
                    {
                        return false;
                    }
                    for dy in 0..2 {
                        for dx in 0..2 {
                            self.world.underground.set(bx + dx, by + dy, crate::world::DIRT);
                        }
                    }
                    // one dump fills one block per carried unit
                    if let Ok(mut q) = self.ecs.get::<&mut Carrying>(ent) {
                        q.amount = q.amount.saturating_sub(1);
                    }
                    true
                }
            },
            FoodKind::Egg => {
                // place the carried egg on the adjacent empty cell
                let grid_kind = self.grid_of(layer).get(tx, ty);
                if grid_kind != EMPTY || chebyshev(self.ant_tile(ant), (tx, ty)) > 1 {
                    return false;
                }
                let mut placed = false;
                for eid in self.egg_ids() {
                    let eent = self.ids[&eid];
                    let mine = self
                        .ecs
                        .get::<&Egg>(eent)
                        .map(|q| q.carried_by == Some(ant))
                        .unwrap_or(false);
                    if !mine {
                        continue;
                    }
                    if let Some(q) = self
                        .ecs
                        .query_one::<(&mut Egg, &mut Pos)>(eent)
                        .unwrap()
                        .get()
                    {
                        q.0.carried_by = None;
                        q.1.p = tile_center(tx, ty);
                        q.1.layer = layer;
                    }
                    placed = true;
                    self.ev(format!("egg #{eid} placed at ({tx},{ty})"));
                    break;
                }
                if placed {
                    if let Ok(mut q) = self.ecs.get::<&mut Carrying>(ent) {
                        q.amount = 0;
                    }
                }
                placed
            }
            FoodKind::Green | FoodKind::Super | FoodKind::Protein | FoodKind::Carbs | FoodKind::Water => {
                // drop one unit as loose (spoiling) food
                let tile = (tx, ty);
                if !self.grid_of(layer).in_bounds(tx, ty)
                    || chebyshev(self.ant_tile(ant), tile) > 1
                    || self.cell_food(layer, tile) >= FOOD_CELL_CAP
                {
                    return false;
                }
                self.spawn_unit_food(layer, tile, carrying.kind, Some(SPOIL_TIME));
                if let Ok(mut q) = self.ecs.get::<&mut Carrying>(ent) {
                    q.amount = 0;
                }
                true
            }
        }
    }

    pub(crate) fn carried_amount(&self, id: u32) -> u32 {
        self.ids
            .get(&id)
            .and_then(|&e| self.ecs.get::<&Carrying>(e).ok())
            .map(|c| c.amount)
            .unwrap_or(0)
    }

    /// Pick up the remembered egg once adjacent; drop the intent if the egg
    /// vanished or hands filled up meanwhile.
    pub(crate) fn resolve_pick_after(&mut self, id: u32, eid: u32) {
        let Some(&eent) = self.ids.get(&eid) else {
            self.set_pick_after(id, None);
            return;
        };
        let valid = self
            .ecs
            .query_one::<(&Egg, &Pos)>(eent)
            .ok()
            .and_then(|mut q| q.get().map(|q| (q.0.carried_by.is_none(), tile_of(q.1.p), q.1.layer)))
            .map(|(free, tile, _)| {
                free && self.carried_amount(id) == 0 && chebyshev(self.ant_tile(id), tile) <= 1
            })
            .unwrap_or(false);
        if !valid {
            if !self.ids.contains_key(&eid) || self.carried_amount(id) > 0 {
                self.set_pick_after(id, None);
            }
            return;
        }
        let aent = self.ids[&id];
        if let Ok(mut q) = self.ecs.get::<&mut Egg>(eent) {
            q.carried_by = Some(id);
        }
        if let Ok(mut q) = self.ecs.get::<&mut Carrying>(aent) {
            q.amount = 1;
            q.kind = FoodKind::Egg;
        }
        self.set_pick_after(id, None);
        self.set_job(id, Job::Manual);
    }

    /// The egg this ant is carrying, if any.
    pub(crate) fn egg_carried_by(&self, ant: u32) -> Option<u32> {
        for eid in self.egg_ids() {
            let ent = self.ids[&eid];
            if self
                .ecs
                .get::<&Egg>(ent)
                .map(|e| e.carried_by == Some(ant))
                .unwrap_or(false)
            {
                return Some(eid);
            }
        }
        None
    }
}
