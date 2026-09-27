use woa_core::{Caste, DevSpawn, EntitySnap, GameRules, Layer, Sim};

fn sim() -> Sim {
    Sim::new(7, GameRules::default())
}

#[test]
fn dev_spawn_spider_lands_on_surface_at_position() {
    let mut s = sim();
    let spiders_before = s
        .snapshot()
        .iter()
        .filter(|e| matches!(e, EntitySnap::Spider(_)))
        .count();
    let id = s.dev_spawn(DevSpawn::Spider, 20.5, 12.5);
    let snap = s.snapshot();
    let spawned = snap
        .iter()
        .find(|e| e.id() == id)
        .expect("spawned spider in snapshot");
    assert!(matches!(spawned, EntitySnap::Spider(_)));
    assert_eq!(spawned.layer(), Layer::Surface);
    let (x, y) = spawned.pos();
    assert!((x - 20.5).abs() < 1e-9 && (y - 12.5).abs() < 1e-9);
    assert_eq!(
        snap.iter()
            .filter(|e| matches!(e, EntitySnap::Spider(_)))
            .count(),
        spiders_before + 1
    );
}

#[test]
fn dev_spawn_food_is_harvestable() {
    let mut s = sim();
    let id = s.dev_spawn(DevSpawn::Food, 10.5, 10.5);
    assert_ne!(id, u32::MAX);
    let before = s
        .snapshot()
        .iter()
        .filter(|e| matches!(e, EntitySnap::Food(_)))
        .count();
    assert_eq!(
        before,
        s.snapshot()
            .iter()
            .filter(|e| matches!(e, EntitySnap::Food(_)))
            .count()
    );
    // worker ants can target it: it exists with a pile amount
    let snap = s.snapshot();
    let food = snap
        .iter()
        .find_map(|e| match e {
            EntitySnap::Food(f) if f.id == id => Some(f),
            _ => None,
        })
        .unwrap();
    assert!(food.amount > 0); // pile amount
}

#[test]
fn dev_set_stores() {
    let mut s = sim();
    s.dev_set_food(50);
    s.dev_set_super(5);
    assert_eq!(s.colony.carbs, 50);
    assert_eq!(s.colony.protein, 5);
}

#[test]
fn dev_kill_ant_and_queen() {
    let mut s = sim();
    let workers = s.colony.ant_count;
    let victim = s
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Ant(a) if a.caste == Caste::Worker => Some(a.id),
            _ => None,
        })
        .unwrap();
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
