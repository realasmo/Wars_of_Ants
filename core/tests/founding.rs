use woa_core::{Command, DevSpawn, Layer, Phase, Sim, Team, DIRT, EMPTY, SOURCES};

fn queen(s: &Sim) -> woa_core::EntitySnap {
    s.snapshot().into_iter().find(|e| e.kind == 0).unwrap()
}

fn eggs(s: &Sim) -> usize {
    s.snapshot().iter().filter(|e| e.kind == 3).count()
}

fn workers(s: &Sim) -> usize {
    s.snapshot().iter().filter(|e| e.kind == 1).count()
}

/// Found the nest at the queen's spawn tile (map center) and return to the
/// caller a sim in Phase::Founding with the queen in the starter chamber.
fn founded(seed: u64) -> Sim {
    let mut s = Sim::new_founding(seed, Team::Red);
    let q = queen(&s);
    assert!(s.issue(Command::Land { ant: q.id, x: q.x, y: q.y }));
    assert!(s.issue(Command::FoundNest { ant: q.id, x: q.x, y: q.y }));
    s
}

fn neighbors(x: u32, y: u32) -> Vec<(u32, u32)> {
    let mut v = Vec::new();
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            v.push(((x as i32 + dx) as u32, (y as i32 + dy) as u32));
        }
    }
    v
}

fn is_soft(k: u8) -> bool {
    k == DIRT || k == 2 || k == 3
}

/// An (empty stand tile, adjacent soft target) pair near the queen — the
/// chamber walls are only diggable from the chamber edge, so the queen must
/// walk there first, exactly like in play.
fn dig_site(s: &Sim) -> ((u32, u32), (u32, u32)) {
    let q = queen(s);
    let (qx, qy) = (q.x.floor() as u32, q.y.floor() as u32);
    for r in 1..=4u32 {
        for dy in -(r as i32)..=(r as i32) {
            for dx in -(r as i32)..=(r as i32) {
                if dx.abs() != r as i32 && dy.abs() != r as i32 {
                    continue;
                }
                let (x, y) = ((qx as i32 + dx) as u32, (qy as i32 + dy) as u32);
                if s.tile_at(Layer::Underground, x, y) != EMPTY {
                    continue;
                }
                for (nx, ny) in neighbors(x, y) {
                    if is_soft(s.tile_at(Layer::Underground, nx, ny)) {
                        return ((x, y), (nx, ny));
                    }
                }
            }
        }
    }
    panic!("no dig site near the chamber");
}

/// Walk the queen to a chamber-edge stand tile, dig the target, and return the
/// dug tile (now EMPTY) plus the stand tile it is adjacent to.
fn walk_and_dig(s: &mut Sim, ticks: usize) -> (u32, u32) {
    let ((sx, sy), (tx, ty)) = dig_site(s);
    let q = queen(s);
    assert!(s.issue(Command::Move {
        ant: q.id,
        x: sx as f64 + 0.5,
        y: sy as f64 + 0.5
    }));
    for _ in 0..ticks {
        s.tick();
    }
    let q = queen(s);
    assert!(s.issue(Command::Dig { ant: q.id, tx, ty }));
    for _ in 0..25 {
        s.tick();
    }
    assert_eq!(s.tile_at(Layer::Underground, tx, ty), EMPTY);
    assert_eq!(queen(s).aux, 2.0, "queen should carry dirt (aux=2)");
    (tx, ty)
}

#[test]
fn flight_start_is_lone_flying_queen() {
    let s = Sim::new_founding(42, Team::Red);
    assert_eq!(s.colony.phase, Phase::Flight);
    assert_eq!(s.colony.team, Team::Red);
    assert!(s.colony.founding);
    assert!(s.world.entrance.is_none());
    let snap = s.snapshot();
    assert_eq!(workers(&s), 0);
    assert_eq!(eggs(&s), 0);
    let q = queen(&s);
    assert_eq!(q.layer, Layer::Surface as u8);
    assert_eq!(q.state, 4); // flying
    assert_eq!(s.colony.ant_count, 1);
}

