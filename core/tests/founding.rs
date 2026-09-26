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
    assert_eq!(s.world.entrance, Some((48, 48)));
    // shaft + 3x3 starter chamber carved around the founding tile
    assert_eq!(s.tile_at(Layer::Underground, 48, 48), EMPTY);
    for y in 49..=51 {
        for x in 47..=49 {
            assert_eq!(s.tile_at(Layer::Underground, x, y), EMPTY);
        }
    }
    let q = queen(&s);
    assert_eq!(q.layer, Layer::Underground as u8);
    assert!(s.tiles_dug() >= 10);
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
    assert!(s.issue(Command::DumpDirt { ant: q.id, tx, ty }));
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
    assert!(s.issue(Command::DumpDirt { ant: q.id, tx: 0, ty: 0 }));
    assert_eq!(queen(&s).aux, 0.0);
    assert_eq!(
        s.tile_at(Layer::Underground, tx, ty),
        EMPTY,
        "dug cell stays dug"
    );
}

#[test]
fn founding_timer_lays_four_eggs_then_workers_hatch() {
    let mut s = founded(42);
    // 60s window (plus a margin: DT is not binary-exact, so the timer can
    // need one extra tick to cross zero)
    for _ in 0..1220 {
        s.tick();
    }
    assert_eq!(s.colony.phase, Phase::Brood);
    assert_eq!(eggs(&s), 4);
    assert_eq!(s.colony.eggs_laid, 4);
    // FOUNDING_EGG_HATCH (180s) after laying
    for _ in 0..3620 {
        s.tick();
    }
    assert_eq!(s.colony.phase, Phase::Colony);
    assert_eq!(workers(&s), 4);
    assert_eq!(s.colony.ant_count, 5);
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
        s.issue(Command::DumpDirt { ant: queen(&s).id, tx, ty });
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
