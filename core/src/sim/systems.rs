//! The per-tick systems, in the exact order `Sim::tick` runs them. Order is
//! part of determinism: reordering changes every seed's future.

use super::{snapshot::source_name, Phase, Sim, DT};
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
                self.rules.queen_fly_speed
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

    /// Advance every harvester: shared progress lives on the Food entity;
    /// a completed unit fills the mandibles and the ant returns to Idle
    /// (the forage/deliver loop re-plans on the next ai tick).
    pub(crate) fn harvesting(&mut self) {
        let ids = self.ant_ids();
        for id in ids {
            let Some(&ent) = self.ids.get(&id) else {
                continue;
            };
            let state = match self.ecs.get::<&AntState>(ent) {
                Ok(q) => (*q).clone(),
                Err(_) => continue,
            };
            let AntState::Harvesting { target } = state else {
                continue;
            };
            let Some(&fent) = self.ids.get(&target) else {
                self.set_state(id, AntState::Idle);
                continue;
            };
            let amount = self.ecs.get::<&Food>(fent).map(|f| f.amount).unwrap_or(0);
            if amount == 0 {
                self.set_state(id, AntState::Idle);
                continue;
            }
            if let Some(kind) = self.advance_harvest(fent) {
                if let Ok(mut q) = self.ecs.get::<&mut Carry>(ent) {
                    *q = Carry::Food(kind);
                }
                self.set_state(id, AntState::Idle);
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
                self.rules.queen_dig_time
            } else {
                self.rules.dig_time
            };
            progress += DT;
            if progress >= dig_time {
                for dy in 0..2u32 {
                    for dx in 0..2u32 {
                        self.world.underground.set(tx + dx, ty + dy, EMPTY);
                    }
                }
                self.tiles_epoch += 1;
                self.dug_tiles += 4;
                // excavated dirt is carried out by every digging caste (up
                // to the digger's per-caste dirt_capacity blocks before
                // dumping) — workers haul spoil to the surface, the
                // founding queen refills or dumps
                if let Ok(mut q) = self.ecs.get::<&mut Carry>(ent) {
                    let blocks = match *q {
                        Carry::Dirt { blocks } => blocks,
                        _ => 0,
                    };
                    *q = Carry::Dirt {
                        blocks: (blocks + 1).min(self.rules.stats_for(caste).dirt_capacity),
                    };
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
                self.end_fight(id);
                continue;
            };
            let tpos = self.ecs.get::<&Pos>(tent).map(|q| *q);
            let tpos = match tpos {
                Ok(t) => t,
                Err(_) => {
                    self.end_fight(id);
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
            if dist > self.rules.ant_range {
                if pos.layer == Layer::Underground {
                    // below ground a straight line would glide through dirt
                    // and rock: route instead. The Fighting state ends here;
                    // attack_after re-engages when the walk arrives.
                    if !self.route(id, pos.layer, tile_of(tpos.p)) {
                        self.set_state(id, AntState::Idle);
                        self.set_attack_after(id, None);
                        self.set_retry(id, 60);
                    }
                    continue;
                }
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
                    .unwrap_or(false)
                    && self
                        .ids
                        .get(&t)
                        .map(|&te| self.ecs.get::<&Fallen>(te).is_err())
                        .unwrap_or(false); // downed ants are not prey
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
                        if ap.layer != Layer::Surface || self.ecs.get::<&Fallen>(aent).is_ok() {
                            continue;
                        }
                        let d = (ap.p - pos.p).len();
                        if d <= self.rules.spider_aggro
                            && best.map(|(bd, _)| d < bd).unwrap_or(true)
                        {
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
                    if dist > self.rules.spider.range {
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
                            // retaliation (F4): an attacked leader's squad
                            // turns on the attacker — same shared-target
                            // semantics as the leader's own attack order
                            self.retaliate(t, pid);
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
                        let dist = self.rng.range(0.0, self.rules.spider_wander);
                        let hx = pred.home.0 as f64 + 0.5 + dx / len * dist;
                        let hy = pred.home.1 as f64 + 0.5 + dy / len * dist;
                        pred.dest = Some((
                            hx.clamp(1.0, self.rules.width as f64 - 2.0),
                            hy.clamp(1.0, self.rules.height as f64 - 2.0),
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
                // downed ants already sit at hp 0 — they bleed out in
                // `specialists`, not here
                self.ids.get(&id).is_some_and(|&e| {
                    self.ecs.get::<&Combat>(e).is_ok_and(|c| c.hp <= 0.0)
                        && self.ecs.get::<&Fallen>(e).is_err()
                })
            })
            .collect();
        for id in dead_ants {
            if id == self.colony.queen_id {
                // the queen never downs — her death ends the colony
                self.kill_cause(id, "combat");
            } else {
                self.fall_ant(id);
            }
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
            let drop = self.rules.protein_per_spider;
            self.ev(format!("spider #{pid} died → {drop} protein"));
            self.kill(pid);
            self.spawn_food(p, self.rules.protein_per_spider, FoodKind::Protein);
        }
    }

    pub(crate) fn queen_system(&mut self) {
        if self.colony.dead {
            return;
        }
        // founding quest: once the queen has stored the ritual water on
        // food blocks, standing on an egg block (orange soil) lays the
        // first brood — no timer, she digs and searches as long as she
        // needs. The eggs prefer the orange cells she is standing on so
        // they can hatch without being carried anywhere.
        if self.colony.phase == Phase::Founding
            && self.colony.quest_water_tally >= self.rules.founding_quest_water
        {
            let qt = self.queen_tile();
            if self.colony.founding && self.soil_at(Layer::Underground, qt.0, qt.1) == SOIL_ORANGE {
                self.colony.phase = Phase::Brood;
                let n = self.rules.founding_eggs;
                self.ev(format!(
                    "founding brood laid on the egg block — {n} eggs incubating"
                ));
                for _ in 0..n {
                    let Some(tile) = self.free_egg_tile() else {
                        break;
                    };
                    self.colony.eggs_laid += 1;
                    self.spawn_egg(
                        tile_center(tile.0, tile.1),
                        Caste::Worker,
                        self.rules.founding_egg_hatch,
                    );
                }
            }
        }
        self.colony.lay_cooldown -= DT;
        if self.colony.founding {
            // Physical feeding (worker-priorities wave): the queen eats real
            // units the feeder delivers (or her own mandibles' load). No
            // automatic store drain — the store counts the pantry, withdraw
            // removes from it. Hungry at self.rules.eat_period, dead at + self.rules.starve_time.
            // The lone-queen grace still freezes hunger: no workers → no
            // feeder, and she may not have farmed anything yet.
            let grace = self.caste_counts().0 == 0;
            if !grace {
                self.colony.hunger_t += DT;
                let hungry_at = self.rules.eat_period;
                let prev = self.colony.hunger_t - DT;
                if self.colony.hunger_t >= hungry_at && prev < hungry_at {
                    self.ev(format!(
                        "queen is hungry — wants {} (no delivery)",
                        super::snapshot::food_name(self.rules.craving_cycle[self.colony.craving_i])
                    ));
                }
                if self.colony.hunger_t >= self.rules.eat_period + self.rules.starve_time * 0.5
                    && prev < self.rules.eat_period + self.rules.starve_time * 0.5
                {
                    self.ev("queen is STARVING".to_string());
                }
                if self.colony.hunger_t >= self.rules.eat_period + self.rules.starve_time {
                    self.colony.dead = true;
                    let q = self.colony.queen_id;
                    self.kill_cause(q, "starvation");
                    return;
                }
            }
            return;
        }
        // Legacy founded economy: abstract drain from the carb store.
        if self.colony.carbs > 0 {
            self.colony.eat_t += DT;
            if self.colony.eat_t >= self.rules.eat_period {
                self.colony.carbs -= 1;
                self.colony.eat_t = 0.0;
            }
            self.colony.starve_t = 0.0;
        } else {
            self.colony.eat_t = 0.0;
            self.colony.starve_t += DT;
            if self.colony.starve_t >= self.rules.starve_time {
                self.colony.dead = true;
                let q = self.colony.queen_id;
                self.kill_cause(q, "starvation");
                return;
            }
        }
        // ongoing auto-laying is legacy-mode only: in a founding game the
        // brood comes from the founding script (production redesign is a
        // later wave)
        if self.colony.lay_cooldown <= 0.0 && self.colony.ant_count < self.rules.max_ants {
            let (workers, soldiers) = self.caste_counts();
            let want = if self.colony.carbs >= self.rules.soldier_cost_green
                && self.colony.protein >= self.rules.soldier_cost_super
                && soldiers * 2 < workers
            {
                Some(Caste::Soldier)
            } else if self.colony.carbs >= self.rules.egg_cost {
                Some(Caste::Worker)
            } else {
                None
            };
            if let Some(caste) = want {
                match self.free_egg_tile() {
                    Some(tile) => {
                        match caste {
                            Caste::Soldier => {
                                self.colony.carbs -= self.rules.soldier_cost_green;
                                self.colony.protein -= self.rules.soldier_cost_super;
                            }
                            Caste::Worker => self.colony.carbs -= self.rules.egg_cost,
                            // the legacy auto-laying above only ever picks
                            // Worker/Soldier — the F4 castes are queen-ordered
                            Caste::Queen | Caste::Honey | Caste::Medic => {}
                        }
                        self.colony.eggs_laid += 1;
                        self.colony.lay_cooldown = self.rules.lay_cooldown;
                        self.spawn_egg(tile_center(tile.0, tile.1), caste, self.rules.egg_time);
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
            let carried_by = self.ecs.get::<&Egg>(eent).ok().and_then(|e| e.carried_by);
            let Some(carrier) = carried_by else {
                continue;
            };
            let cpos = self
                .ids
                .get(&carrier)
                .and_then(|&ce| self.ecs.get::<&Pos>(ce).ok().map(|p| (p.p, p.layer)));
            match cpos {
                Some((p, layer)) => {
                    if let Some(q) = self.ecs.query_one::<(&mut Pos,)>(eent).unwrap().get() {
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
            let (hatch, caste, carried, pos) =
                match self.ecs.query_one::<(&Egg, &Pos)>(ent).unwrap().get() {
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
                && self.colony.ant_count < self.rules.max_ants
            {
                let p = pos.p;
                self.ev(format!(
                    "egg #{id} hatched on orange at ({:.0},{:.0})",
                    p.x, p.y
                ));
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

    /// Specialist-caste systems (F4): honey secretion, the downed-ant
    /// bleed/heal cycle. Runs after queen_system, before eggs.
    pub(crate) fn specialists(&mut self) {
        // Honey ants secrete one honeydew unit per honey_period where they
        // stand: on a silver pantry cell it banks (ledger + pile, like any
        // delivery), anywhere else it drops as a spoiling loose unit that
        // workers must haul home.
        let honeys: Vec<u32> = self
            .ant_ids()
            .into_iter()
            .filter(|&id| {
                self.ids
                    .get(&id)
                    .map(|&e| {
                        self.ecs
                            .get::<&Ant>(e)
                            .is_ok_and(|a| a.caste == Caste::Honey)
                    })
                    .unwrap_or(false)
            })
            .collect();
        for id in honeys {
            let Some(&ent) = self.ids.get(&id) else {
                continue;
            };
            let (gen_t, pos) = match self
                .ecs
                .query_one::<(&mut Ant, &Pos)>(ent)
                .ok()
                .and_then(|mut q| q.get().map(|(a, p)| (a.gen_t, *p)))
            {
                Some(v) => v,
                None => continue,
            };
            let gen_t = gen_t + DT;
            if gen_t < self.rules.honey_period {
                if let Ok(mut a) = self.ecs.get::<&mut Ant>(ent) {
                    a.gen_t = gen_t;
                }
                continue;
            }
            if let Ok(mut a) = self.ecs.get::<&mut Ant>(ent) {
                a.gen_t = 0.0;
            }
            let tile = crate::path::tile_of(pos.p);
            let on_silver = pos.layer == Layer::Underground
                && self.soil_at(Layer::Underground, tile.0, tile.1) == SOIL_SILVER;
            if on_silver && self.block_room(Layer::Underground, tile, FoodKind::Honeydew) > 0 {
                self.colony.honeydew += 1;
                self.colony.delivered += 1;
                self.spawn_unit_food(Layer::Underground, tile, FoodKind::Honeydew, None);
                // spawn_unit_food marks pantry joins stored; force the fresh
                // pile stored too so the ledger and the pile agree
                for fid in self.food_on(Layer::Underground, tile) {
                    let fent = self.ids[&fid];
                    if let Ok(mut q) = self.ecs.get::<&mut Food>(fent) {
                        if q.kind == FoodKind::Honeydew {
                            q.stored = true;
                            q.spoil = None;
                        }
                    }
                }
                self.ev(format!(
                    "honey ant #{id} banked 1 honeydew at ({},{})",
                    tile.0, tile.1
                ));
            } else if self.spawn_unit_food(
                pos.layer,
                tile,
                FoodKind::Honeydew,
                Some(self.rules.spoil_time),
            ) {
                self.ev(format!(
                    "honey ant #{id} secreted 1 honeydew at ({},{}) — haul it home",
                    tile.0, tile.1
                ));
            } else {
                // the block here can't take honeydew (full or typed to
                // another kind) — the secretion is lost
                self.ev(format!(
                    "honey ant #{id} spilled 1 honeydew at ({},{}) — no room",
                    tile.0, tile.1
                ));
            }
        }
        // downed ants ride their medic (like eggs); a dead medic drops
        // them where they lie — the carrier check below clears the carry on
        // death, so a stale carried_by can't outlive the medic
        let fallen: Vec<u32> = self
            .ids
            .iter()
            .filter_map(|(&id, &ent)| self.ecs.get::<&Fallen>(ent).ok().map(|_| id))
            .collect();
        for &fid in &fallen {
            let Some(&fent) = self.ids.get(&fid) else {
                continue;
            };
            let carrier = self
                .ecs
                .get::<&Fallen>(fent)
                .ok()
                .and_then(|f| f.carried_by);
            let Some(m) = carrier else { continue };
            let mpos = self.ids.get(&m).and_then(|&ment| {
                self.ecs
                    .query_one::<&Pos>(ment)
                    .ok()
                    .and_then(|mut q| q.get().copied())
            });
            match mpos {
                Some(p) => {
                    if let Some(q) = self.ecs.query_one::<(&mut Pos,)>(fent).unwrap().get() {
                        q.0.p = p.p;
                        q.0.layer = p.layer;
                    }
                }
                None => {
                    // the medic died mid-carry: drop the patient in place
                    if let Ok(mut q) = self.ecs.get::<&mut Fallen>(fent) {
                        q.carried_by = None;
                    }
                }
            }
        }
        for fid in fallen {
            let Some(&ent) = self.ids.get(&fid) else {
                continue;
            };
            let Some(f) = self
                .ecs
                .query_one::<&mut Fallen>(ent)
                .ok()
                .and_then(|mut q| q.get().map(|f| *f))
            else {
                continue;
            };
            if let Some(heal_t) = f.heal_t {
                let left = heal_t - DT;
                if left <= 0.0 {
                    // back on its feet: full hp, resumes its old job
                    let _ = self.ecs.remove_one::<Fallen>(ent);
                    if let Ok(mut c) = self.ecs.get::<&mut Combat>(ent) {
                        c.hp = c.max_hp;
                    }
                    self.ev(format!("medic healed ant #{fid} — back on its feet"));
                } else if let Ok(mut q) = self.ecs.get::<&mut Fallen>(ent) {
                    q.heal_t = Some(left);
                }
            } else {
                let bleed = f.bleed_t - DT;
                if bleed <= 0.0 {
                    self.kill_cause(fid, "bled out");
                } else if let Ok(mut q) = self.ecs.get::<&mut Fallen>(ent) {
                    q.bleed_t = bleed;
                }
            }
        }
    }

    /// A caste ant drops to the ground instead of dying: downed, bleeding,
    /// waiting for a medic — carried items fall beside it first.
    pub(crate) fn fall_ant(&mut self, id: u32) {
        let Some(&ent) = self.ids.get(&id) else {
            return;
        };
        let carry = self
            .ecs
            .get::<&Carry>(ent)
            .map(|c| *c)
            .unwrap_or(Carry::None);
        if carry != Carry::None {
            let pos = self.ecs.get::<&Pos>(ent).map(|q| *q).unwrap_or(Pos {
                p: Vec2::default(),
                layer: Layer::Surface,
            });
            let tile = crate::path::tile_of(pos.p);
            match carry {
                Carry::Egg => {
                    for eid in self.egg_ids() {
                        let eent = self.ids[&eid];
                        let mine = self
                            .ecs
                            .get::<&Egg>(eent)
                            .map(|q| q.carried_by == Some(id))
                            .unwrap_or(false);
                        if mine {
                            if let Ok(mut q) = self.ecs.get::<&mut Egg>(eent) {
                                q.carried_by = None;
                            }
                        }
                    }
                }
                Carry::Food(kind) => {
                    // a falling carrier spills its food where it drops —
                    // lost when the block there has no room for it
                    if !self.spawn_unit_food(pos.layer, tile, kind, Some(self.rules.spoil_time)) {
                        self.ev(format!(
                            "ant #{id} dropped 1 {} — no room, spilled",
                            super::snapshot::food_name(kind)
                        ));
                    }
                }
                Carry::Wood => {
                    self.spawn_collectible(tile_center(tile.0, tile.1), CollectibleVariant::Wood);
                }
                Carry::Wool => {
                    self.spawn_collectible(tile_center(tile.0, tile.1), CollectibleVariant::Wool);
                }
                _ => {} // dirt vanishes with the collapse
            }
            if let Ok(mut q) = self.ecs.get::<&mut Carry>(ent) {
                *q = Carry::None;
            }
        }
        self.set_state(id, AntState::Idle);
        let bleed = self.rules.bleed_time;
        let _ = self.ecs.insert_one(
            ent,
            Fallen {
                bleed_t: bleed,
                heal_t: None,
                carried_by: None,
            },
        );
        self.ev(format!(
            "ant #{id} is DOWN — bleeds out in {bleed:.0}s without a medic"
        ));
    }

    /// The victim's squad turns on its attacker (F4 retaliation).
    fn retaliate(&mut self, victim: u32, attacker: u32) {
        let ids = self.ant_ids();
        for fid in ids {
            let follows = self
                .ids
                .get(&fid)
                .and_then(|&e| self.ecs.get::<&WorkerAi>(e).ok())
                .map(|ai| {
                    matches!(&ai.job, Job::Follow(l, _) if *l == victim)
                        && ai.attack_after != Some(attacker)
                })
                .unwrap_or(false);
            if follows {
                self.set_attack_after(fid, Some(attacker));
                self.ev(format!(
                    "ant #{fid} retaliates — spider #{attacker} hit leader #{victim}"
                ));
            }
        }
    }

    pub(crate) fn cleanup(&mut self) {
        let ids = self.food_ids();
        for id in ids {
            let ent = self.ids[&id];
            let (amount, spoil, pos, harvest_t, src) =
                match self.ecs.query_one::<(&Food, &Pos)>(ent).unwrap().get() {
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
