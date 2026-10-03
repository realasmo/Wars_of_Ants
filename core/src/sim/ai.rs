//! Worker and queen AI: job selection, walk-to-act intent resolution,
//! foraging/pantry logistics decisions, scouting and source discovery.

use super::{Phase, Sim};
use crate::components::*;
use crate::path::{chebyshev, manhattan, tile_of};
use crate::world::{block_of, EMPTY, SOIL_ORANGE, SOIL_SILVER};

impl Sim {
    pub(crate) fn worker_ai(&mut self) {
        if self.colony.dead {
            return;
        }
        self.designate_feeder();
        let ids = self.ant_ids();
        for id in ids {
            let ent = match self.ids.get(&id) {
                Some(&e) => e,
                None => continue,
            };
            #[allow(clippy::type_complexity)]
            let (
                caste,
                pos,
                state,
                carrying,
                job,
                pending,
                retry,
                attack_after,
                dig_after,
                drop_after,
                pick_after,
                land_after,
                found_after,
            ) = {
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
                    q.4.job.clone(),
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
            // A downed ant lies still until a medic hauls it home (F4).
            if self.ecs.get::<&Fallen>(ent).is_ok() {
                continue;
            }
            // The queen is player-driven: she never takes forage/dig jobs and
            // never auto-picks-up food — only Attack orders and walk-to-dig
            // intents move her.
            if caste == Caste::Queen {
                if !matches!(state, AntState::Idle) {
                    continue;
                }
                // the queen shares the worker backoff: decrement it, or any
                // set_retry would freeze her walk-to-act intents forever
                if retry > 0 {
                    self.set_retry(id, retry - 1);
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
                    // same semantics as the worker branch: a successful route
                    // KEEPS the intent — the order re-resolves (and engages)
                    // when the walk arrives. Clearing here made the queen
                    // surface, stand where the spider used to be, and die.
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
                            if tlayer == pos.layer {
                                self.set_state(id, AntState::Fighting { target: tid });
                                true
                            } else {
                                self.route(id, tlayer, ttile)
                            }
                        })
                        .unwrap_or(true);
                    if !ok {
                        self.set_attack_after(id, None);
                        self.set_retry(id, 60);
                    }
                    continue;
                }
                // hungry and holding the craved resource: eat straight from
                // the mandibles (the solo queen farms her own survival; the
                // fed queen can top herself up when the feeder is absent)
                if self.colony.hunger_t >= self.rules.eat_period {
                    if let Carry::Food(k) = carrying {
                        if k == self.craving() {
                            if let Some(&aent) = self.ids.get(&id) {
                                if let Ok(mut q) = self.ecs.get::<&mut Carry>(aent) {
                                    *q = Carry::None;
                                }
                            }
                            let who = "queen".to_string();
                            self.queen_fed(who, k);
                            continue;
                        }
                    }
                }
                // the queen is player-driven and never takes forage jobs,
                // but she may farm like any ant: stand on food with free
                // mandibles → visibly work it; a full haul banks on silver
                // soil (pantry cells) when she walks over one
                if carrying == Carry::None && pos.layer == Layer::Surface {
                    if let Some((cid, variant)) = self.collectible_on_tile(tile_of(pos.p)) {
                        let carry = self.take_collectible(cid, variant);
                        if let Some(&aent) = self.ids.get(&id) {
                            if let Ok(mut q) = self.ecs.get::<&mut Carry>(aent) {
                                *q = carry;
                            }
                        }
                    } else if let Some(fid) = self.food_on_tile(tile_of(pos.p)) {
                        self.set_state(id, AntState::Harvesting { target: fid });
                    }
                } else if pos.layer == Layer::Underground && matches!(carrying, Carry::Food(_)) {
                    let tile = tile_of(pos.p);
                    if let Carry::Food(kind) = carrying {
                        if self.soil_at(Layer::Underground, tile.0, tile.1) == SOIL_SILVER
                            && self.block_room(Layer::Underground, tile, kind) > 0
                        {
                            self.store_food(id, tile);
                        }
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
                    // same-layer: engage now. attack_after deliberately stays
                    // set — combat() may re-route the chase (underground) and
                    // the intent re-resolves on arrival; it clears when the
                    // target dies or the route backs off.
                    Some((tlayer, _ttile)) if tlayer == pos.layer => {
                        self.set_pending(id, None);
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
                if matches!(self.carry_of(id), Carry::Dirt { blocks } if blocks
                    == self.rules.stats_for(caste).dirt_capacity)
                {
                    // hands full of spoil: haul it out before digging more
                    self.set_dig_after(id, None);
                    self.haul_out_dirt(id);
                } else if !self.block_soft(bx, by) {
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
            // Medic specialists (F4): a downed ant in the mandibles always
            // resumes healing (player orders may have interrupted it); an
            // unoccupied medic drops EVERYTHING for the nearest casualty —
            // Fetch/Deliver/DigTile included (GoHome and Rest hand out those
            // jobs, so the override must catch them too, or medics farm).
            if caste == Caste::Medic {
                if let Carry::Fallen = carrying {
                    if !matches!(job, Job::Heal(_)) {
                        if let Some(fid) = self.carried_fallen(id) {
                            self.set_job(id, Job::Heal(fid));
                        }
                    }
                } else {
                    // finish banking a carried food unit; player orders and
                    // active rescue/heal jobs stand; everything else is
                    // overridable
                    let overridable = match &job {
                        Job::Manual
                        | Job::Follow(..)
                        | Job::Rescue(_)
                        | Job::Heal(_)
                        | Job::Hold => false,
                        Job::Deliver(_, _) => matches!(carrying, Carry::Food(_)),
                        _ => true,
                    };
                    if overridable {
                        match self.nearest_fallen() {
                            Some(fid) => {
                                self.set_job(id, Job::Rescue(fid));
                                continue;
                            }
                            None => {
                                self.set_job(id, Job::Hold);
                                continue;
                            }
                        }
                    }
                }
            }
            match job {
                Job::Manual => {
                    if pos.layer == Layer::Surface && carrying == Carry::None {
                        // collectibles (wood/wool) are instant pickups
                        if let Some((cid, variant)) = self.collectible_on_tile(tile_of(pos.p)) {
                            let carry = self.take_collectible(cid, variant);
                            if let Some(&aent) = self.ids.get(&id) {
                                if let Ok(mut q) = self.ecs.get::<&mut Carry>(aent) {
                                    *q = carry;
                                }
                            }
                            continue;
                        }
                        // stand on food → visibly work it (Harvesting drives
                        // the mandible animation and fills hands on pickup)
                        if let Some(fid) = self.food_on_tile(tile_of(pos.p)) {
                            self.set_state(id, AntState::Harvesting { target: fid });
                        }
                    } else if matches!(carrying, Carry::Dirt { .. }) {
                        // a spoil-laden manual worker can't harvest (hands
                        // gate) and its route may have been interrupted by
                        // auto-digging — nothing is actionable until the
                        // dirt is dumped, so haul it out automatically;
                        // farming resumes clean-handed on the next click
                        self.haul_out_dirt(id);
                    } else if pos.layer == Layer::Underground
                        && matches!(carrying, Carry::Food(FoodKind::Green | FoodKind::Super))
                    {
                        // bank carried food: on silver cells, or beside the
                        // queen when no pantry cell is handy
                        let tile = tile_of(pos.p);
                        let on_silver =
                            self.soil_at(Layer::Underground, tile.0, tile.1) == SOIL_SILVER;
                        let near_queen = chebyshev(tile, self.queen_tile()) <= 1;
                        if let Carry::Food(kind) = carrying {
                            if (on_silver || near_queen)
                                && self.block_room(Layer::Underground, tile, kind) > 0
                            {
                                self.store_food(id, tile);
                            }
                        }
                    }
                }
                Job::Idle => {
                    if carrying != Carry::None {
                        if matches!(carrying, Carry::Dirt { .. }) {
                            // spoil goes to the surface, never into the nest
                            self.haul_out_dirt(id);
                        } else if let Carry::Food(kind) = carrying {
                            match self.pantry_tile(kind) {
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
                        }
                    } else if let Some(t) = self.colony.dig_queue.pop() {
                        self.set_job(id, Job::DigTile(t.0, t.1));
                    } else if let Some(fid) = self.best_food() {
                        self.set_job(id, Job::Fetch(fid));
                    } else {
                        // nothing known to farm: drift where we stand for a
                        // while, then head home and rest (auto-scouting is
                        // gone — far sources are found by the player)
                        self.set_job(id, Job::Loiter(self.rules.loiter_hops));
                    }
                }
                Job::Feed => {
                    if matches!(carrying, Carry::Dirt { .. }) {
                        // spoil picked up en route must go before anything else
                        self.haul_out_dirt(id);
                        continue;
                    }
                    let craving = self.craving();
                    let hungry = self.colony.hunger_t >= self.rules.eat_period;
                    match carrying {
                        Carry::Food(k) if k == craving => {
                            let (qlayer, qtile) = self.queen_where();
                            if qlayer == pos.layer && chebyshev(tile_of(pos.p), qtile) <= 1 {
                                if hungry {
                                    self.feed_queen(id);
                                } else {
                                    // meal ready — stand by her until hunger hits
                                    self.set_retry(id, 50);
                                }
                            } else if !self.route(id, qlayer, qtile) {
                                self.set_retry(id, 60);
                            }
                        }
                        Carry::Food(kind) => {
                            // craving moved on (the queen self-fed): re-bank
                            // this unit, then fetch the new craving
                            if pos.layer == Layer::Underground {
                                let tile = tile_of(pos.p);
                                let on_silver =
                                    self.soil_at(Layer::Underground, tile.0, tile.1) == SOIL_SILVER;
                                let near_queen = chebyshev(tile, self.queen_tile()) <= 1;
                                if (on_silver || near_queen)
                                    && self.block_room(Layer::Underground, tile, kind) > 0
                                {
                                    self.store_food(id, tile);
                                } else if let Some((px, py)) = self.pantry_tile(kind) {
                                    if tile != (px, py)
                                        && !self.route(id, Layer::Underground, (px, py))
                                    {
                                        self.set_retry(id, 60);
                                    }
                                } else {
                                    self.set_retry(id, 100);
                                }
                            } else if let Some((px, py)) = self.pantry_tile(kind) {
                                if !self.route(id, Layer::Underground, (px, py)) {
                                    self.set_retry(id, 60);
                                }
                            } else {
                                self.set_retry(id, 100);
                            }
                        }
                        Carry::None => match self.craving_pile() {
                            Some((fid, ftile)) => {
                                if pos.layer == Layer::Underground && tile_of(pos.p) == ftile {
                                    if !self.withdraw_pantry(id, fid) {
                                        self.set_retry(id, 60);
                                    }
                                } else if !self.route(id, Layer::Underground, ftile) {
                                    self.set_retry(id, 60);
                                }
                            }
                            None => {
                                // pantry can't satisfy the craving: wait
                                // beside the queen — her request display is
                                // the player-facing signal to farm it
                                let (qlayer, qtile) = self.queen_where();
                                let near =
                                    qlayer == pos.layer && chebyshev(tile_of(pos.p), qtile) <= 2;
                                if !near && !self.route(id, qlayer, qtile) {
                                    self.set_retry(id, 60);
                                }
                            }
                        },
                        // eggs/wood/wool are unreachable for a feeding worker
                        // (only player commands put them in worker hands, and
                        // that takes the ant out of the feeder role) — park
                        // safely rather than act on full hands
                        _ => {
                            self.set_retry(id, 100);
                        }
                    }
                }
                Job::Loiter(hops) => {
                    if matches!(carrying, Carry::Dirt { .. }) {
                        self.haul_out_dirt(id);
                        continue;
                    }
                    if let Some(fid) = self.best_food() {
                        // work appeared while drifting — take it
                        self.set_job(id, Job::Fetch(fid));
                        continue;
                    }
                    if hops == 0 {
                        self.set_job(id, Job::GoHome);
                        continue;
                    }
                    // one short hop near the current spot, then pause a beat
                    let (cx, cy) = (tile_of(pos.p).0 as i32, tile_of(pos.p).1 as i32);
                    let r = self.rules.loiter_radius as i32;
                    let dx = self.rng.irange(0, 2 * self.rules.loiter_radius) as i32 - r;
                    let dy = self.rng.irange(0, 2 * self.rules.loiter_radius) as i32 - r;
                    let tx = (cx + dx).clamp(1, self.rules.width as i32 - 2) as u32;
                    let ty = (cy + dy).clamp(1, self.rules.height as i32 - 2) as u32;
                    self.set_job(id, Job::Loiter(hops - 1));
                    if (tx, ty) == (cx as u32, cy as u32) {
                        self.set_retry(id, 20);
                    } else if !self.route(id, pos.layer, (tx, ty)) {
                        // can't wander there — head home instead
                        self.set_job(id, Job::GoHome);
                    }
                }
                Job::GoHome => {
                    if matches!(carrying, Carry::Dirt { .. }) {
                        self.haul_out_dirt(id);
                        continue;
                    }
                    if let Some(fid) = self.best_food() {
                        self.set_job(id, Job::Fetch(fid));
                        continue;
                    }
                    if pos.layer == Layer::Underground {
                        // home: rest a stretch before the next glance outside
                        self.set_job(id, Job::Rest(self.rules.home_rest_ticks));
                    } else if let Some((ex, ey)) = self.world.entrance {
                        if !self.route(id, Layer::Underground, (ex + 1, ey + 3)) {
                            self.set_retry(id, 60);
                        }
                    } else {
                        self.set_retry(id, 100);
                    }
                }
                Job::GoOut => {
                    if matches!(carrying, Carry::Dirt { .. }) {
                        self.haul_out_dirt(id);
                        continue;
                    }
                    if let Some(fid) = self.best_food() {
                        self.set_job(id, Job::Fetch(fid));
                        continue;
                    }
                    // pop out for a look around the nest mouth — this is how
                    // near-nest sources stay discoverable without scouting
                    if let Some((ex, ey)) = self.world.entrance {
                        let r = self.rules.loiter_radius as i32;
                        let dx = self.rng.irange(0, 2 * self.rules.loiter_radius) as i32 - r;
                        let dy = self.rng.irange(0, 2 * self.rules.loiter_radius) as i32 - r;
                        let tx = (ex as i32 + 1 + dx).clamp(1, self.rules.width as i32 - 2) as u32;
                        let ty = (ey as i32 + 1 + dy).clamp(1, self.rules.height as i32 - 2) as u32;
                        if pos.layer == Layer::Surface
                            && chebyshev(tile_of(pos.p), (ex, ey)) <= self.rules.loiter_radius + 2
                        {
                            self.set_job(id, Job::Loiter(self.rules.loiter_hops));
                        } else if !self.route(id, Layer::Surface, (tx, ty)) {
                            self.set_retry(id, 60);
                        }
                    } else {
                        self.set_retry(id, 100);
                    }
                }
                Job::Rest(t) => {
                    if let Some(fid) = self.best_food() {
                        self.set_job(id, Job::Fetch(fid));
                        continue;
                    }
                    if t <= 1 {
                        self.set_job(id, Job::GoOut);
                    } else {
                        self.set_job(id, Job::Rest(t - 1));
                    }
                }
                Job::DigTile(tx, ty) => {
                    if matches!(carrying, Carry::Dirt { blocks } if blocks
                        == self.rules.stats_for(caste).dirt_capacity)
                    {
                        // hands full of spoil: haul it out, then come back
                        self.haul_out_dirt(id);
                        continue;
                    }
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
                Job::Hold => {
                    // stand by near the queen — but a casualty beats standby
                    if let Some(fid) = self.nearest_fallen() {
                        self.set_job(id, Job::Rescue(fid));
                        continue;
                    }
                    let (qlayer, qtile) = self.queen_where();
                    if qlayer == pos.layer && chebyshev(tile_of(pos.p), qtile) <= 3 {
                        self.set_retry(id, 50);
                    } else if !self.route(id, qlayer, qtile) {
                        self.set_retry(id, 60);
                    }
                }
                Job::Rescue(fid) => {
                    let info = self.ids.get(&fid).and_then(|&fent| {
                        self.ecs
                            .query_one::<(&Fallen, &Pos)>(fent)
                            .ok()
                            .and_then(|mut q| q.get().map(|(f, p)| (*f, *p)))
                    });
                    let Some((fallen, fpos)) = info else {
                        self.set_job(id, Job::Idle);
                        continue;
                    };
                    if fallen.carried_by.is_some()
                        || fallen.heal_t.is_some()
                        || carrying != Carry::None
                    {
                        // another medic got there first, healing already, or
                        // the medic's hands filled up meanwhile
                        self.set_job(id, Job::Idle);
                        continue;
                    }
                    if fpos.layer == pos.layer && chebyshev(tile_of(pos.p), tile_of(fpos.p)) <= 1 {
                        if let Some(&fent) = self.ids.get(&fid) {
                            if let Ok(mut q) = self.ecs.get::<&mut Fallen>(fent) {
                                q.carried_by = Some(id);
                            }
                        }
                        if let Some(&aent) = self.ids.get(&id) {
                            if let Ok(mut q) = self.ecs.get::<&mut Carry>(aent) {
                                *q = Carry::Fallen;
                            }
                        }
                        self.set_job(id, Job::Heal(fid));
                        self.ev(format!("medic #{id} picked up downed ant #{fid}"));
                    } else if !self.route(id, fpos.layer, tile_of(fpos.p)) {
                        self.set_retry(id, 60);
                    }
                }
                Job::Heal(fid) => {
                    let info = self.ids.get(&fid).and_then(|&fent| {
                        self.ecs
                            .query_one::<(&Fallen, &Pos)>(fent)
                            .ok()
                            .and_then(|mut q| q.get().map(|(f, p)| (*f, *p)))
                    });
                    let Some((fallen, fpos)) = info else {
                        self.set_job(id, Job::Idle);
                        continue;
                    };
                    if let Carry::Fallen = carrying {
                        // phase 1 — haul the patient home: underground, by
                        // the queen
                        let (qlayer, qtile) = self.queen_where();
                        if pos.layer == Layer::Underground && chebyshev(tile_of(pos.p), qtile) <= 3
                        {
                            if let Some(&fent) = self.ids.get(&fid) {
                                if let Some(q) =
                                    self.ecs.query_one::<(&mut Pos,)>(fent).unwrap().get()
                                {
                                    q.0.p = pos.p;
                                    q.0.layer = pos.layer;
                                }
                                if let Ok(mut q) = self.ecs.get::<&mut Fallen>(fent) {
                                    q.carried_by = None;
                                }
                            }
                            if let Some(&aent) = self.ids.get(&id) {
                                if let Ok(mut q) = self.ecs.get::<&mut Carry>(aent) {
                                    *q = Carry::None;
                                }
                            }
                            self.ev(format!("medic #{id} set ant #{fid} down in the nest"));
                        } else if !self.route(id, qlayer, qtile) {
                            self.set_retry(id, 60);
                        }
                        continue;
                    }
                    let _ = fpos;
                    // phase 2 — standing by the patient: start the healing
                    // once the pantry can pay its water cost
                    if fallen.heal_t.is_none() {
                        let cost = self.rules.water_per_heal;
                        if self.pantry_units(FoodKind::Water) >= cost
                            && self.withdraw_pantry_units(FoodKind::Water, cost)
                        {
                            if let Some(&fent) = self.ids.get(&fid) {
                                if let Ok(mut q) = self.ecs.get::<&mut Fallen>(fent) {
                                    q.heal_t = Some(self.rules.heal_time);
                                }
                            }
                            self.ev(format!(
                                "medic #{id} starts healing ant #{fid} ({cost} water, {:.0}s)",
                                self.rules.heal_time
                            ));
                        } else {
                            // no water yet — wait; the bleed keeps running
                            self.set_retry(id, 60);
                        }
                    } else {
                        // specialists() revives the patient when heal_t ends
                        self.set_retry(id, 20);
                    }
                }
                Job::Follow(leader, resume) => {
                    // squad follow: stay near the leader (cross-layer via the
                    // entrance); manual commands naturally leave the squad
                    let Some(&lent) = self.ids.get(&leader) else {
                        // the leader is gone — back to the interrupted job
                        let back = resume.map(|b| *b).unwrap_or(Job::Idle);
                        self.set_job(id, back);
                        continue;
                    };
                    let linfo = {
                        let mut lq = self.ecs.query_one::<(&Pos, &AntState)>(lent).unwrap();
                        lq.get().map(|(lpos, _)| (lpos.layer, tile_of(lpos.p)))
                    };
                    let Some((llayer, ltile)) = linfo else {
                        self.set_job(id, Job::Idle);
                        continue;
                    };
                    if llayer == pos.layer {
                        if chebyshev(tile_of(pos.p), ltile) > 2 && !self.route(id, llayer, ltile) {
                            self.set_retry(id, 30);
                        }
                        // close enough: hold position with the squad
                    } else if !self.route(id, llayer, ltile) {
                        self.set_retry(id, 30);
                    }
                }
                Job::Fetch(fid) => {
                    if !self.ids.contains_key(&fid) {
                        self.set_job(id, Job::Idle);
                        continue;
                    }
                    if matches!(carrying, Carry::Dirt { .. }) {
                        // spoil picked up en route (auto-digging through soft
                        // tiles) must not be silently overwritten by the
                        // harvest — dump it, then the forage loop resumes
                        self.haul_out_dirt(id);
                        continue;
                    }
                    // unit already in the mandibles → hand off to delivery
                    // (pantry run, or dig out more nest when it's full)
                    if let Carry::Food(kind) = carrying {
                        match self.pantry_tile(kind) {
                            Some(t) => self.set_job(id, Job::Deliver(t.0, t.1)),
                            None => {
                                self.ev(format!("pantry full — ant #{id} digs expansion"));
                                match self.pick_dig_target() {
                                    Some(t) => self.set_job(id, Job::DigTile(t.0, t.1)),
                                    None => self.set_retry(id, 100),
                                }
                            }
                        }
                        continue;
                    }
                    let Some((ftile, amount)) = self.food_info(fid) else {
                        self.set_job(id, Job::Idle);
                        continue;
                    };
                    if pos.layer == Layer::Surface {
                        if tile_of(pos.p) == ftile {
                            if amount > 0 {
                                self.set_state(id, AntState::Harvesting { target: fid });
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
                            if let Carry::Food(kind) = carrying {
                                if self.block_room(Layer::Underground, tile, kind) > 0 {
                                    self.store_food(id, tile);
                                }
                                // else: pantry cell filled (or its block went
                                // typed-foreign) on the way — re-pick
                            }
                            self.set_job(id, Job::Idle);
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
        self.squad_convert();
    }

    /// Squad conversion (X-menu group mode): a leader who starts farming or
    /// attacking converts every follower to that activity — they persist at
    /// it (farm the source / rampage the hostiles) and stop following until
    /// re-recruited with another "all ants join" order.
    fn squad_convert(&mut self) {
        if self.colony.dead {
            return;
        }
        enum Lead {
            Farm(u32),
            Attack(u32),
        }
        let mut leads: Vec<(u32, Lead)> = Vec::new();
        for id in self.ant_ids() {
            let Some(&ent) = self.ids.get(&id) else {
                continue;
            };
            let info = self
                .ecs
                .query_one::<(&AntState, &WorkerAi)>(ent)
                .ok()
                .and_then(|mut q| q.get().map(|(st, ai)| ((*st).clone(), ai.attack_after)));
            let Some((state, attack_after)) = info else {
                continue;
            };
            let lead = if let Some(t) = attack_after {
                // the attack order was just issued (walk-to-attack included)
                Lead::Attack(t)
            } else {
                match state {
                    AntState::Harvesting { target } => Lead::Farm(target),
                    AntState::Fighting { target } => Lead::Attack(target),
                    _ => continue,
                }
            };
            leads.push((id, lead));
        }
        for (leader, lead) in leads {
            let ids = self.ant_ids();
            for fid in ids {
                let follows = self
                    .ids
                    .get(&fid)
                    .and_then(|&e| self.ecs.get::<&WorkerAi>(e).ok())
                    .map(|ai| matches!(ai.job, Job::Follow(l, _) if l == leader))
                    .unwrap_or(false);
                if !follows {
                    continue;
                }
                match lead {
                    Lead::Farm(target) => {
                        // workers AND soldiers farm the leader's source;
                        // specialists (honey/medic) keep following — their
                        // role is the point, not extra hands
                        let specialist = self
                            .ids
                            .get(&fid)
                            .and_then(|&e| self.ecs.get::<&Ant>(e).ok())
                            .map(|a| matches!(a.caste, Caste::Honey | Caste::Medic))
                            .unwrap_or(false);
                        if specialist {
                            continue;
                        }
                        self.set_job(fid, Job::Fetch(target));
                        self.ev(format!(
                            "ant #{fid} joins the harvest of #{target} (leader #{leader})"
                        ));
                    }
                    Lead::Attack(target) => {
                        // assist-attack: the whole squad (workers included)
                        // fights the leader's target, then returns to
                        // following when it dies — no rampage, keep it simple
                        let current = self
                            .ids
                            .get(&fid)
                            .and_then(|&e| self.ecs.get::<&WorkerAi>(e).ok())
                            .map(|ai| ai.attack_after)
                            .unwrap_or(None);
                        if current != Some(target) {
                            self.set_attack_after(fid, Some(target));
                            self.ev(format!(
                                "ant #{fid} joins the attack on spider #{target} (leader #{leader})"
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Keep exactly one worker assigned to feeding the queen (founding
    /// mode). The designation holds while the chosen ant is alive, a worker,
    /// and not tied up by the player (Manual) or a squad (Follow); otherwise
    /// it passes to the lowest-id such worker — stealing one mid-fetch is
    /// fine: a carried craved unit goes straight to the queen.
    fn designate_feeder(&mut self) {
        if !self.colony.founding {
            return;
        }
        let feeder_ok = |sim: &Sim, id: u32| -> bool {
            sim.ids.get(&id).is_some_and(|&e| {
                sim.ecs
                    .get::<&Ant>(e)
                    .is_ok_and(|a| a.caste == Caste::Worker)
                    && sim.ecs.get::<&Fallen>(e).is_err()
                    && sim
                        .ecs
                        .get::<&WorkerAi>(e)
                        .is_ok_and(|ai| !matches!(ai.job, Job::Manual | Job::Follow(..)))
            })
        };
        if self.colony.feeder_id.is_some_and(|id| feeder_ok(self, id)) {
            return;
        }
        let lost = self.colony.feeder_id.filter(|id| self.ids.contains_key(id));
        let cand = self
            .ant_ids()
            .into_iter()
            .filter(|&id| feeder_ok(self, id))
            .min();
        self.colony.feeder_id = cand;
        if let Some(c) = cand {
            match lost {
                Some(old) => {
                    self.set_job(c, Job::Feed);
                    self.ev(format!(
                        "feeder #{old} unavailable — worker #{c} feeds the queen"
                    ));
                }
                None => {
                    self.set_job(c, Job::Feed);
                    self.ev(format!("worker #{c} is the queen's feeder"));
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
                .any(|&(ax, ay)| chebyshev((ax, ay), tile) <= self.rules.sight_range)
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
        let craving = self.craving();
        let mut best: Option<(u32, u32, u32, u32)> = None;
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
            // founding: forage what the queen craves first — the physical
            // feeding economy starves beside full carb piles otherwise.
            // Legacy keeps the old protein-first key unchanged.
            let super_first = if ff.kind == FoodKind::Super || ff.kind == FoodKind::Protein {
                0
            } else {
                1
            };
            let key = if self.colony.founding {
                (
                    if ff.kind == craving { 0 } else { 1 },
                    super_first,
                    manhattan(tile, entrance),
                    fid,
                )
            } else {
                (super_first, manhattan(tile, entrance), 0, fid)
            };
            if best.map(|b| key < b).unwrap_or(true) {
                best = Some(key);
            }
        }
        best.map(|k| k.3)
    }

    /// Where carriers place collected food of `kind`: the nearest silver
    /// cell to the entrance whose typed block has room for that kind,
    /// else the first free cell that does (both visible + safe).
    pub(crate) fn pantry_tile(&self, kind: FoodKind) -> Option<(u32, u32)> {
        let (ex, ey) = self.world.entrance?;
        let mut fallback: Option<(u32, u32)> = None;
        for r in 0..=14u32 {
            for tile in self.ring_tiles((ex, ey), r) {
                if self.world.underground.get(tile.0, tile.1) != EMPTY {
                    continue;
                }
                if self.block_room(Layer::Underground, tile, kind) == 0 {
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

    /// Haul carried dirt out through the entrance and discard it above
    /// ground (workers' spoil goes to the surface, never back into the
    /// nest). Walk-to-drop machinery handles the crossing; a surface dirt
    /// drop discards everything carried.
    pub(crate) fn haul_out_dirt(&mut self, id: u32) {
        let Some((ex, ey)) = self.world.entrance else {
            self.set_retry(id, 100);
            return;
        };
        let dump = (ex + 1, ey + 1);
        self.set_drop_after(id, Some(dump));
        if !self.route(id, Layer::Surface, dump) {
            self.set_drop_after(id, None);
            self.set_retry(id, 60);
        }
    }

    pub(crate) fn pick_dig_target(&self) -> Option<(u32, u32)> {
        let (qx, qy) = self.queen_tile();
        for r in 1u32..=self.rules.dig_expand_radius {
            for dy in -(r as i32)..=(r as i32) {
                for dx in -(r as i32)..=(r as i32) {
                    if dx.abs() != r as i32 && dy.abs() != r as i32 {
                        continue;
                    }
                    let x = qx as i32 + dx;
                    let y = qy as i32 + dy;
                    if x < 1
                        || y < 1
                        || x >= self.rules.width as i32 - 1
                        || y >= self.rules.height as i32 - 1
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

    /// Nearest free egg cell around the queen — preferring ORANGE soil
    /// cells (eggs only hatch on orange, so the founding brood lands where
    /// it can hatch without being carried), falling back to any empty
    /// cell within the same radius.
    pub(crate) fn free_egg_tile(&self) -> Option<(u32, u32)> {
        let (qx, qy) = self.queen_tile();
        for prefer_orange in [true, false] {
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
                            || x >= self.rules.width as i32 - 1
                            || y >= self.rules.height as i32 - 1
                        {
                            continue;
                        }
                        let (x, y) = (x as u32, y as u32);
                        if self.world.underground.get(x, y) == EMPTY
                            && !self.tile_occupied((x, y), Layer::Underground)
                            && (!prefer_orange
                                || self.soil_at(Layer::Underground, x, y) == SOIL_ORANGE)
                        {
                            return Some((x, y));
                        }
                    }
                }
            }
        }
        None
    }

    pub(crate) fn queen_tile(&self) -> (u32, u32) {
        self.ant_tile(self.colony.queen_id)
    }

    /// The downed ant this medic is carrying, if any.
    pub(crate) fn carried_fallen(&self, medic: u32) -> Option<u32> {
        for (&id, &ent) in self.ids.iter() {
            if let Ok(f) = self.ecs.get::<&Fallen>(ent) {
                if f.carried_by == Some(medic) {
                    return Some(id);
                }
            }
        }
        None
    }

    /// Nearest unattended casualty (downed, not carried, not yet healing),
    /// tie → lowest id. Medics drop everything for it.
    pub(crate) fn nearest_fallen(&self) -> Option<u32> {
        let mut best: Option<((u32, u32), u32)> = None; // ((chebyshev from queen, layer), id)
        let home = self.queen_tile();
        for (&id, &ent) in self.ids.iter() {
            let Ok(f) = self.ecs.get::<&Fallen>(ent) else {
                continue;
            };
            if f.carried_by.is_some() || f.heal_t.is_some() {
                continue;
            }
            let Ok(p) = self.ecs.get::<&Pos>(ent) else {
                continue;
            };
            let key = (chebyshev(tile_of(p.p), home), p.layer as u8 as u32);
            if best.map(|b| (key, id) < b).unwrap_or(true) {
                best = Some((key, id));
            }
        }
        best.map(|(_, id)| id)
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
                    Caste::Queen | Caste::Honey | Caste::Medic => {}
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
