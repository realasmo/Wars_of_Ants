//! Worker and queen AI: job selection, walk-to-act intent resolution,
//! foraging/pantry logistics decisions, scouting and source discovery.

use super::{Phase, Sim};
use crate::balance::*;
use crate::components::*;
use crate::path::{chebyshev, manhattan, tile_of};
use crate::world::{block_of, EMPTY, SOIL_SILVER};

impl Sim {
    pub(crate) fn worker_ai(&mut self) {
        if self.colony.dead {
            return;
        }
        let ids = self.ant_ids();
        for id in ids {
            let ent = match self.ids.get(&id) {
                Some(&e) => e,
                None => continue,
            };
            #[allow(clippy::type_complexity)]
            let (caste, pos, state, carrying, job, pending, retry, attack_after, dig_after, drop_after, pick_after, land_after, found_after) = {
                let mut qo = match self
                    .ecs
                    .query_one::<(&Ant, &Pos, &AntState, &Carry, &WorkerAi)>(ent)
                {
                    Ok(q) => q,
                    Err(_) => continue,
                };
                let q = match qo.get() {
                    Some(q) => q,
                    None => continue,
                };
                (
                    q.0.caste,
                    *q.1,
                    (*q.2).clone(),
                    *q.3,
                    q.4.job,
                    q.4.pending,
                    q.4.retry,
                    q.4.attack_after,
                    q.4.dig_after,
                    q.4.drop_after,
                    q.4.pick_after,
                    q.4.land_after,
                    q.4.found_after,
                )
            };
            // The queen is player-driven: she never takes forage/dig jobs and
            // never auto-picks-up food — only Attack orders and walk-to-dig
            // intents move her.
            if caste == Caste::Queen {
                if !matches!(state, AntState::Idle) || retry > 0 {
                    continue;
                }
                if land_after {
                    // the flight reached its destination: touch down
                    self.set_land_after(id, false);
                    self.colony.phase = Phase::Grounded;
                    let p = self.ant_pos(id);
                    self.ev(format!("queen landed at ({:.0},{:.0})", p.x, p.y));
                    continue;
                }
                if let Some((tx, ty)) = found_after {
                    if self.ant_tile(id) == (tx, ty) {
                        self.set_found_after(id, None);
                        self.try_found_nest(id, tx, ty);
                    } else if !self.route(id, Layer::Surface, (tx, ty)) {
                        self.set_found_after(id, None);
                        self.set_retry(id, 60);
                    }
                    continue;
                }
                if let Some((bx, by)) = dig_after {
                    if !self.block_soft(bx, by) {
                        self.set_dig_after(id, None);
                    } else if self.block_adjacent(id, bx, by) {
                        self.set_dig_after(id, None);
                        self.set_state(
                            id,
                            AntState::Digging {
                                tx: bx,
                                ty: by,
                                progress: 0.0,
                                resume: None,
                            },
                        );
                    } else if !self.route_to_block(id, bx, by) {
                        self.set_dig_after(id, None);
                        self.set_retry(id, 60);
                    }
                    continue;
                }
                if let Some((tx, ty)) = drop_after {
                    if self.carry_of(id) == Carry::None {
                        self.set_drop_after(id, None);
                    } else if self.drop_in_range(id, tx, ty) {
                        self.set_drop_after(id, None);
                        self.try_drop(id, tx, ty);
                    } else if !self.route_for_drop(id, tx, ty) {
                        self.set_drop_after(id, None);
                        self.set_retry(id, 60);
                    }
                    continue;
                }
                if let Some(eid) = pick_after {
                    self.resolve_pick_after(id, eid);
                    continue;
                }
                if let Some(tid) = attack_after {
                    let ok = self
                        .ids
                        .get(&tid)
                        .and_then(|&tent| {
                            self.ecs
                                .get::<&Pos>(tent)
                                .ok()
                                .map(|q| (q.layer, tile_of(q.p)))
                        })
                        .map(|(tlayer, ttile)| {
                            if let (Layer::Surface, Layer::Surface) = (tlayer, pos.layer) {
                                self.set_state(id, AntState::Fighting { target: tid });
                                true
                            } else {
                                self.route(id, tlayer, ttile)
                            }
                        })
                        .unwrap_or(true);
                    if ok {
                        self.set_attack_after(id, None);
                    } else {
                        self.set_retry(id, 60);
                    }
                }
                continue;
            }
            if !matches!(state, AntState::Idle) {
                continue;
            }
            if retry > 0 {
                self.set_retry(id, retry - 1);
                continue;
            }
            if let Some(tid) = attack_after {
                let tinfo = self.ids.get(&tid).and_then(|&tent| {
                    self.ecs
                        .get::<&Pos>(tent)
                        .ok()
                        .map(|q| (q.layer, tile_of(q.p)))
                });
                match tinfo {
                    Some((Layer::Surface, _ttile)) if pos.layer == Layer::Surface => {
                        self.set_pending(id, None);
                        self.set_attack_after(id, None);
                        self.set_state(id, AntState::Fighting { target: tid });
                    }
                    Some((tlayer, ttile)) => {
                        if !self.route(id, tlayer, ttile) {
                            self.set_attack_after(id, None);
                            self.set_retry(id, 60);
                        }
                    }
                    None => {
                        self.set_attack_after(id, None);
                    }
                }
                continue;
            }
            if let Some((layer, tile)) = pending {
                self.set_pending(id, None);
                if !self.route(id, layer, tile) {
                    self.set_retry(id, 60);
                }
                continue;
            }
            // walk-to-dig intents: dig the remembered block on arrival
            if let Some((bx, by)) = dig_after {
                if !self.block_soft(bx, by) {
                    self.set_dig_after(id, None);
                } else if self.block_adjacent(id, bx, by) {
                    self.set_dig_after(id, None);
                    self.set_state(
                        id,
                        AntState::Digging {
                            tx: bx,
                            ty: by,
                            progress: 0.0,
                            resume: None,
                        },
                    );
                } else if !self.route_to_block(id, bx, by) {
                    self.set_dig_after(id, None);
                    self.set_retry(id, 60);
                }
                continue;
            }
            // walk-to-drop / walk-to-pick intents resolve on arrival
            if let Some((tx, ty)) = drop_after {
                if self.carry_of(id) == Carry::None {
                    self.set_drop_after(id, None);
                } else if self.drop_in_range(id, tx, ty) {
                    self.set_drop_after(id, None);
                    self.try_drop(id, tx, ty);
                } else if !self.route_for_drop(id, tx, ty) {
                    self.set_drop_after(id, None);
                    self.set_retry(id, 60);
                }
                continue;
            }
            if let Some(eid) = pick_after {
                self.resolve_pick_after(id, eid);
                continue;
            }
            match job {
                Job::Manual => {
                    if pos.layer == Layer::Surface && carrying == Carry::None {
                        if let Some(fid) = self.food_on_tile(tile_of(pos.p)) {
                            if let Some(&fent) = self.ids.get(&fid) {
                                if let Some(kind) = self.advance_harvest(fent) {
                                    if let Some(&aent) = self.ids.get(&id) {
                                        if let Ok(mut q) = self.ecs.get::<&mut Carry>(aent) {
                                            *q = Carry::Food(kind);
                                        }
                                    }
                                }
                            }
                        }
                    } else if pos.layer == Layer::Underground
                        && matches!(
                            carrying,
                            Carry::Food(FoodKind::Green | FoodKind::Super)
                        )
                    {
                        // bank carried food: on silver cells, or beside the
                        // queen when no pantry cell is handy
                        let tile = tile_of(pos.p);
                        let on_silver =
                            self.soil_at(Layer::Underground, tile.0, tile.1) == SOIL_SILVER;
                        let near_queen = chebyshev(tile, self.queen_tile()) <= 1;
                        if (on_silver || near_queen)
                            && self.cell_food(Layer::Underground, tile) < FOOD_CELL_CAP
                        {
                            self.store_food(id, tile);
                        }
                    }
                }
                Job::Idle => {
                    if carrying != Carry::None {
                        match self.pantry_tile() {
                            Some(t) => self.set_job(id, Job::Deliver(t.0, t.1)),
                            None => {
                                // pantry full: dig out more nest, then deliver
                                self.ev(format!("pantry full — ant #{id} digs expansion"));
                                match self.pick_dig_target() {
                                    Some(t) => self.set_job(id, Job::DigTile(t.0, t.1)),
                                    None => self.set_retry(id, 100),
                                }
                            }
                        }
                    } else if let Some(t) = self.colony.dig_queue.pop() {
                        self.set_job(id, Job::DigTile(t.0, t.1));
                    } else if let Some(fid) = self.best_food() {
                        self.set_job(id, Job::Fetch(fid));
                    } else if let Some((ex, ey)) = self.world.entrance {
                        // nothing known: scout outward from the nest
                        let a = self.rng.range(0.0, std::f64::consts::TAU);
                        let d = self.rng.range(12.0, 35.0);
                        let dx = (ex as f64 + a.cos() * d).clamp(2.0, self.config.width as f64 - 3.0)
                            as u32;
                        let dy = (ey as f64 + a.sin() * d).clamp(2.0, self.config.height as f64 - 3.0)
                            as u32;
                        self.set_job(id, Job::Scout(dx, dy));
                    } else {
                        self.set_retry(id, 50);
                    }
                }
                Job::DigTile(tx, ty) => {
                    let (bx, by) = block_of(tx, ty);
                    if !self.block_soft(bx, by) {
                        self.set_job(id, Job::Idle);
                    } else if pos.layer == Layer::Underground && self.block_adjacent(id, bx, by) {
                        self.set_state(
                            id,
                            AntState::Digging {
                                tx: bx,
                                ty: by,
                                progress: 0.0,
                                resume: None,
                            },
                        );
                    } else if !self.route_to_block(id, bx, by) {
                        self.set_job(id, Job::Idle);
                        self.set_retry(id, 60);
                    }
                }
                Job::Scout(dx, dy) => {
                    if pos.layer == Layer::Underground {
                        // scouting happens on the surface
                        if !self.route(id, Layer::Surface, (dx, dy)) {
                            self.set_retry(id, 60);
                            self.set_job(id, Job::Idle);
                        }
                    } else if tile_of(pos.p) == (dx, dy) {
                        // looked around; let Idle decide the next move
                        self.set_job(id, Job::Idle);
                        self.set_retry(id, 10);
                    } else if !self.route(id, Layer::Surface, (dx, dy)) {
                        self.set_retry(id, 60);
                        self.set_job(id, Job::Idle);
                    }
                }
                Job::Fetch(fid) => {
                    let fent = match self.ids.get(&fid) {
                        Some(&e) => e,
                        None => {
                            self.set_job(id, Job::Idle);
                            continue;
                        }
                    };
                    let Some((ftile, amount)) = self.food_info(fid) else {
                        self.set_job(id, Job::Idle);
                        continue;
                    };
                    if pos.layer == Layer::Surface {
                        if tile_of(pos.p) == ftile {
                            if amount > 0 {
                                if let Some(kind) = self.advance_harvest(fent) {
                                    if let Some(&aent) = self.ids.get(&id) {
                                        if let Ok(mut q) = self.ecs.get::<&mut Carry>(aent) {
                                            *q = Carry::Food(kind);
                                        }
                                    }
                                    match self.pantry_tile() {
                                        Some(t) => self.set_job(id, Job::Deliver(t.0, t.1)),
                                        None => {
                                            // pantry full: dig out more nest first
                                            self.ev(format!("pantry full — ant #{id} digs expansion"));
                                            match self.pick_dig_target() {
                                                Some(t) => self.set_job(id, Job::DigTile(t.0, t.1)),
                                                None => self.set_retry(id, 100),
                                            }
                                        }
                                    }
                                }
                                // else: still harvesting — stay on the source
                            } else {
                                self.set_job(id, Job::Idle);
                            }
                        } else if !self.route(id, Layer::Surface, ftile) {
                            self.set_retry(id, 60);
                            self.set_job(id, Job::Idle);
                        }
                    } else if !self.route(id, Layer::Surface, ftile) {
                        self.set_retry(id, 60);
                    }
                }
                Job::Deliver(tx, ty) => {
                    if carrying == Carry::None {
                        self.set_job(id, Job::Idle);
                        continue;
                    }
                    if pos.layer == Layer::Underground {
                        let tile = tile_of(pos.p);
                        if tile == (tx, ty) {
                            if self.cell_food(Layer::Underground, tile) < FOOD_CELL_CAP {
                                self.store_food(id, tile);
                                self.set_job(id, Job::Idle);
                            } else {
                                // pantry cell filled up on the way — re-pick
                                self.set_job(id, Job::Idle);
                            }
                        } else if !self.route(id, Layer::Underground, (tx, ty)) {
                            self.set_retry(id, 60);
                            self.set_job(id, Job::Idle);
                        }
                    } else if !self.route(id, Layer::Underground, (tx, ty)) {
                        self.set_retry(id, 60);
                    }
                }
            }
        }
    }

