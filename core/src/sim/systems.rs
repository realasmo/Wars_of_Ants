//! The per-tick systems, in the exact order `Sim::tick` runs them. Order is
//! part of determinism: reordering changes every seed's future.

use super::{snapshot::source_name, Phase, Sim, DT};
use crate::balance::*;
use crate::components::*;
use crate::math::Vec2;
use crate::path::tile_of;
use crate::world::tile_center;
use crate::world::{block_of, EMPTY, SOIL_ORANGE, SOIL_SILVER};

impl Sim {
    pub(crate) fn movement(&mut self) {
        let ids = self.ant_ids();
        for id in ids {
            let ent = match self.ids.get(&id) {
                Some(&e) => e,
                None => continue,
            };
            let (speed, caste, mut pos, state) = match self
                .ecs
                .query_one::<(&Ant, &Pos, &AntState)>(ent)
                .unwrap()
                .get()
            {
                Some(q) => (q.0.speed, q.0.caste, *q.1, (*q.2).clone()),
                None => continue,
            };
            // the founding queen flies faster than any ant walks
            let speed = if caste == Caste::Queen && self.colony.phase == Phase::Flight {
                QUEEN_FLY_SPEED
            } else {
                speed
            };
            // a starving colony moves sluggishly until fed (founding economy;
            // the legacy test economy is tuned without this)
            let speed = if self.colony.founding && self.colony.carbs < CARB_LOW {
                speed * CARB_SLOWDOWN
            } else {
                speed
            };
            let (path, mut next, then_swap) = match state {
                AntState::Moving {
                    path,
                    next,
                    then_swap,
                } => (path, next, then_swap),
                _ => continue,
            };
            let mut p = pos.p;
            let mut budget = speed * DT;
            let mut dig_target: Option<(u32, u32)> = None;
            let mut finished = false;
            while budget > 0.0 {
                if next >= path.len() {
                    finished = true;
                    break;
                }
                let target = path[next];
                let (tx, ty) = tile_of(target);
                if pos.layer == Layer::Underground {
                    let kind = self.world.underground.get(tx, ty);
                    if crate::world::Grid::is_soft(kind) {
                        // workers auto-dig whole blocks; a partially-solid
                        // block is a stale path — just stop and re-plan
                        let (bx, by) = block_of(tx, ty);
                        if self.block_soft(bx, by) {
                            dig_target = Some((bx, by));
                        } else {
                            finished = true;
                        }
                        break;
                    }
                }
                let d = target - p;
                let dist = d.len();
                if dist <= budget {
                    p = target;
                    next += 1;
                    budget -= dist;
                } else {
                    p = p + d * (budget / dist);
                    budget = 0.0;
                }
            }
            if next >= path.len() && dig_target.is_none() {
                finished = true;
            }
            let new_state = if let Some((tx, ty)) = dig_target {
                AntState::Digging {
                    tx,
                    ty,
                    progress: 0.0,
                    resume: Some(Box::new((path, next, then_swap))),
                }
            } else if finished {
                if then_swap {
                    pos.layer = pos.layer.other();
                    if let Some((ex, ey)) = self.world.entrance {
                        // the entrance hole is a 2×2 block — land on its center
                        p = Vec2::new(ex as f64 + 1.0, ey as f64 + 1.0);
                    }
                }
                AntState::Idle
            } else {
                AntState::Moving {
                    path,
                    next,
                    then_swap,
                }
            };
            if let Some(q) = self
                .ecs
                .query_one::<(&mut Pos, &mut AntState)>(ent)
                .unwrap()
                .get()
            {
                q.0.p = p;
                q.0.layer = pos.layer;
                *q.1 = new_state;
            }
        }
    }

