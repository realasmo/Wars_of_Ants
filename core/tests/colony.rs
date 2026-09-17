use woa_core::{Command, Config, Layer, Sim, DIRT, EMPTY};

fn first_worker(s: &Sim) -> u32 {
    s.snapshot().iter().find(|e| e.kind == 1).unwrap().id
}

#[test]
fn deterministic_same_seed() {
    let mut a = Sim::new(42, Config::default());
    let mut b = Sim::new(42, Config::default());
    for _ in 0..2000 {
        a.tick();
        b.tick();
    }
    assert_eq!(a.snapshot(), b.snapshot());
    assert_eq!(a.tiles(Layer::Underground), b.tiles(Layer::Underground));
    assert_eq!(a.colony.food, b.colony.food);
}

#[test]
fn colony_grows() {
    let mut s = Sim::new(7, Config::default());
    for _ in 0..30_000 {
        s.tick();
    }
    assert!(!s.colony.dead, "colony should survive");
    assert!(
        s.colony.eggs_laid >= 2,
        "eggs_laid = {}",
        s.colony.eggs_laid
    );
    assert!(s.colony.ant_count > 3, "ant_count = {}", s.colony.ant_count);
    assert!(
        s.colony.delivered >= 30,
        "delivered = {}",
        s.colony.delivered
    );
}

#[test]
fn queen_starves_without_food() {
    let cfg = Config {
        food_clusters: 0,
        start_workers: 1,
        ..Config::default()
    };
    let mut s = Sim::new(3, cfg);
    for _ in 0..6000 {
        s.tick();
    }
    assert!(s.colony.dead, "queen should starve");
}

#[test]
fn nest_expands_when_brood_space_runs_out() {
    let cfg = Config {
        food_clusters: 12,
        max_ants: 60,
        start_workers: 4,
        ..Config::default()
    };
    let mut s = Sim::new(11, cfg);
    for _ in 0..40_000 {
        s.tick();
    }
    assert!(
        s.colony.eggs_laid > 15,
        "eggs_laid = {}",
        s.colony.eggs_laid
    );
    assert!(s.tiles_dug() > 0, "tiles_dug = {}", s.tiles_dug());
}

#[test]
fn spider_dies_to_swarm_and_soldier_is_bred() {
    let cfg = Config {
        spiders: 1,
        start_workers: 8,
        food_clusters: 4,
        ..Config::default()
    };
    let mut s = Sim::new(23, cfg);
    let spider = s
        .snapshot()
        .iter()
        .find(|e| e.kind == 4)
        .expect("spider spawned")
        .id;
    let mut soldier_seen = false;
    for i in 0..12_000 {
        if i % 50 == 0 {
            for e in s.snapshot() {
                if e.kind == 1 {
                    s.issue(Command::Attack {
                        ant: e.id,
                        target: spider,
                    });
                }
            }
        }
        s.tick();
        if s.snapshot().iter().any(|e| e.kind == 5) {
            soldier_seen = true;
        }
    }
    assert!(
        !s.snapshot().iter().any(|e| e.id == spider),
        "spider should be dead"
    );
    assert!(soldier_seen, "a soldier should have been bred");
}

#[test]
fn dig_command_digs_tile() {
    let mut s = Sim::new(1, Config::default());
    let e = s.world.entrance.0;
    assert_eq!(s.tile_at(Layer::Underground, e + 3, 2), DIRT);
    let w = first_worker(&s);
    assert!(s.issue(Command::Move {
        ant: w,
        x: (e + 2) as f64 + 0.5,
        y: 2.5,
    }));
    for _ in 0..600 {
        s.tick();
    }
    assert!(s.issue(Command::Dig {
        ant: w,
        tx: e + 3,
        ty: 2
    }));
    for _ in 0..100 {
        s.tick();
    }
    assert_eq!(s.tile_at(Layer::Underground, e + 3, 2), EMPTY);
}