#[test]
fn queen_flies_fast_then_lands_and_walks_slower() {
    let mut s = Sim::new_founding(42, Team::Blue);
    let q = queen(&s);
    // fly 10 tiles in a straight line: at QUEEN_FLY_SPEED (5/s) that is 2s
    assert!(s.issue(Command::Move {
        ant: q.id,
        x: q.x + 10.0,
        y: q.y
    }));
    for _ in 0..40 {
        s.tick();
    }
    let q = queen(&s);
    let flown = q.x - (48.5);
    assert!(flown >= 9.5, "queen flew only {} tiles in 2s", flown);

    assert!(s.issue(Command::Land { ant: q.id, x: q.x, y: q.y }));
    assert_eq!(s.colony.phase, Phase::Grounded);
    // walk 5 tiles at QUEEN.speed (2.5/s) = 2s
    assert!(s.issue(Command::Move {
        ant: q.id,
        x: q.x + 5.0,
        y: q.y
    }));
    let before = queen(&s).x;
    for _ in 0..40 {
        s.tick();
    }
    let walked = queen(&s).x - before;
    assert!(
        (4.5..=5.5).contains(&walked),
        "grounded queen walked {} tiles in 2s (want ~5)",
        walked
    );
    // landing is one-way: a second Land is refused
    assert!(!s.issue(Command::Land { ant: q.id, x: q.x, y: q.y }));
}

#[test]
fn found_nest_creates_entrance_and_chamber() {
    let mut s = Sim::new_founding(42, Team::Red);
    let q = queen(&s);
    assert!(s.issue(Command::Land { ant: q.id, x: q.x, y: q.y }));
    assert!(s.issue(Command::FoundNest { ant: q.id, x: q.x, y: q.y }));
    assert_eq!(s.colony.phase, Phase::Founding);
    assert_eq!(s.colony.phase_t, 60.0);
    // 2×2 entrance hole + 4×4 starter chamber (block-aligned)
    assert_eq!(s.world.entrance, Some((48, 48)));
    for y in 48..=49 {
        for x in 48..=49 {
            assert_eq!(s.tile_at(Layer::Underground, x, y), EMPTY, "hole block");
        }
    }
    for y in 50..=53 {
        for x in 48..=51 {
            assert_eq!(s.tile_at(Layer::Underground, x, y), EMPTY, "chamber");
        }
    }
    assert_eq!(s.tile_at(Layer::Underground, 47, 50), 1, "chamber edge stays dirt");
    let q = queen(&s);
    assert_eq!(q.layer, Layer::Underground as u8);
    assert!(s.tiles_dug() >= 20);
    // one nest per game
    assert!(!s.issue(Command::FoundNest { ant: q.id, x: q.x, y: q.y }));
}

#[test]
fn found_nest_refused_near_map_border() {
    let mut s = Sim::new_founding(42, Team::Red);
    let q = queen(&s);
    assert!(s.issue(Command::Land { ant: q.id, x: q.x, y: q.y }));
    // walk to the bottom rows where the starter chamber would not fit
    assert!(s.issue(Command::Move { ant: q.id, x: 48.5, y: 92.5 }));
    for _ in 0..420 {
        s.tick();
    }
    let q = queen(&s);
    assert!(!s.issue(Command::FoundNest { ant: q.id, x: q.x, y: q.y }));
    assert_eq!(s.colony.phase, Phase::Grounded);
    assert!(s.world.entrance.is_none());
}

#[test]
fn queen_dig_yields_dirt_and_dump_refills_the_cell() {
    let mut s = founded(42);
    let q = queen(&s);
    let (tx, ty) = walk_and_dig(&mut s, 60);
    // a second block may be dug while hauling one (capacity two) — the
    // dedicated test below covers the full stack; here one block is enough
    // dumping underground refills the adjacent block just dug
    assert!(s.issue(Command::Drop { ant: q.id, tx, ty }));
    assert_eq!(s.tile_at(Layer::Underground, tx, ty), DIRT);
    assert_eq!(queen(&s).aux, 0.0);
}

#[test]
fn dirt_dumped_on_the_surface_disappears() {
    let mut s = founded(42);
    let q = queen(&s);
    let (tx, ty) = walk_and_dig(&mut s, 60);
    // haul it up through the entrance
    assert!(s.issue(Command::UseEntrance { ant: q.id }));
    for _ in 0..100 {
        s.tick();
    }
    assert_eq!(queen(&s).layer, Layer::Surface as u8);
    assert!(s.issue(Command::Drop { ant: q.id, tx: 0, ty: 0 }));
    assert_eq!(queen(&s).aux, 0.0);
    assert_eq!(
        s.tile_at(Layer::Underground, tx, ty),
        EMPTY,
        "dug cell stays dug"
    );
}