    pub(crate) fn digging(&mut self) {
        let ids = self.ant_ids();
        for id in ids {
            let ent = match self.ids.get(&id) {
                Some(&e) => e,
                None => continue,
            };
            let state = match self.ecs.get::<&AntState>(ent) {
                Ok(q) => (*q).clone(),
                Err(_) => continue,
            };
            let (tx, ty, mut progress, resume) = match state {
                AntState::Digging {
                    tx,
                    ty,
                    progress,
                    resume,
                } => (tx, ty, progress, resume),
                _ => continue,
            };
            // tx, ty are the 2×2 block origin
            if !self.block_soft(tx, ty) {
                match resume {
                    Some(b) => {
                        self.set_state(
                            id,
                            AntState::Moving {
                                path: b.0,
                                next: b.1,
                                then_swap: b.2,
                            },
                        );
                    }
                    None => self.set_state(id, AntState::Idle),
                }
                continue;
            }
            let caste = self
                .ecs
                .get::<&Ant>(ent)
                .map(|a| a.caste)
                .unwrap_or(Caste::Worker);
            let dig_time = if caste == Caste::Queen {
                QUEEN_DIG_TIME
            } else {
                DIG_TIME
            };
            progress += DT;
            if progress >= dig_time {
                for dy in 0..2u32 {
                    for dx in 0..2u32 {
                        self.world.underground.set(tx + dx, ty + dy, EMPTY);
                    }
                }
                self.dug_tiles += 4;
                // the founding queen carries excavated dirt out (up to
                // DIRT_CAPACITY blocks before dumping); workers' spoil
                // handling is a later wave
                if caste == Caste::Queen {
                    if let Ok(mut q) = self.ecs.get::<&mut Carrying>(ent) {
                        q.kind = FoodKind::Dirt;
                        q.amount = (q.amount + 1).min(DIRT_CAPACITY);
                    }
                }
                match resume {
                    Some(b) => {
                        self.set_state(
                            id,
                            AntState::Moving {
                                path: b.0,
                                next: b.1,
                                then_swap: b.2,
                            },
                        );
                    }
                    None => self.set_state(id, AntState::Idle),
                }
            } else {
                self.set_state(
                    id,
                    AntState::Digging {
                        tx,
                        ty,
                        progress,
                        resume,
                    },
                );
            }
        }
    }

    pub(crate) fn combat(&mut self) {
        let ids = self.ant_ids();
        for id in ids {
            let ent = match self.ids.get(&id) {
                Some(&e) => e,
                None => continue,
            };
            let (pos, state, speed, dmg, atk_cd, atk_t) = {
                let mut qo = match self.ecs.query_one::<(&Pos, &AntState, &Ant, &Combat)>(ent) {
                    Ok(q) => q,
                    Err(_) => continue,
                };
                let q = match qo.get() {
                    Some(q) => q,
                    None => continue,
                };
                (*q.0, q.1.clone(), q.2.speed, q.3.dmg, q.3.atk_cd, q.3.atk_t)
            };
            let target = match state {
                AntState::Fighting { target } => target,
                _ => continue,
            };
            let Some(&tent) = self.ids.get(&target) else {
                self.set_state(id, AntState::Idle);
                self.set_job(id, Job::Idle);
                continue;
            };
            let tpos = self.ecs.get::<&Pos>(tent).map(|q| *q);
            let tpos = match tpos {
                Ok(t) => t,
                Err(_) => {
                    self.set_state(id, AntState::Idle);
                    self.set_job(id, Job::Idle);
                    continue;
                }
            };
            if tpos.layer != pos.layer {
                self.set_state(id, AntState::Idle);
                continue;
            }
            let mut new_pos = pos;
            let mut new_t = atk_t;
            let d = tpos.p - new_pos.p;
            let dist = d.len();
            if dist > ANT_RANGE {
                let step = speed * DT;
                if dist > step {
                    new_pos.p = new_pos.p + d * (step / dist);
                } else {
                    new_pos.p = tpos.p;
                }
                new_t = (new_t - DT).max(0.0);
            } else {
                new_t -= DT;
                if new_t <= 0.0 {
                    new_t = atk_cd;
                    if let Ok(mut c) = self.ecs.get::<&mut Combat>(tent) {
                        c.hp -= dmg;
                    }
                    if let Ok(mut p) = self.ecs.get::<&mut Predator>(tent) {
                        p.hp -= dmg;
                    }
                }
            }
            if let Some(q) = self
                .ecs
                .query_one::<(&mut Pos, &mut Combat)>(ent)
                .unwrap()
                .get()
            {
                q.0.p = new_pos.p;
                q.1.atk_t = new_t;
            }
        }
    }