    /// Sources within sight of any colony ant become known (scouting).
    pub(crate) fn discover(&mut self) {
        if self.colony.dead {
            return;
        }
        let ants: Vec<(u32, u32)> = self
            .ant_ids()
            .into_iter()
            .map(|id| self.ant_tile(id))
            .collect();
        let mut found = Vec::new();
        for &fid in &self.food_ids() {
            let ent = self.ids[&fid];
            let Ok(fp) = self.ecs.get::<&Pos>(ent) else {
                continue;
            };
            if fp.layer != Layer::Surface {
                continue;
            }
            let Ok(ff) = self.ecs.get::<&Food>(ent) else {
                continue;
            };
            if ff.harvest_t <= 0.0 || ff.amount == 0 || self.colony.known.contains(&fid) {
                continue;
            }
            let tile = tile_of(fp.p);
            if ants
                .iter()
                .any(|&(ax, ay)| chebyshev((ax, ay), tile) <= SIGHT_RANGE)
            {
                found.push(fid);
            }
        }
        for fid in found {
            self.colony.known.insert(fid);
            if let Some(&ent) = self.ids.get(&fid) {
                let info = self
                    .ecs
                    .query_one::<(&Food, &Pos)>(ent)
                    .ok()
                    .and_then(|mut q| q.get().map(|(f, p)| (f.src, f.amount, tile_of(p.p))));
                if let Some((src, amount, tile)) = info {
                    self.ev(format!(
                        "source discovered: {} #{} — {} units at ({},{})",
                        super::snapshot::source_name(src),
                        fid,
                        amount,
                        tile.0,
                        tile.1
                    ));
                }
            }
        }
    }