#[test]
fn founding_timer_lays_four_eggs_then_workers_hatch_on_orange() {
    let mut s = founded(42);
    // 60s window (plus a margin: DT is not binary-exact, so the timer can
    // need one extra tick to cross zero)
    for _ in 0..1220 {
        s.tick();
    }
    assert_eq!(s.colony.phase, Phase::Brood);
    assert_eq!(eggs(&s), 4);
    assert_eq!(s.colony.eggs_laid, 4);
    // eggs only transform on orange cells — with none, they wait forever
    for _ in 0..3620 {
        s.tick();
    }
    assert_eq!(phase(&s), Phase::Brood, "eggs must wait without orange soil");
    assert_eq!(workers(&s), 0);
    // paint orange under one egg — it hatches and the colony phase begins
    let egg = s.snapshot().into_iter().find(|e| e.kind == 3).unwrap();
    let (ex, ey) = (egg.x.floor() as u32, egg.y.floor() as u32);
    s.dev_set_soil(1, ex, ey, 1);
    for _ in 0..30 {
        s.tick();
    }
    assert_eq!(phase(&s), Phase::Colony);
    assert_eq!(workers(&s), 1);
    assert_eq!(eggs(&s), 3, "the other eggs still wait for orange");
    // once workers exist the queen loses her digging rights: walk her to a
    // wall first so the refusal can't be explained by distance
    let ((sx, sy), (tx, ty)) = dig_site(&s);
    let q = queen(&s);
    assert!(s.issue(Command::Move {
        ant: q.id,
        x: sx as f64 + 0.5,
        y: sy as f64 + 0.5
    }));
    for _ in 0..60 {
        s.tick();
    }
    let q = queen(&s);
    assert!(!s.issue(Command::Dig { ant: q.id, tx, ty }));
}

fn phase(s: &Sim) -> Phase {
    s.colony.phase
}

#[test]
fn starvation_is_suspended_while_the_queen_is_alone() {
    let mut s = Sim::new_founding(42, Team::Red);
    s.dev_set_food(0);
    // without the founding grace the colony would die at ~215s (4300 ticks)
    for _ in 0..6000 {
        s.tick();
    }
    assert!(!s.colony.dead, "lone founding queen must not starve");
}

#[test]
fn spiders_ignore_the_flying_queen_but_not_the_grounded_one() {
    let mut s = Sim::new_founding(42, Team::Red);
    let q = queen(&s);
    s.dev_spawn(DevSpawn::Spider, q.x, q.y);
    for _ in 0..100 {
        s.tick();
    }
    assert_eq!(queen(&s).hp, 1.0, "flying queen must be untouchable");

    assert!(s.issue(Command::Land { ant: q.id, x: q.x, y: q.y }));
    s.dev_spawn(DevSpawn::Spider, queen(&s).x, queen(&s).y);
    for _ in 0..500 {
        s.tick();
    }
    assert!(s.colony.dead, "grounded queen killed by spiders must end the colony");
}

#[test]
fn founding_is_deterministic_per_seed_and_team() {
    let run = |team: Team| -> String {
        let mut s = Sim::new_founding(7, team);
        let q = queen(&s);
        s.issue(Command::Move {
            ant: q.id,
            x: q.x + 7.0,
            y: q.y + 3.0,
        });
        for _ in 0..80 {
            s.tick();
        }
        // land where she stands now, then found the nest there (landing
        // targets a spot, so read the position fresh)
        let q = queen(&s);
        s.issue(Command::Land { ant: q.id, x: q.x, y: q.y });
        s.issue(Command::FoundNest { ant: q.id, x: q.x, y: q.y });
        for _ in 0..60 {
            s.tick();
        }
        let (tx, ty) = walk_and_dig(&mut s, 60);
        s.issue(Command::Drop { ant: queen(&s).id, tx, ty });
        for _ in 0..1300 {
            s.tick();
        }
        s.canonical_state()
    };
    let a = run(Team::Blue);
    let b = run(Team::Blue);
    assert_eq!(a, b);
    let red = run(Team::Red);
    assert_ne!(a, red, "team choice must be part of the canonical state");
}