    pub(crate) fn predators(&mut self) {
        let pids = self
            .ids
            .iter()
            .filter_map(|(&id, &ent)| self.ecs.get::<&Predator>(ent).is_ok().then_some(id))
            .collect::<Vec<u32>>();
        for pid in pids {
            let ent = match self.ids.get(&pid) {
                Some(&e) => e,
                None => continue,
            };
            let (mut pred, pos) = {
                let mut qo = match self.ecs.query_one::<(&mut Predator, &Pos)>(ent) {
                    Ok(q) => q,
                    Err(_) => continue,
                };
                let q = match qo.get() {
                    Some(q) => q,
                    None => continue,
                };
                (*q.0, *q.1)
            };
            if let Some(t) = pred.target {
                let valid = self
                    .ids
                    .get(&t)
                    .and_then(|&te| self.ecs.get::<&Pos>(te).ok())
                    .map(|p| p.layer == Layer::Surface)
                    .unwrap_or(false);
                if !valid {
                    pred.target = None;
                }
            }
            if pred.target.is_none() {
                // nobody is attackable while the founding queen is airborne
                if self.colony.phase != Phase::Flight {
                    let mut best: Option<(f64, u32)> = None;
                    for aid in self.ant_ids() {
                        let aent = self.ids[&aid];
                        let Ok(ap) = self.ecs.get::<&Pos>(aent) else {
                            continue;
                        };
                        if ap.layer != Layer::Surface {
                            continue;
                        }
                        let d = (ap.p - pos.p).len();
                        if d <= SPIDER_AGGRO && best.map(|(bd, _)| d < bd).unwrap_or(true) {
                            best = Some((d, aid));
                        }
                    }
                    pred.target = best.map(|(_, id)| id);
                }
            }
            let mut new_pos = pos;
            match pred.target {
                Some(t) => {
                    let tent = self.ids[&t];
                    let tp = match self.ecs.get::<&Pos>(tent) {
                        Ok(q) => *q,
                        Err(_) => {
                            continue;
                        }
                    };
                    let d = tp.p - new_pos.p;
                    let dist = d.len();
                    if dist > SPIDER.range {
                        let step = pred.speed * DT;
                        if dist > step {
                            new_pos.p = new_pos.p + d * (step / dist);
                        } else {
                            new_pos.p = tp.p;
                        }
                    } else {
                        pred.atk_t -= DT;
                        if pred.atk_t <= 0.0 {
                            pred.atk_t = pred.atk_cd;
                            if let Ok(mut c) = self.ecs.get::<&mut Combat>(tent) {
                                c.hp -= pred.dmg;
                            }
                        }
                    }
                }
                None => {
                    pred.wander_t -= DT;
                    let need_dest = pred
                        .dest
                        .map(|d| {
                            (d.0 - new_pos.p.x) * (d.0 - new_pos.p.x)
                                + (d.1 - new_pos.p.y) * (d.1 - new_pos.p.y)
                                < 0.05
                        })
                        .unwrap_or(true);
                    if need_dest || pred.wander_t <= 0.0 {
                        let dx = self.rng.range(-1.0, 1.0);
                        let dy = self.rng.range(-1.0, 1.0);
                        let len = (dx * dx + dy * dy).sqrt().max(0.001);
                        let dist = self.rng.range(0.0, SPIDER_WANDER);
                        let hx = pred.home.0 as f64 + 0.5 + dx / len * dist;
                        let hy = pred.home.1 as f64 + 0.5 + dy / len * dist;
                        pred.dest = Some((
                            hx.clamp(1.0, self.config.width as f64 - 2.0),
                            hy.clamp(1.0, self.config.height as f64 - 2.0),
                        ));
                        pred.wander_t = self.rng.range(3.0, 6.0);
                    }
                    if let Some((dx_, dy_)) = pred.dest {
                        let d = Vec2::new(dx_, dy_) - new_pos.p;
                        let dist = d.len();
                        let step = pred.speed * DT;
                        if dist > step {
                            new_pos.p = new_pos.p + d * (step / dist);
                        } else {
                            new_pos.p = Vec2::new(dx_, dy_);
                        }
                    }
                }
            }
            if let Ok(mut q) = self.ecs.query_one::<(&mut Predator, &mut Pos)>(ent) {
                if let Some(q) = q.get() {
                    *q.0 = pred;
                    q.1.p = new_pos.p;
                }
            }
        }
    }