    pub(crate) fn best_food(&self) -> Option<u32> {
        let entrance = self.world.entrance?;
        let mut best: Option<(u32, u32, u32)> = None;
        for &fid in &self.food_ids() {
            let ent = self.ids[&fid];
            let (Ok(fp), Ok(ff)) = (self.ecs.get::<&Pos>(ent), self.ecs.get::<&Food>(ent)) else {
                continue;
            };
            if ff.stored {
                continue; // pantry piles are not forage targets
            }
            if ff.harvest_t > 0.0 && !self.colony.known.contains(&fid) {
                continue; // undiscovered sources need scouting first
            }
            if ff.amount == 0 {
                continue;
            }
            let tile = tile_of(fp.p);
            let super_first = if ff.kind == FoodKind::Super || ff.kind == FoodKind::Protein {
                0
            } else {
                1
            };
            let key = (super_first, manhattan(tile, entrance), fid);
            if best.map(|b| key < b).unwrap_or(true) {
                best = Some(key);
            }
        }
        best.map(|(_, _, fid)| fid)
    }

    /// Where carriers place collected food: the nearest silver cell to the
    /// entrance with room, else the first free cell (both visible + safe).
    pub(crate) fn pantry_tile(&self) -> Option<(u32, u32)> {
        let (ex, ey) = self.world.entrance?;
        let mut fallback: Option<(u32, u32)> = None;
        for r in 0..=14u32 {
            for tile in self.ring_tiles((ex, ey), r) {
                if self.world.underground.get(tile.0, tile.1) != EMPTY {
                    continue;
                }
                if self.cell_food(Layer::Underground, tile) >= FOOD_CELL_CAP {
                    continue;
                }
                if self.soil_at(Layer::Underground, tile.0, tile.1) == SOIL_SILVER {
                    return Some(tile);
                }
                if fallback.is_none() {
                    fallback = Some(tile);
                }
            }
        }
        fallback
    }