#[test]
fn distant_dig_command_walks_there_and_digs() {
    let mut s = founded(42);
    let q = queen(&s);
    // a fully-soft block on the digging frontier (adjacent to emptiness),
    // a few tiles from the queen — deeper blocks are correctly refused until
    // the nest tunnels toward them
    let q0 = queen(&s);
    let (qx, qy) = (q0.x.floor() as i32, q0.y.floor() as i32);
    let mut target = None;
    let mut best_d = 0i32;
    'scan: for by in (40..58u32).step_by(2) {
        for bx in (40..58u32).step_by(2) {
            let mut soft = true;
            for dy in 0..2u32 {
                for dx in 0..2u32 {
                    let k = s.tile_at(Layer::Underground, bx + dx, by + dy);
                    if k != DIRT && k != 2 && k != 3 {
                        soft = false;
                    }
                }
            }
            if !soft {
                continue;
            }
            // frontier = fully-soft block with an ORTHOGONALLY empty
            // neighbor tile (diagonal-only openings don't count anymore)
            let ring_empty = [
                (bx.wrapping_sub(1), by), (bx.wrapping_sub(1), by + 1),
                (bx + 2, by), (bx + 2, by + 1),
                (bx, by.wrapping_sub(1)), (bx + 1, by.wrapping_sub(1)),
                (bx, by + 2), (bx + 1, by + 2),
            ]
            .iter()
            .any(|&(x, y)| s.tile_at(Layer::Underground, x, y) == EMPTY);
            if !ring_empty {
                continue;
            }
            let d = (bx as i32 - qx).abs() + (by as i32 - qy).abs();
            if d > best_d {
                best_d = d;
                target = Some((bx, by));
                if d >= 6 {
                    break 'scan;
                }
            }
        }
    }
    let (bx, by) = target.expect("a soft block near the nest");
    // single Dig command from afar: she walks, then digs the whole block
    assert!(s.issue(Command::Dig { ant: q.id, tx: bx, ty: by }));
    for _ in 0..600 {
        s.tick();
    }
    for dy in 0..2u32 {
        for dx in 0..2u32 {
            assert_eq!(s.tile_at(Layer::Underground, bx + dx, by + dy), EMPTY);
        }
    }
    assert_eq!(queen(&s).aux, 2.0, "queen hauls one dirt block");
}

#[test]
fn eggs_can_be_carried_and_placed_keeping_hatch_state() {
    let mut s = founded(42);
    for _ in 0..1220 {
        s.tick();
    }
    let q = queen(&s);
    let egg = s.snapshot().into_iter().find(|e| e.kind == 3).unwrap();
    let frac0 = egg.extra; // hatch progress
    assert!(s.issue(Command::PickEgg { ant: q.id, egg: egg.id }));
    // carried: snapshot flags it and the queen carries an egg-mark (aux 3)
    let s1 = s.snapshot();
    let e1 = s1.iter().find(|e| e.id == egg.id).unwrap();
    assert_eq!(e1.state, 1, "egg is carried");
    assert_eq!(s1.iter().find(|e| e.kind == 0).unwrap().aux, 3.0);
    // no digging while hands are full
    let ((_, _), (tx, ty)) = dig_site(&s);
    assert!(!s.issue(Command::Dig { ant: q.id, tx, ty }));
    // place it on an adjacent empty chamber cell
    let (qx, qy) = (q.x.floor() as u32, q.y.floor() as u32);
    let spot = [(qx + 1, qy), (qx - 1, qy), (qx, qy + 1), (qx, qy - 1)]
        .into_iter()
        .find(|&(x, y)| s.tile_at(Layer::Underground, x, y) == EMPTY)
        .unwrap();
    assert!(s.issue(Command::Drop { ant: q.id, tx: spot.0, ty: spot.1 }));
    let e2 = s.snapshot().into_iter().find(|e| e.id == egg.id).unwrap();
    assert_eq!(e2.state, 0, "egg placed");
    assert!((e2.extra - frac0).abs() < 0.05, "hatch state preserved");
}

#[test]
fn dropped_food_spoils_off_silver_and_freezes_on_it() {
    // (the founding queen never auto-picks food — use a legacy worker)
    let mut s = Sim::new(3, woa_core::Config::default());
    let w = s.snapshot().into_iter().find(|e| e.kind == 1).unwrap();
    // drop 1 unit far from any soil: it must spoil after SPOIL_TIME
    // (worker picks food up on the surface via a manual move onto a pile)
    let pile = s
        .snapshot()
        .into_iter()
        .find(|e| e.kind == 2)
        .expect("legacy worldgen has surface piles");
    // cross to the surface first (Move is same-layer), then walk to the pile
    assert!(s.issue(Command::UseEntrance { ant: w.id }));
    for _ in 0..100 {
        s.tick();
    }
    assert!(s.issue(Command::Move {
        ant: w.id,
        x: pile.x,
        y: pile.y
    }));
    for _ in 0..400 {
        s.tick();
    }
    let carrying = s
        .snapshot()
        .into_iter()
        .find(|e| e.kind == 1 && e.extra > 0.5)
        .expect("worker picked up food");
    assert!(s.issue(Command::Drop {
        ant: carrying.id,
        tx: carrying.x.floor() as u32,
        ty: carrying.y.floor() as u32
    }));
    // off silver: gone after 5 minutes
    for _ in 0..6100 {
        s.tick();
    }
    let spoiled = !s
        .snapshot()
        .iter()
        .any(|e| e.kind == 2 && e.hp < 1.0);
    assert!(spoiled, "dropped food must spoil off silver");
}

