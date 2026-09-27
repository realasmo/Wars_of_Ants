use woa_core::{
    Activity, AntSnap, Carry, Caste, Command, DevSpawn, EggSnap, EntitySnap, FoodKind, FoodRole,
    Layer, Phase, Sim, Team, DIRT, EMPTY, SOURCES, START_FOOD,
};

fn queen(s: &Sim) -> AntSnap {
    s.snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Ant(a) if a.caste == Caste::Queen => Some(a),
            _ => None,
        })
        .unwrap()
}

fn eggs(s: &Sim) -> usize {
    s.snapshot()
        .iter()
        .filter(|e| matches!(e, EntitySnap::Egg(_)))
        .count()
}

fn workers(s: &Sim) -> usize {
    s.snapshot()
        .iter()
        .filter(|e| matches!(e, EntitySnap::Ant(a) if a.caste == Caste::Worker))
        .count()
}

fn first_egg(s: &Sim) -> EggSnap {
    s.snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Egg(e) => Some(e),
            _ => None,
        })
        .unwrap()
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
    assert_eq!(
        queen(s).carry,
        Carry::Dirt { blocks: 1 },
        "queen should haul the dug block"
    );
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
    assert_eq!(q.layer, Layer::Surface);
    assert_eq!(q.activity, Activity::Flying);
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
    assert!(!s.issue(Command::Land {
        ant: q.id,
        x: q.x,
        y: q.y
    }));
}

#[test]
fn found_nest_creates_entrance_and_chamber() {
    let mut s = Sim::new_founding(42, Team::Red);
    let q = queen(&s);
    assert!(s.issue(Command::Land { ant: q.id, x: q.x, y: q.y }));
    assert!(s.issue(Command::FoundNest {
        ant: q.id,
        x: q.x,
        y: q.y
    }));
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
    assert_eq!(q.layer, Layer::Underground);
    assert!(s.tiles_dug() >= 20);
    // one nest per game
    assert!(!s.issue(Command::FoundNest {
        ant: q.id,
        x: q.x,
        y: q.y
    }));
}

#[test]
fn found_nest_refused_near_map_border() {
    let mut s = Sim::new_founding(42, Team::Red);
    let q = queen(&s);
    assert!(s.issue(Command::Land { ant: q.id, x: q.x, y: q.y }));
    // walk to the bottom rows where the starter chamber would not fit
    assert!(s.issue(Command::Move {
        ant: q.id,
        x: 48.5,
        y: 92.5
    }));
    for _ in 0..420 {
        s.tick();
    }
    let q = queen(&s);
    assert!(!s.issue(Command::FoundNest {
        ant: q.id,
        x: q.x,
        y: q.y
    }));
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
    assert_eq!(queen(&s).carry, Carry::None);
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
    assert_eq!(queen(&s).layer, Layer::Surface);
    assert!(s.issue(Command::Drop { ant: q.id, tx: 0, ty: 0 }));
    assert_eq!(queen(&s).carry, Carry::None);
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
    let egg = first_egg(&s);
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
fn solo_founding_queen_does_not_burn_the_reserves() {
    let mut s = founded(42);
    // the whole solo window (60s excavation + 180s incubation) passes with
    // no possible income — the reserves must survive to meet the workforce
    for _ in 0..(235 * 20) {
        s.tick();
    }
    assert!(
        !s.colony.dead,
        "lone founding queen must not die during incubation"
    );
    assert_eq!(
        s.colony.carbs, START_FOOD,
        "the solo queen must not eat through the founding reserves"
    );
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
        s.issue(Command::FoundNest {
            ant: q.id,
            x: q.x,
            y: q.y
        });
        for _ in 0..60 {
            s.tick();
        }
        let (tx, ty) = walk_and_dig(&mut s, 60);
        s.issue(Command::Drop {
            ant: queen(&s).id,
            tx,
            ty,
        });
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
    assert_eq!(
        queen(&s).carry,
        Carry::Dirt { blocks: 1 },
        "queen hauls one dirt block"
    );
}

#[test]
fn eggs_can_be_carried_and_placed_keeping_hatch_state() {
    let mut s = founded(42);
    for _ in 0..1220 {
        s.tick();
    }
    let q = queen(&s);
    let egg = first_egg(&s);
    let frac0 = egg.hatch_left; // remaining incubation fraction
    assert!(s.issue(Command::PickEgg { ant: q.id, egg: egg.id }));
    // carried: the snapshot flags it and the queen carries the egg
    let s1 = s.snapshot();
    let e1 = s1
        .iter()
        .find_map(|e| match e {
            EntitySnap::Egg(e) if e.id == egg.id => Some(e),
            _ => None,
        })
        .unwrap();
    assert!(e1.carried, "egg is carried");
    assert_eq!(queen(&s).carry, Carry::Egg);
    // no digging while hands are full
    let ((_, _), (tx, ty)) = dig_site(&s);
    assert!(!s.issue(Command::Dig { ant: q.id, tx, ty }));
    // place it on an adjacent empty chamber cell
    let (qx, qy) = (q.x.floor() as u32, q.y.floor() as u32);
    let spot = [(qx + 1, qy), (qx - 1, qy), (qx, qy + 1), (qx, qy - 1)]
        .into_iter()
        .find(|&(x, y)| s.tile_at(Layer::Underground, x, y) == EMPTY)
        .unwrap();
    assert!(s.issue(Command::Drop {
        ant: q.id,
        tx: spot.0,
        ty: spot.1
    }));
    let e2 = s
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Egg(e) if e.id == egg.id => Some(e),
            _ => None,
        })
        .unwrap();
    assert!(!e2.carried, "egg placed");
    assert!((e2.hatch_left - frac0).abs() < 0.05, "hatch state preserved");
}