    pub(crate) fn pick_dig_target(&self) -> Option<(u32, u32)> {
        let (qx, qy) = self.queen_tile();
        for r in 1u32..=DIG_EXPAND_RADIUS {
            for dy in -(r as i32)..=(r as i32) {
                for dx in -(r as i32)..=(r as i32) {
                    if dx.abs() != r as i32 && dy.abs() != r as i32 {
                        continue;
                    }
                    let x = qx as i32 + dx;
                    let y = qy as i32 + dy;
                    if x < 1
                        || y < 1
                        || x >= self.config.width as i32 - 1
                        || y >= self.config.height as i32 - 1
                    {
                        continue;
                    }
                    let (x, y) = (x as u32, y as u32);
                    let (bx, by) = block_of(x, y);
                    if !self.block_soft(bx, by) {
                        continue;
                    }
                    if self.has_empty_neighbor(x, y) {
                        return Some((x, y));
                    }
                }
            }
        }
        None
    }

    pub(crate) fn has_empty_neighbor(&self, x: u32, y: u32) -> bool {
        [(1u32, 0u32), (u32::MAX, 0), (0, 1), (0, u32::MAX)]
            .iter()
            .any(|&(dx, dy)| {
                let nx = x.wrapping_add(dx);
                let ny = y.wrapping_add(dy);
                self.world.underground.in_bounds(nx, ny)
                    && self.world.underground.get(nx, ny) == EMPTY
            })
    }