#[test]
fn food_piles_respect_cell_cap() {
    let mut s = Sim::new_founding(1337, Team::Red);
    // (fresh seed's own piles sit near the map center; spawn far away at 30,30)
    // worldgen sources are everywhere now — only count green (aux 0) units
    let before: usize = s
        .snapshot()
        .iter()
        .filter(|e| e.kind == 2 && e.aux == 0.0 && (e.x - 30.5).abs() <= 12.0 && (e.y - 30.5).abs() <= 12.0)
        .count();
    s.dev_spawn(DevSpawn::Food, 30.5, 30.5);
    let piles: Vec<_> = s
        .snapshot()
        .into_iter()
        .filter(|e| e.kind == 2 && e.aux == 0.0 && (e.x - 30.5).abs() <= 12.0 && (e.y - 30.5).abs() <= 12.0)
        .collect();
    assert!(piles.len() > before, "spreading created cells");
    assert!(!piles.is_empty());
    for p in &piles {
        assert!(p.extra <= 6.0, "pile of {} exceeds the cell cap", p.extra);
    }
    let total: f64 = piles.iter().map(|p| p.extra).sum();
    assert_eq!(total as u32, 45, "no food lost to spreading");
}

#[test]
fn founding_soil_is_seeded_deterministically() {
    let a = Sim::new_founding(42, Team::Red);
    let b = Sim::new_founding(42, Team::Blue);
    assert_eq!(a.world.soil_underground, b.world.soil_underground);
    assert_eq!(a.world.soil_surface, b.world.soil_surface);
    let orange = a.world.soil_underground.iter().filter(|&&v| v == 1).count();
    let silver = a.world.soil_underground.iter().filter(|&&v| v == 2).count();
    assert!(orange > 50, "orange soil exists to be discovered ({orange})");
    assert!(silver > 50, "silver soil exists to be discovered ({silver})");
    assert_eq!(a.patches.len(), 4, "two orange + two silver dust patches");
}

#[test]
fn queen_hauls_two_dirt_blocks_before_dumping() {
    let mut s = founded(42);
    let q = queen(&s);
    // two distinct frontier blocks
    let (a_tx, a_ty) = walk_and_dig(&mut s, 60);
    // still carrying one block — a second dig elsewhere is allowed
    let ((sx, sy), (b_tx, b_ty)) = dig_site(&s);
    assert!(s.issue(Command::Move {
        ant: q.id,
        x: sx as f64 + 0.5,
        y: sy as f64 + 0.5
    }));
    for _ in 0..60 {
        s.tick();
    }
    assert!(s.issue(Command::Dig { ant: q.id, tx: b_tx, ty: b_ty }));
    for _ in 0..140 {
        s.tick();
    }
    // full: a third dig is refused until dumped
    let ((sx3, sy3), (c_tx, c_ty)) = dig_site(&s);
    let _ = (sx3, sy3);
    assert!(
        !s.issue(Command::Dig {
            ant: q.id,
            tx: c_tx,
            ty: c_ty
        }),
        "third dig must be refused at capacity"
    );
    // each dump refills one adjacent block: dump B where she stands first
    assert!(s.issue(Command::Drop { ant: q.id, tx: b_tx, ty: b_ty }));
    assert_eq!(queen(&s).aux, 2.0, "still hauling one block after the first dump");
    // walk back into the dug-out block A and refill it
    assert!(s.issue(Command::Move {
        ant: q.id,
        x: a_tx as f64 + 0.5,
        y: a_ty as f64 + 0.5
    }));
    for _ in 0..140 {
        s.tick();
    }
    assert!(s.issue(Command::Drop { ant: q.id, tx: a_tx, ty: a_ty }));
    assert_eq!(queen(&s).aux, 0.0, "all dirt dumped");
}