#[test]
fn dropped_food_spoils_off_silver_and_freezes_on_it() {
    // (the founding queen never auto-picks food — use a legacy worker)
    let mut s = Sim::new(3, woa_core::Config::default());
    let w = s
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Ant(a) if a.caste == Caste::Worker => Some(a.id),
            _ => None,
        })
        .unwrap();
    // drop 1 unit far from any soil: it must spoil after SPOIL_TIME
    // (worker picks food up on the surface via a manual move onto a pile)
    let pile = s
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Food(f) => Some(f),
            _ => None,
        })
        .expect("legacy worldgen has surface piles");
    // cross to the surface first (Move is same-layer), then walk to the pile
    assert!(s.issue(Command::UseEntrance { ant: w }));
    for _ in 0..100 {
        s.tick();
    }
    assert!(s.issue(Command::Move {
        ant: w,
        x: pile.x,
        y: pile.y
    }));
    for _ in 0..400 {
        s.tick();
    }
    let carrying = s
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Ant(a) if a.carry != Carry::None => Some(a),
            _ => None,
        })
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
    let spoiled = !s.snapshot().iter().any(|e| {
        matches!(
            e,
            EntitySnap::Food(woa_core::FoodSnap {
                role: FoodRole::Loose { spoil: Some(_) },
                ..
            })
        )
    });
    assert!(spoiled, "dropped food must spoil off silver");
}