    pub(crate) fn cleanup_deaths(&mut self) {
        let dead_ants: Vec<u32> = self
            .ant_ids()
            .into_iter()
            .filter(|&id| {
                self.ids
                    .get(&id)
                    .and_then(|&e| self.ecs.get::<&Combat>(e).ok())
                    .map(|c| c.hp <= 0.0)
                    .unwrap_or(false)
            })
            .collect();
        for id in dead_ants {
            self.kill_cause(id, "combat");
        }
        let dead_predators: Vec<u32> = self
            .ids
            .iter()
            .filter_map(|(&id, &ent)| {
                self.ecs
                    .get::<&Predator>(ent)
                    .ok()
                    .filter(|p| p.hp <= 0.0)
                    .map(|_| id)
            })
            .collect();
        for pid in dead_predators {
            let ent = self.ids[&pid];
            let p = self.ecs.get::<&Pos>(ent).map(|q| q.p).unwrap_or_default();
            self.ev(format!("spider #{pid} died → {PROTEIN_PER_SPIDER} protein"));
            self.kill(pid);
            self.spawn_food(p, PROTEIN_PER_SPIDER, FoodKind::Protein);
        }
    }

    pub(crate) fn queen_system(&mut self) {
        if self.colony.dead {
            return;
        }
        // founding script: excavation window ends → the first brood is laid
        if self.colony.phase == Phase::Founding {
            self.colony.phase_t -= DT;
            if self.colony.phase_t <= 0.0 {
                self.colony.phase = Phase::Brood;
                self.colony.phase_t = 0.0;
                self.ev("founding time over — brood laid".to_string());
                for _ in 0..FOUNDING_EGGS {
                    let Some(tile) = self.free_egg_tile() else {
                        break;
                    };
                    self.colony.eggs_laid += 1;
                    self.spawn_egg(tile_center(tile.0, tile.1), Caste::Worker, FOUNDING_EGG_HATCH);
                }
            }
        }
        let slow_now = self.colony.founding && self.colony.carbs < CARB_LOW;
        if slow_now != self.colony.slowed {
            self.colony.slowed = slow_now;
            self.ev(format!(
                "colony {} (carbs {})",
                if slow_now { "SLOWED — 60% speed" } else { "back to full speed" },
                self.colony.carbs
            ));
        }
        self.colony.lay_cooldown -= DT;
        if self.colony.carbs > 0 {
            self.colony.eat_t += DT;
            if self.colony.eat_t >= EAT_PERIOD {
                self.colony.carbs -= 1;
                self.colony.eat_t = 0.0;
            }
            self.colony.starve_t = 0.0;
        } else {
            self.colony.eat_t = 0.0;
            // founding grace: the lone queen carries reserves — starvation
            // only threatens a colony that has a workforce
            let grace = self.colony.founding && self.caste_counts().0 == 0;
            if !grace {
                self.colony.starve_t += DT;
                if self.colony.starve_t >= STARVE_TIME {
                    self.colony.dead = true;
                    let q = self.colony.queen_id;
                    self.kill_cause(q, "starvation");
                    return;
                }
            }
        }
        // ongoing auto-laying is legacy-mode only: in a founding game the
        // brood comes from the founding script (production redesign is a
        // later wave)
        if self.colony.founding {
            return;
        }
        if self.colony.lay_cooldown <= 0.0 && self.colony.ant_count < self.config.max_ants {
            let (workers, soldiers) = self.caste_counts();
            let want = if self.colony.carbs >= SOLDIER_COST_GREEN
                && self.colony.protein >= SOLDIER_COST_SUPER
                && soldiers * 2 < workers
            {
                Some(Caste::Soldier)
            } else if self.colony.carbs >= EGG_COST {
                Some(Caste::Worker)
            } else {
                None
            };
            if let Some(caste) = want {
                match self.free_egg_tile() {
                    Some(tile) => {
                        match caste {
                            Caste::Soldier => {
                                self.colony.carbs -= SOLDIER_COST_GREEN;
                                self.colony.protein -= SOLDIER_COST_SUPER;
                            }
                            Caste::Worker => self.colony.carbs -= EGG_COST,
                            Caste::Queen => {}
                        }
                        self.colony.eggs_laid += 1;
                        self.colony.lay_cooldown = LAY_COOLDOWN;
                        self.spawn_egg(tile_center(tile.0, tile.1), caste, EGG_TIME);
                    }
                    None => {
                        if self.colony.dig_queue.len() < 3 {
                            if let Some(t) = self.pick_dig_target() {
                                self.colony.dig_queue.push(t);
                            }
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn eggs(&mut self) {
        let ids = self.egg_ids();
        // carried eggs ride their carrier; a dead carrier drops them in place
        for &eid in &ids {
            let Some(&eent) = self.ids.get(&eid) else {
                continue;
            };
            let carried_by = self
                .ecs
                .get::<&Egg>(eent)
                .ok()
                .and_then(|e| e.carried_by);
            let Some(carrier) = carried_by else {
                continue;
            };
            let cpos = self.ids.get(&carrier).and_then(|&ce| {
                self.ecs.get::<&Pos>(ce).ok().map(|p| (p.p, p.layer))
            });
            match cpos {
                Some((p, layer)) => {
                    if let Some(q) = self
                        .ecs
                        .query_one::<(&mut Pos,)>(eent)
                        .unwrap()
                        .get()
                    {
                        q.0.p = p;
                        q.0.layer = layer;
                    }
                }
                None => {
                    if let Ok(mut q) = self.ecs.get::<&mut Egg>(eent) {
                        q.carried_by = None;
                    }
                }
            }
        }
        for id in ids {
            let ent = match self.ids.get(&id) {
                Some(&e) => e,
                None => continue,
            };
            let (hatch, caste, carried, pos) = match self
                .ecs
                .query_one::<(&Egg, &Pos)>(ent)
                .unwrap()
                .get()
            {
                Some(q) => (q.0.hatch, q.0.caste, q.0.carried_by, *q.1),
                None => continue,
            };
            let hatch_prev = hatch;
            let hatch = hatch - DT;
            // transformation happens only on an empty orange cell — ready
            // eggs wait wherever they are until moved to one. (The legacy
            // founded start has no soil system: any empty cell hatches.)
            let on_orange = pos.layer == Layer::Underground
                && (!self.colony.founding
                    || self.soil_at(
                        Layer::Underground,
                        pos.p.x.floor() as u32,
                        pos.p.y.floor() as u32,
                    ) == SOIL_ORANGE);
            if hatch <= 0.0
                && carried.is_none()
                && on_orange
                && self.colony.ant_count < self.config.max_ants
            {
                let p = pos.p;
                self.ev(format!("egg #{id} hatched on orange at ({:.0},{:.0})", p.x, p.y));
                self.kill(id);
                self.spawn_ant(caste, p, Layer::Underground);
                // the first hatched worker ends the founding script
                if self.colony.phase == Phase::Brood && self.caste_counts().0 > 0 {
                    self.colony.phase = Phase::Colony;
                    self.ev("first worker hatched — colony phase".to_string());
                }
            } else if hatch <= 0.0 && hatch_prev > 0.0 && carried.is_none() && !on_orange {
                if let Ok(mut q) = self.ecs.get::<&mut Egg>(ent) {
                    q.hatch = 0.0;
                }
                self.ev(format!("egg #{id} ready — waiting for orange soil"));
            } else if let Ok(mut q) = self.ecs.get::<&mut Egg>(ent) {
                q.hatch = hatch.max(0.0);
            }
        }
    }

    pub(crate) fn cleanup(&mut self) {
        let ids = self.food_ids();
        for id in ids {
            let ent = self.ids[&id];
            let (amount, spoil, pos, harvest_t, src) = match self
                .ecs
                .query_one::<(&Food, &Pos)>(ent)
                .unwrap()
                .get()
            {
                Some(q) => (q.0.amount, q.0.spoil, *q.1, q.0.harvest_t, q.0.src),
                None => continue,
            };
            if amount == 0 {
                self.colony.known.remove(&id);
                if harvest_t > 0.0 {
                    self.ev(format!("source depleted: {} #{}", source_name(src), id));
                }
                self.kill(id);
                continue;
            }
            // dropped food spoils on non-silver cells; silver freezes the timer
            if let Some(mut t) = spoil {
                let tile = (pos.p.x.floor() as u32, pos.p.y.floor() as u32);
                let silver = self.soil_at(pos.layer, tile.0, tile.1) == SOIL_SILVER;
                if silver {
                    if let Ok(mut q) = self.ecs.get::<&mut Food>(ent) {
                        q.spoil = Some(t);
                    }
                    continue;
                }
                t -= DT;
                if t <= 0.0 {
                    self.colony.known.remove(&id);
                    self.ev(format!("food #{id} spoiled ({} units lost)", amount));
                    self.kill(id);
                } else if let Ok(mut q) = self.ecs.get::<&mut Food>(ent) {
                    q.spoil = Some(t);
                }
            }
        }
    }
}