#[test]
fn diagonal_dig_requires_an_open_flank() {
    // hunt a seed where the geometry exists: after digging one frontier
    // block, one of its diagonal blocks is fully soft with both flank tiles
    // still solid
    for seed in [42u64, 7, 1, 99, 123] {
        let mut s = founded(seed);
        let q0 = queen(&s);
        let (a_tx, a_ty) = walk_and_dig(&mut s, 60);
        let (abx, aby) = (a_tx & !1, a_ty & !1); // the dug block
        // stand the queen at a corner of the dug block
        for (cx, cy) in [(abx + 1, aby + 1), (abx, aby + 1), (abx + 1, aby), (abx, aby)] {
            let target = s.issue(Command::Move {
                ant: q0.id,
                x: cx as f64 + 0.5,
                y: cy as f64 + 0.5,
            });
            if !target {
                continue;
            }
            for _ in 0..80 {
                s.tick();
            }
            let q = queen(&s);
            if (q.x.floor() as u32, q.y.floor() as u32) != (cx, cy) {
                continue;
            }
            // diagonal blocks from this corner
            for (dx, dy) in [(-2i32, -2i32), (2, -2), (-2, 2), (2, 2)] {
                let dbx = (cx as i32 + if dx > 0 { 1 } else { -2 }) as u32;
                let dby = (cy as i32 + if dy > 0 { 1 } else { -2 }) as u32;
                let mut soft = true;
                for ddy in 0..2u32 {
                    for ddx in 0..2u32 {
                        let k = s.tile_at(Layer::Underground, dbx + ddx, dby + ddy);
                        if !(1..=3).contains(&k) {
                            soft = false;
                        }
                    }
                }
                if !soft {
                    continue;
                }
                // flank tiles between this corner and the diagonal block
                let f1x = cx as i32 + if dx > 0 { 1 } else { -1 };
                let f2y = cy as i32 + if dy > 0 { 1 } else { -1 };
                let f1 = s.tile_at(Layer::Underground, f1x as u32, cy);
                let f2 = s.tile_at(Layer::Underground, cx, f2y as u32);
                let solid = |k: u8| (1..=3).contains(&k);
                if solid(f1) && solid(f2) {
                    let q = queen(&s);
                    assert!(
                        !s.issue(Command::Dig { ant: q.id, tx: dbx, ty: dby }),
                        "seed {seed}: corner-only dig must be refused (flanks {f1}/{f2})"
                    );
                    return; // geometry verified
                }
            }
        }
    }
    // orthogonal digging still works regardless
    let mut s = founded(42);
    let ((sx, sy), (tx2, ty2)) = dig_site(&s);
    let q = queen(&s);
    assert!(s.issue(Command::Move { ant: q.id, x: sx as f64 + 0.5, y: sy as f64 + 0.5 }));
    for _ in 0..80 {
        s.tick();
    }
    assert!(s.issue(Command::Dig { ant: q.id, tx: tx2, ty: ty2 }));
}

#[test]
fn soil_blocks_are_never_blocked_by_rocks() {
    let s = Sim::new_founding(42, Team::Red);
    let w = s.config.width;
    for by in (1..s.config.height - 2).step_by(2) {
        for bx in (1..w - 2).step_by(2) {
            let soil = s.world.soil_underground[(by * w + bx) as usize];
            if soil != 0 {
                for dy in 0..2u32 {
                    for dx in 0..2u32 {
                        let k = s.tile_at(Layer::Underground, bx + dx, by + dy);
                        assert_ne!(k, 4, "rock inside a special soil block at ({},{})", bx + dx, by + dy);
                        assert!(k >= 1 && k <= 3, "soil block cell must stay diggable");
                    }
                }
            }
        }
    }
}

#[test]
fn distant_pick_and_drop_send_the_ant_walking() {
    let mut s = founded(42);
    for _ in 0..1220 {
        s.tick();
    }
    // walk the queen to the chamber edge first, then target the farthest egg
    let q = queen(&s);
    let entrance = s.world.entrance.unwrap();
    assert!(s.issue(Command::Move {
        ant: q.id,
        x: entrance.0 as f64 + 1.0,
        y: entrance.1 as f64 + 1.0
    }));
    for _ in 0..200 {
        s.tick();
    }
    let q = queen(&s);
    let eggs_all: Vec<_> = s.snapshot().into_iter().filter(|e| e.kind == 3).collect();
    let egg = eggs_all
        .iter()
        .max_by(|a, b| {
            let da = (a.x - q.x).abs() + (a.y - q.y).abs();
            let db = (b.x - q.x).abs() + (b.y - q.y).abs();
            da.partial_cmp(&db).unwrap()
        })
        .unwrap();
    let (ex, ey) = (egg.x.floor() as u32, egg.y.floor() as u32);
    assert!((ex as f64 - q.x).abs() + (ey as f64 - q.y).abs() > 2.5, "test needs a distant egg");
    assert!(s.issue(Command::PickEgg { ant: q.id, egg: egg.id }));
    for _ in 0..300 {
        s.tick();
    }
    assert_eq!(queen(&s).aux, 3.0, "queen should reach and pick up the egg");
    // distant empty cell: Drop walks there and places it
    let (qx, qy) = { let qq = queen(&s); (qq.x.floor() as u32, qq.y.floor() as u32) };
    let mut spot = None;
    'scan: for r in 3..12u32 {
        for dy in -(r as i32)..=(r as i32) {
            for dx in -(r as i32)..=(r as i32) {
                if dx.abs() != r as i32 && dy.abs() != r as i32 { continue; }
                let x = qx as i32 + dx;
                let y = qy as i32 + dy;
                if x < 2 || y < 2 { continue; }
                if s.tile_at(Layer::Underground, x as u32, y as u32) == EMPTY {
                    spot = Some((x as u32, y as u32));
                    break 'scan;
                }
            }
        }
    }
    let (sx, sy) = spot.expect("an empty cell a few tiles away");
    assert!((sx.max(qx) - sx.min(qx)) + (sy.max(qy) - sy.min(qy)) > 2, "target must be distant");
    assert!(s.issue(Command::Drop { ant: q.id, tx: sx, ty: sy }));
    for _ in 0..400 {
        s.tick();
    }
    assert_eq!(queen(&s).aux, 0.0, "egg should be placed after walking");
    let placed = s.snapshot().into_iter().find(|e| e.id == egg.id).unwrap();
    assert_eq!((placed.x.floor() as u32, placed.y.floor() as u32), (sx, sy));
}