#[test]
fn food_piles_respect_cell_cap() {
    let mut s = Sim::new_founding(1337, Team::Red);
    // (fresh seed's own piles sit near the map center; spawn far away at 30,30)
    // worldgen sources are everywhere now — only count green units near the
    // dev drop point
    let green_near = |snaps: &[EntitySnap]| -> Vec<woa_core::FoodSnap> {
        snaps
            .iter()
            .filter_map(|e| match e {
                EntitySnap::Food(f)
                    if f.kind == FoodKind::Green
                        && (f.x - 30.5).abs() <= 12.0
                        && (f.y - 30.5).abs() <= 12.0 =>
                {
                    Some(f.clone())
                }
                _ => None,
            })
            .collect()
    };
    let before = green_near(&s.snapshot()).len();
    s.dev_spawn(DevSpawn::Food, 30.5, 30.5);
    let piles = green_near(&s.snapshot());
    assert!(piles.len() > before, "spreading created cells");
    assert!(!piles.is_empty());
    for p in &piles {
        assert!(p.amount <= 6, "pile of {} exceeds the cell cap", p.amount);
    }
    let total: u32 = piles.iter().map(|p| p.amount).sum();
    assert_eq!(total, 45, "no food lost to spreading");
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
    assert!(s.issue(Command::Dig {
        ant: q.id,
        tx: b_tx,
        ty: b_ty
    }));
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
    assert!(s.issue(Command::Drop {
        ant: q.id,
        tx: b_tx,
        ty: b_ty
    }));
    assert_eq!(
        queen(&s).carry,
        Carry::Dirt { blocks: 1 },
        "still hauling one block after the first dump"
    );
    // walk back into the dug-out block A and refill it
    assert!(s.issue(Command::Move {
        ant: q.id,
        x: a_tx as f64 + 0.5,
        y: a_ty as f64 + 0.5
    }));
    for _ in 0..140 {
        s.tick();
    }
    assert!(s.issue(Command::Drop {
        ant: q.id,
        tx: a_tx,
        ty: a_ty
    }));
    assert_eq!(queen(&s).carry, Carry::None, "all dirt dumped");
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
                        !s.issue(Command::Dig {
                            ant: q.id,
                            tx: dbx,
                            ty: dby
                        }),
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
    assert!(s.issue(Command::Move {
        ant: q.id,
        x: sx as f64 + 0.5,
        y: sy as f64 + 0.5
    }));
    for _ in 0..80 {
        s.tick();
    }
    assert!(s.issue(Command::Dig {
        ant: q.id,
        tx: tx2,
        ty: ty2
    }));
}

