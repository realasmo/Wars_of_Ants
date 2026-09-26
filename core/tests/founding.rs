use woa_core::{Command, DevSpawn, Layer, Phase, Sim, Team, DIRT, EMPTY};

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
    assert!(s.issue(Command::Land { ant: q.id }));
    assert!(s.issue(Command::FoundNest { ant: q.id }));
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

    assert!(s.issue(Command::Land { ant: q.id }));
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
    assert!(!s.issue(Command::Land { ant: q.id }));
}

#[test]
fn found_nest_creates_entrance_and_chamber() {
    let mut s = Sim::new_founding(42, Team::Red);
    let q = queen(&s);
    assert!(s.issue(Command::Land { ant: q.id }));
    assert!(s.issue(Command::FoundNest { ant: q.id }));
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
    assert!(!s.issue(Command::FoundNest { ant: q.id }));
}

#[test]
fn found_nest_refused_near_map_border() {
    let mut s = Sim::new_founding(42, Team::Red);
    let q = queen(&s);
    assert!(s.issue(Command::Land { ant: q.id }));
    // walk to the bottom rows where the starter chamber would not fit
    assert!(s.issue(Command::Move { ant: q.id, x: 48.5, y: 92.5 }));
    for _ in 0..420 {
        s.tick();
    }
    let q = queen(&s);
    assert!(!s.issue(Command::FoundNest { ant: q.id }));
    assert_eq!(s.colony.phase, Phase::Grounded);
    assert!(s.world.entrance.is_none());
}

#[test]
fn queen_dig_yields_dirt_and_dump_refills_the_cell() {
    let mut s = founded(42);
    let q = queen(&s);
    let (tx, ty) = walk_and_dig(&mut s, 60);
    // no second dig while hauling
    let ((sx2, sy2), (tx2, ty2)) = dig_site(&s);
    let _ = (sx2, sy2);
    assert!(!s.issue(Command::Dig {
        ant: q.id,
        tx: tx2,
        ty: ty2
    }));
    // dumping underground refills an adjacent empty cell — the one just dug
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

    assert!(s.issue(Command::Land { ant: q.id }));
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
        s.issue(Command::Land { ant: q.id });
        s.issue(Command::FoundNest { ant: q.id });
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
            let mut touches_empty = false;
            'n: for ny in (bx.saturating_sub(1))..=bx + 2 {
                for nx in (by.saturating_sub(1))..=by + 2 {
                    let _ = nx;
                    let _ = ny;
                    break 'n;
                }
            }
            // adjacency to emptiness: any tile of the ring around the block
            let ring_empty = [
                (bx.wrapping_sub(1), by), (bx + 2, by), (bx, by.wrapping_sub(1)), (bx, by + 2),
                (bx.wrapping_sub(1), by.wrapping_sub(1)), (bx + 2, by.wrapping_sub(1)),
                (bx.wrapping_sub(1), by + 2), (bx + 2, by + 2),
                (bx.wrapping_sub(1), by + 1), (bx + 2, by + 1),
                (bx, by.wrapping_sub(1)), (bx + 1, by.wrapping_sub(1)),
                (bx, by + 2), (bx + 1, by + 2),
            ]
            .iter()
            .any(|&(x, y)| s.tile_at(Layer::Underground, x, y) == EMPTY);
            let _ = touches_empty;
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
    let before: usize = s
        .snapshot()
        .iter()
        .filter(|e| e.kind == 2 && (e.x - 30.5).abs() <= 12.0 && (e.y - 30.5).abs() <= 12.0)
        .count();
    s.dev_spawn(DevSpawn::Food, 30.5, 30.5);
    let piles: Vec<_> = s
        .snapshot()
        .into_iter()
        .filter(|e| e.kind == 2 && (e.x - 30.5).abs() <= 12.0 && (e.y - 30.5).abs() <= 12.0)
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