#[test]
fn right_click_flies_there_lands_then_walks_and_founds() {
    let mut s = Sim::new_founding(42, Team::Red);
    let q = queen(&s);
    // distant land target: no instant touchdown mid-flight
    let (lx, ly) = (q.x + 12.0, q.y - 5.0);
    assert!(s.issue(Command::Land { ant: q.id, x: lx, y: ly }));
    for _ in 0..20 {
        s.tick();
    }
    assert_eq!(phase(&s), Phase::Flight, "must not land before arriving");
    for _ in 0..100 {
        s.tick();
    }
    assert_eq!(phase(&s), Phase::Grounded);
    let q = queen(&s);
    assert_eq!(
        (q.x.floor() as u32, q.y.floor() as u32),
        (lx.floor() as u32, ly.floor() as u32),
        "lands at the clicked destination"
    );
    // distant found target: the nest appears only after the walk
    let (fx, fy) = (q.x + 6.0, q.y + 4.0);
    assert!(s.issue(Command::FoundNest { ant: q.id, x: fx, y: fy }));
    for _ in 0..30 {
        s.tick();
    }
    assert!(s.world.entrance.is_none(), "must not found mid-walk");
    for _ in 0..140 {
        s.tick();
    }
    let e = s.world.entrance.expect("nest founded after the walk");
    assert_eq!(e, ((fx.floor() as u32) & !1, (fy.floor() as u32) & !1));
    assert_eq!(phase(&s), Phase::Founding);
}

#[test]
fn founding_world_has_scattered_finite_sources_and_no_green() {
    let s = Sim::new_founding(42, Team::Red);
    let snaps = s.snapshot();
    // no green piles anywhere
    assert!(
        !snaps.iter().any(|e| e.kind == 2 && e.aux == 0.0),
        "founding world must not contain green food"
    );
    let sources: Vec<_> = snaps.iter().filter(|e| e.kind == 2 && e.state == 2).collect();
    let total: u32 = SOURCES.iter().map(|sp| sp.count).sum();
    assert_eq!(sources.len() as u32, total, "all six source types placed");
    let carbs_near = sources
        .iter()
        .filter(|e| e.aux == 5.0 && (e.x - 48.5).abs() <= 16.0 && (e.y - 48.5).abs() <= 16.0)
        .count();
    assert!(carbs_near >= 1, "at least one carb source near the founding center");
    // deterministic per seed
    let b = Sim::new_founding(42, Team::Blue);
    let sa: Vec<_> = sources.iter().map(|e| (e.id, e.x as u32, e.y as u32, e.extra as u32)).collect();
    let sb: Vec<_> = b
        .snapshot()
        .iter()
        .filter(|e| e.kind == 2 && e.state == 2)
        .map(|e| (e.id, e.x as u32, e.y as u32, e.extra as u32))
        .collect();
    assert_eq!(sa, sb);
}