#[test]
fn soil_blocks_align_with_dig_blocks_and_are_diggable() {
    let s = Sim::new_founding(42, Team::Red);
    let w = s.config.width;
    // scan even block origins only — odd cells belong to even blocks
    for by in (0..s.config.height).step_by(2) {
        for bx in (0..w).step_by(2) {
            let soil = s.world.soil_underground[(by * w + bx) as usize];
            if soil != 0 {
                // alignment: the soil block must be exactly one dig block
                assert_eq!((bx & !1, by & !1), (bx, by), "soil block at ({bx},{by}) is off the dig grid");
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
    let eggs_all: Vec<EggSnap> = s
        .snapshot()
        .into_iter()
        .filter_map(|e| match e {
            EntitySnap::Egg(e) => Some(e),
            _ => None,
        })
        .collect();
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
    assert!(s.issue(Command::PickEgg {
        ant: q.id,
        egg: egg.id
    }));
    for _ in 0..300 {
        s.tick();
    }
    assert_eq!(
        queen(&s).carry,
        Carry::Egg,
        "queen should reach and pick up the egg"
    );
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
    assert!(s.issue(Command::Drop {
        ant: q.id,
        tx: sx,
        ty: sy
    }));
    for _ in 0..400 {
        s.tick();
    }
    assert_eq!(queen(&s).carry, Carry::None, "egg should be placed after walking");
    let placed = s
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Egg(e) if e.id == egg.id => Some(e),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        (placed.x.floor() as u32, placed.y.floor() as u32),
        (sx, sy)
    );
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
    assert!(s.issue(Command::FoundNest {
        ant: q.id,
        x: fx,
        y: fy
    }));
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
    let foods: Vec<&woa_core::FoodSnap> = snaps
        .iter()
        .filter_map(|e| match e {
            EntitySnap::Food(f) => Some(f),
            _ => None,
        })
        .collect();
    // no green piles anywhere
    assert!(
        !foods.iter().any(|f| f.kind == FoodKind::Green),
        "founding world must not contain green food"
    );
    let sources: Vec<&woa_core::FoodSnap> = foods
        .iter()
        .copied()
        .filter(|f| matches!(f.role, FoodRole::Source { .. }))
        .collect();
    let total: u32 = SOURCES.iter().map(|sp| sp.count).sum();
    assert_eq!(sources.len() as u32, total, "all six source types placed");
    let carbs_near = sources
        .iter()
        .filter(|f| f.kind == FoodKind::Carbs && (f.x - 48.5).abs() <= 16.0 && (f.y - 48.5).abs() <= 16.0)
        .count();
    assert!(carbs_near >= 1, "at least one carb source near the founding center");
    // deterministic per seed
    let b = Sim::new_founding(42, Team::Blue);
    let sa: Vec<_> = sources.iter().map(|f| (f.id, f.x as u32, f.y as u32, f.amount)).collect();
    let sb: Vec<_> = b
        .snapshot()
        .iter()
        .filter_map(|e| match e {
            EntitySnap::Food(f) if matches!(f.role, FoodRole::Source { .. }) => {
                Some((f.id, f.x as u32, f.y as u32, f.amount))
            }
            _ => None,
        })
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
    let e = first_egg(&s);
    s.dev_set_soil(1, e.x.floor() as u32, e.y.floor() as u32, 1);
    s.dev_set_soil(1, e.x.floor() as u32 + 2, e.y.floor() as u32, 1);
    for _ in 0..3620 + 60 {
        s.tick();
    }
    assert_eq!(phase(&s), Phase::Colony);
    // dev-spawn a strawberry (4s harvest) far from the nest, unknown —
    // pick a spot clear of worldgen sources (their placement shifted with
    // the soil-alignment fix)
    let q = queen(&s);
    let mut sx = q.x + 20.0;
    let mut sy = q.y + 15.0;
    let clear = |snaps: &[EntitySnap], x: f64, y: f64| {
        !snaps.iter().any(
            |e| matches!(e, EntitySnap::Food(f) if matches!(f.role, FoodRole::Source { .. }) && (f.x - x).abs() < 4.0 && (f.y - y).abs() < 4.0),
        )
    };
    while !clear(&s.snapshot(), sx, sy) {
        sx += 5.0;
        sy += 3.0;
    }
    s.dev_spawn(DevSpawn::Source(4), sx, sy);
    let src = s
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Food(f)
                if matches!(f.role, FoodRole::Source { .. })
                    && (f.x - sx).abs() < 1.0
                    && (f.y - sy).abs() < 1.0 =>
            {
                Some(f)
            }
            _ => None,
        })
        .unwrap();
    let amount0 = src.amount;
    // not discovered: no worker fetches it while it's out of sight — verify
    // via knowledge set indirectly: a nearby ant discovers it instantly
    let w = s
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Ant(a) if a.caste == Caste::Worker => Some(a.id),
            _ => None,
        })
        .unwrap();
    assert!(s.issue(Command::Move { ant: w, x: sx, y: sy }));
    for _ in 0..10 {
        s.tick();
    }
    // within sight now; give the worker time to arrive and harvest (4s/unit)
    for _ in 0..500 {
        s.tick();
    }
    let after: Option<u32> = s
        .snapshot()
        .iter()
        .filter_map(|e| match e {
            EntitySnap::Food(f)
                if matches!(f.role, FoodRole::Source { .. })
                    && (f.x - sx).abs() < 1.5
                    && (f.y - sy).abs() < 1.5 =>
            {
                Some(f.amount)
            }
            _ => None,
        })
        .next();
    assert!(
        after.map(|a| a < amount0).unwrap_or(true),
        "harvesting consumed units from the source ({after:?} vs {amount0})"
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
        gone = !s.snapshot().iter().any(|e| {
            matches!(e, EntitySnap::Food(f) if matches!(f.role, FoodRole::Source { .. }) && (f.x - ex as f64 - 3.5).abs() < 1.0)
        });
        if gone {
            break;
        }
    }
    assert!(gone, "depleted sources despawn (finite map resources)");
}