    pub(crate) fn free_egg_tile(&self) -> Option<(u32, u32)> {
        let (qx, qy) = self.queen_tile();
        for r in 0u32..=3 {
            for dy in -(r as i32)..=(r as i32) {
                for dx in -(r as i32)..=(r as i32) {
                    if dx.abs() != r as i32 && dy.abs() != r as i32 {
                        continue;
                    }
                    let x = qx as i32 + dx;
                    let y = qy as i32 + dy;
                    if x < 1
                        || y < 1
                        || x >= self.config.width as i32 - 1
                        || y >= self.config.height as i32 - 1
                    {
                        continue;
                    }
                    let (x, y) = (x as u32, y as u32);
                    if self.world.underground.get(x, y) == EMPTY
                        && !self.tile_occupied((x, y), Layer::Underground)
                    {
                        return Some((x, y));
                    }
                }
            }
        }
        None
    }

    pub(crate) fn queen_tile(&self) -> (u32, u32) {
        self.ant_tile(self.colony.queen_id)
    }

    pub(crate) fn caste_counts(&self) -> (u32, u32) {
        let mut workers = 0;
        let mut soldiers = 0;
        for id in self.ant_ids() {
            let ent = self.ids[&id];
            if let Ok(q) = self.ecs.get::<&Ant>(ent) {
                match q.caste {
                    Caste::Worker => workers += 1,
                    Caste::Soldier => soldiers += 1,
                    Caste::Queen => {}
                }
            }
        }
        (workers, soldiers)
    }

    fn tile_occupied(&self, tile: (u32, u32), layer: Layer) -> bool {
        for (_e, (_egg, pos)) in self.ecs.query::<(&Egg, &Pos)>().iter() {
            if pos.layer == layer && tile_of(pos.p) == tile {
                return true;
            }
        }
        false
    }
}