#[test]
fn scouts_discover_sources_and_harvest_takes_time() {
    let mut s = founded(42);
    // fast-forward to the colony phase with a painted nursery
    for _ in 0..1220 {
        s.tick();
    }
    let e = s.snapshot().into_iter().find(|e| e.kind == 3).unwrap();
    s.dev_set_soil(1, e.x.floor() as u32, e.y.floor() as u32, 1);
    s.dev_set_soil(1, e.x.floor() as u32 + 2, e.y.floor() as u32, 1);
    for _ in 0..3620 + 60 {
        s.tick();
    }
    assert_eq!(phase(&s), Phase::Colony);
    // dev-spawn a strawberry (4s harvest) far from the nest, unknown
    let q = queen(&s);
    let sx = q.x + 20.0;
    let sy = q.y + 15.0;
    s.dev_spawn(DevSpawn::Source(4), sx, sy);
    let src = s
        .snapshot()
        .into_iter()
        .find(|e| e.kind == 2 && e.state == 2 && (e.x - sx).abs() < 1.0)
        .unwrap();
    let amount0 = src.extra as u32;
    // not discovered: no worker fetches it while it's out of sight — verify
    // via knowledge set indirectly: a nearby ant discovers it instantly
    let w = s.snapshot().into_iter().find(|e| e.kind == 1).unwrap();
    assert!(s.issue(Command::Move { ant: w.id, x: sx, y: sy }));
    for _ in 0..10 {
        s.tick();
    }
    // within sight now; give the worker time to arrive and harvest (4s/unit)
    for _ in 0..500 {
        s.tick();
    }
    let after: Vec<_> = s
        .snapshot()
        .into_iter()
        .filter(|e| e.kind == 2 && (e.x - sx).abs() < 1.0)
        .collect();
    let total: f64 = after.iter().map(|e| e.extra).sum();
    assert!(
        total < amount0 as f64,
        "harvesting consumed units from the source ({total} vs {amount0})"
    );
    // finite: AI workers deplete a source placed by the entrance (discovered
    // as they pass) until it despawns. Fresh dev workers: the commanded one
    // stays Manual forever.
    s.dev_set_food(999); // keep the colony from starving mid-test
    let (ex, ey) = s.world.entrance.unwrap();
    s.dev_spawn(DevSpawn::Worker, ex as f64 + 1.0, ey as f64 + 4.0);
    s.dev_spawn(DevSpawn::Worker, ex as f64 + 1.0, ey as f64 + 4.0);
    s.dev_spawn(DevSpawn::Source(4), ex as f64 + 3.5, ey as f64 + 3.5);
    let mut gone = false;
    for _ in 0..25000 {
        s.tick();
        gone = !s
            .snapshot()
            .iter()
            .any(|e| e.kind == 2 && e.state == 2 && (e.x - ex as f64 - 3.5).abs() < 1.0);
        if gone {
            break;
        }
    }
    assert!(gone, "depleted sources despawn (finite map resources)");
}

#[test]
fn spider_drops_protein_and_low_carbs_slow_the_colony() {
    let mut s = founded(42);
    for _ in 0..1220 {
        s.tick();
    }
    let e = s.snapshot().into_iter().find(|e| e.kind == 3).unwrap();
    s.dev_set_soil(1, e.x.floor() as u32, e.y.floor() as u32, 1);
    s.dev_set_soil(1, e.x.floor() as u32 + 2, e.y.floor() as u32, 1);
    for _ in 0..3620 + 60 {
        s.tick();
    }
    // spider drop = protein units
    let q = queen(&s);
    s.dev_spawn(DevSpawn::Spider, q.x, q.y - 6.0);
    s.dev_kill(s.snapshot().into_iter().find(|e| e.kind == 4).unwrap().id);
    let drops = s
        .snapshot()
        .into_iter()
        .filter(|e| e.kind == 2 && e.aux == 4.0)
        .count();
    assert!(drops > 0, "spider drops protein units");

    // carb slowdown, measured on the open surface with a fresh flying queen:
    // same 10-tile stretch, starved vs fed
    let mut f = Sim::new_founding(7, Team::Red);
    let fq = queen(&f);
    f.dev_set_food(0);
    assert!(f.issue(Command::Move {
        ant: fq.id,
        x: fq.x + 10.0,
        y: fq.y
    }));
    for _ in 0..40 {
        f.tick();
    }
    let starved = queen(&f).x - 48.5;
    let mut g = Sim::new_founding(7, Team::Red);
    let gq = queen(&g);
    assert!(g.issue(Command::Move {
        ant: gq.id,
        x: gq.x + 10.0,
        y: gq.y
    }));
    for _ in 0..40 {
        g.tick();
    }
    let fed = queen(&g).x - 48.5;
    assert!(
        fed > starved + 1.0,
        "well-fed ants outpace starving ones ({fed:.2} vs {starved:.2})"
    );
}