#[test]
fn spider_drops_protein() {
    let mut s = founded(42);
    for _ in 0..1220 {
        s.tick();
    }
    let e = first_egg(&s);
    s.dev_set_soil(1, e.x.floor() as u32, e.y.floor() as u32, 1);
    s.dev_set_soil(1, e.x.floor() as u32 + 2, e.y.floor() as u32, 1);
    for _ in 0..3620 + 60 {
        s.tick();
    }
    // spider drop = protein units
    let q = queen(&s);
    s.dev_spawn(DevSpawn::Spider, q.x, q.y - 6.0);
    let spider = s
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Spider(p) => Some(p.id),
            _ => None,
        })
        .unwrap();
    s.dev_kill(spider);
    let drops = s
        .snapshot()
        .iter()
        .filter(|e| {
            matches!(
                e,
                EntitySnap::Food(f) if f.kind == FoodKind::Protein
            )
        })
        .count();
    assert!(drops > 0, "spider drops protein units");
}

#[test]
fn event_log_records_causes_and_discoveries() {
    let mut s = Sim::new_founding(42, Team::Red);
    // run a founding flow and check the story reads back
    let q = queen(&s);
    s.issue(Command::Land { ant: q.id, x: q.x, y: q.y });
    s.issue(Command::FoundNest {
        ant: q.id,
        x: q.x,
        y: q.y,
    });
    // through founding (60s) + the full brood timer (180s): eggs go ready
    for _ in 0..4900 {
        s.tick();
    }
    let log = s.event_lines().join("\n");
    assert!(log.contains("queen landed"), "landed event: {log}");
    assert!(log.contains("nest founded"), "founded event: {log}");
    assert!(log.contains("brood laid"), "brood event: {log}");
    assert!(log.contains("egg #"), "egg laid events: {log}");
    // egg ready + waiting for orange (no nursery in this flow)
    assert!(log.contains("waiting for orange"), "waiting event: {log}");
    // death with cause: kill the queen via dev
    let total_before = s.event_total();
    s.dev_kill(s.colony.queen_id);
    for _ in 0..5 {
        s.tick();
    }
    let log2 = s.event_lines().join("\n");
    assert!(log2.contains("COLONY DIED"), "colony death event: {log2}");
    assert!(log2.contains("(dev)"), "death cause recorded: {log2}");
    assert!(s.event_total() > total_before);
    // combat cause: found a sim, spawn a spider on the grounded queen
    let mut c = Sim::new_founding(42, Team::Red);
    let q2 = queen(&c);
    c.issue(Command::Land {
        ant: q2.id,
        x: q2.x,
        y: q2.y,
    });
    c.dev_spawn(DevSpawn::Spider, queen(&c).x, queen(&c).y);
    for _ in 0..600 {
        c.tick();
    }
    let log3 = c.event_lines().join("\n");
    assert!(
        log3.contains("COLONY DIED") && log3.contains("(combat)"),
        "combat cause recorded: {log3}"
    );
}

#[test]
fn worker_dig_collects_dirt_then_auto_dumps() {
    let mut s = founded(42);
    let q = queen(&s);
    let wid = s.dev_spawn(DevSpawn::Worker, q.x, q.y);
    let worker = |s: &Sim| {
        s.snapshot()
            .into_iter()
            .find_map(|e| match e {
                EntitySnap::Ant(a) if a.id == wid => Some((a.carry, a.layer)),
                _ => None,
            })
            .unwrap()
    };
    // a commanded dig collects the spoil (the auto-haul may start the moment
    // the dig completes — sample for the dirt rather than asserting a pose)
    let ((sx, sy), (tx, ty)) = dig_site(&s);
    assert!(s.issue(Command::Move {
        ant: wid,
        x: sx as f64 + 0.5,
        y: sy as f64 + 0.5
    }));
    for _ in 0..100 {
        s.tick();
    }
    assert!(s.issue(Command::Dig { ant: wid, tx, ty }));
    let mut saw_dirt = false;
    for _ in 0..250 {
        s.tick();
        if matches!(worker(&s).0, Carry::Dirt { .. }) {
            saw_dirt = true;
        }
    }
    assert!(saw_dirt, "worker collects dug dirt");
    // the spoil is auto-hauled out and discarded above ground — hands end
    // empty and the worker is back inside or on the surface, never frozen
    let mut dumped = false;
    for _ in 0..1200 {
        s.tick();
        let (carry, layer) = worker(&s);
        if carry == Carry::None && matches!(worker(&s).0, Carry::None) {
            if layer == Layer::Surface || layer == Layer::Underground {
                dumped = true;
                break;
            }
        }
    }
    assert!(dumped, "worker auto-dumps the spoil after digging");
}

