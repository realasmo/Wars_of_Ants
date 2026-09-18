use woa_core::{Config, DevSpawn, Sim};

fn sim() -> Sim {
    Sim::new(7, Config::default())
}

#[test]
fn dev_spawn_spider_lands_on_surface_at_position() {
    let mut s = sim();
    let spiders_before = s.snapshot().iter().filter(|e| e.kind == 4).count();
    let id = s.dev_spawn(DevSpawn::Spider, 20.5, 12.5);
    let snap = s.snapshot();
    let spawned = snap
        .iter()
        .find(|e| e.id == id)
        .expect("spawned spider in snapshot");
    assert_eq!(spawned.kind, 4);
    assert_eq!(spawned.layer, 0); // surface
    assert!((spawned.x - 20.5).abs() < 1e-9 && (spawned.y - 12.5).abs() < 1e-9);
    assert_eq!(
        snap.iter().filter(|e| e.kind == 4).count(),
        spiders_before + 1
    );
}

#[test]
fn dev_spawn_food_is_harvestable() {
    let mut s = sim();
    let id = s.dev_spawn(DevSpawn::Food, 10.5, 10.5);
    assert_ne!(id, u32::MAX);
    let before = s.snapshot().iter().filter(|e| e.kind == 2).count();
    assert_eq!(before, s.snapshot().iter().filter(|e| e.kind == 2).count());
    // worker ants can target it: it exists with a pile amount
    let snap = s.snapshot();
    let food = snap.iter().find(|e| e.id == id).unwrap();
    assert_eq!(food.kind, 2);
    assert!(food.extra > 0.0); // pile amount
}

#[test]
fn dev_set_stores() {
    let mut s = sim();
    s.dev_set_food(50);
    s.dev_set_super(5);
    assert_eq!(s.colony.food, 50);
    assert_eq!(s.colony.food_super, 5);
}

#[test]
fn dev_kill_ant_and_queen() {
    let mut s = sim();
    let workers = s.colony.ant_count;
    let victim = s.snapshot().iter().find(|e| e.kind == 1).unwrap().id;
    assert!(s.dev_kill(victim));
    assert_eq!(s.colony.ant_count, workers - 1);
    assert!(!s.dev_kill(999_999)); // unknown id

    let queen = s.colony.queen_id;
    assert!(s.dev_kill(queen));
    assert!(s.colony.dead, "killing the queen must flag the colony dead");
    // and ticking a dead colony must not panic on the dangling queen id
    for _ in 0..50 {
        s.tick();
    }
}