#[test]
fn harvesting_is_a_visible_activity_and_fills_mandibles() {
    let mut s = founded(42);
    let q = queen(&s);
    let wid = s.dev_spawn(DevSpawn::Worker, q.x, q.y);
    let snap = |s: &Sim, id: u32| {
        s.snapshot()
            .into_iter()
            .find_map(|e| match e {
                EntitySnap::Ant(a) if a.id == id => Some((a.activity, a.carry)),
                _ => None,
            })
            .unwrap()
    };
    // a strawberry beside the entrance (4s/unit — the Harvesting state is
    // visible for the whole pickup)
    let (ex, ey) = s.world.entrance.unwrap();
    s.dev_spawn(DevSpawn::Source(4), ex as f64 + 1.5, ey as f64 - 1.5);
    // moves are same-layer: hop the entrance first, then walk to the source
    assert!(s.issue(Command::UseEntrance { ant: wid }));
    for _ in 0..400 {
        s.tick();
        let on_surface = s
            .snapshot()
            .into_iter()
            .any(|e| matches!(e, EntitySnap::Ant(a) if a.id == wid && a.layer == Layer::Surface));
        if on_surface {
            break;
        }
    }
    assert!(s.issue(Command::Move {
        ant: wid,
        x: ex as f64 + 1.5,
        y: ey as f64 - 1.5
    }));
    let mut saw_harvesting = false;
    let mut farmed = false;
    for _ in 0..1500 {
        s.tick();
        let (act, carry) = snap(&s, wid);
        if act == Activity::Harvesting {
            saw_harvesting = true;
        }
        if matches!(carry, Carry::Food(_)) {
            farmed = true;
            break;
        }
    }
    assert!(saw_harvesting, "harvesting shows as an activity");
    assert!(farmed, "completed unit lands in the mandibles");
}

#[test]
fn queen_farms_and_banks_on_silver() {
    let mut s = founded(42);
    let q = queen(&s);
    let (ex, ey) = s.world.entrance.unwrap();
    // strawberry by the hole; the queen crosses, farms one unit, comes
    // home and banks it on a painted silver cell
    s.dev_spawn(DevSpawn::Source(4), ex as f64 + 1.5, ey as f64 - 1.5);
    assert!(s.issue(Command::UseEntrance { ant: q.id }));
    for _ in 0..400 {
        s.tick();
        if queen(&s).layer == Layer::Surface {
            break;
        }
    }
    assert!(s.issue(Command::Move {
        ant: q.id,
        x: ex as f64 + 1.5,
        y: ey as f64 - 1.5
    }));
    let mut harvested = false;
    for _ in 0..800 {
        s.tick();
        if matches!(queen(&s).carry, Carry::Food(_)) {
            harvested = true;
            break;
        }
    }
    assert!(harvested, "the queen can farm a source herself");

    // paint silver inside the chamber and walk her onto it
    s.dev_set_soil(1, ex + 1, ey + 4, 2);
    assert!(s.issue(Command::UseEntrance { ant: q.id }));
    for _ in 0..400 {
        s.tick();
        if queen(&s).layer == Layer::Underground {
            break;
        }
    }
    assert!(s.issue(Command::Move {
        ant: q.id,
        x: ex as f64 + 1.5,
        y: ey as f64 + 4.5
    }));
    let before = s.colony.carbs;
    for _ in 0..600 {
        s.tick();
        if queen(&s).carry == Carry::None {
            break;
        }
    }
    assert_eq!(queen(&s).carry, Carry::None, "queen banks her haul");
    assert!(s.colony.carbs > before, "the banked unit lands in the store");
}
