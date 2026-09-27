use woa_core::{Carry, Caste, Command, EntitySnap, GameRules, Layer, Sim, DIRT, EMPTY};

fn first_worker(s: &Sim) -> u32 {
    s.snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Ant(a) if a.caste == Caste::Worker => Some(a.id),
            _ => None,
        })
        .unwrap()
}

fn ant_by_id(s: &Sim, id: u32) -> Option<woa_core::AntSnap> {
    s.snapshot().into_iter().find_map(|e| match e {
        EntitySnap::Ant(a) if a.id == id => Some(a),
        _ => None,
    })
}

#[test]
fn deterministic_same_seed() {
    let mut a = Sim::new(42, GameRules::default());
    let mut b = Sim::new(42, GameRules::default());
    for _ in 0..2000 {
        a.tick();
        b.tick();
    }
    assert_eq!(a.snapshot(), b.snapshot());
    assert_eq!(a.tiles(Layer::Underground), b.tiles(Layer::Underground));
    assert_eq!(a.colony.carbs, b.colony.carbs);
}

#[test]
fn colony_grows() {
    // 12 clusters: enough finite food that survival at the 30k-tick horizon is
    // guaranteed, not seed luck (every legacy world starves once its piles
    // run out — the queen alone eats ~1200 carbs in 30k ticks)
    let cfg = GameRules {
        food_clusters: 12,
        ..GameRules::default()
    };
    let mut s = Sim::new(7, cfg);
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
    let cfg = GameRules {
        food_clusters: 0,
        start_workers: 1,
        ..GameRules::default()
    };
    let mut s = Sim::new(3, cfg);
    for _ in 0..6000 {
        s.tick();
    }
    assert!(s.colony.dead, "queen should starve");
}

#[test]
fn nest_expands_when_brood_space_runs_out() {
    let cfg = GameRules {
        food_clusters: 12,
        max_ants: 60,
        start_workers: 4,
        ..GameRules::default()
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
    let cfg = GameRules {
        spiders: 1,
        start_workers: 8,
        food_clusters: 4,
        ..GameRules::default()
    };
    let mut s = Sim::new(23, cfg);
    let spider = s
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Spider(p) => Some(p.id),
            _ => None,
        })
        .expect("spider spawned");
    let mut soldier_seen = false;
    for i in 0..12_000 {
        if i % 50 == 0 {
            for e in s.snapshot() {
                if let EntitySnap::Ant(a) = e {
                    if a.caste == Caste::Worker {
                        s.issue(Command::Attack {
                            ant: a.id,
                            target: spider,
                        });
                    }
                }
            }
        }
        s.tick();
        if s.snapshot()
            .iter()
            .any(|e| matches!(e, EntitySnap::Ant(a) if a.caste == Caste::Soldier))
        {
            soldier_seen = true;
        }
    }
    assert!(
        !s.snapshot().iter().any(|e| e.id() == spider),
        "spider should be dead"
    );
    assert!(soldier_seen, "a soldier should have been bred");
}

#[test]
fn player_loop_entrance_pickup_deposit() {
    let mut s = Sim::new(31, GameRules::default());
    let w = first_worker(&s);
    assert!(s.issue(Command::UseEntrance { ant: w }));
    let mut surfaced = false;
    for _ in 0..2000 {
        s.tick();
        if let Some(a) = ant_by_id(&s, w) {
            if a.layer == Layer::Surface {
                surfaced = true;
                break;
            }
        }
    }
    assert!(surfaced, "worker should reach the surface via entrance");

    let food_snap = s
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Food(f) if f.amount > 0 => Some(f),
            _ => None,
        })
        .unwrap();
    let (fx, fy) = (food_snap.x, food_snap.y);
    assert!(s.issue(Command::Move {
        ant: w,
        x: fx,
        y: fy,
    }));
    let mut carrying = false;
    for _ in 0..3000 {
        s.tick();
        if let Some(a) = ant_by_id(&s, w) {
            if a.carry != Carry::None {
                carrying = true;
                break;
            }
        }
    }
    assert!(carrying, "manual worker should auto-pick food it stands on");

    assert!(s.issue(Command::UseEntrance { ant: w }));
    let mut home = false;
    for _ in 0..3000 {
        s.tick();
        if let Some(a) = ant_by_id(&s, w) {
            if a.layer == Layer::Underground {
                home = true;
                break;
            }
        }
    }
    assert!(home, "worker should return underground");

    let q = s.colony.queen_id;
    let delivered_before = s.colony.delivered;
    let q_snap = ant_by_id(&s, q).unwrap();
    assert!(s.issue(Command::Move {
        ant: w,
        x: q_snap.x,
        y: q_snap.y,
    }));
    let mut deposited = false;
    for _ in 0..2000 {
        s.tick();
        if s.colony.delivered > delivered_before {
            if let Some(a) = ant_by_id(&s, w) {
                if a.carry == Carry::None {
                    deposited = true;
                    break;
                }
            }
        }
    }
    assert!(deposited, "manual worker should deposit food at the queen");
}

#[test]
fn dig_command_digs_tile() {
    let mut s = Sim::new(1, GameRules::default());
    let e = s.world.entrance.unwrap().0;
    assert_eq!(s.tile_at(Layer::Underground, e + 3, 2), DIRT);
    let w = first_worker(&s);
    // stand inside the chamber (a block dig target must not overlap the walk
    // path, or the worker auto-digs it on the way there)
    assert!(s.issue(Command::Move {
        ant: w,
        x: (e + 1) as f64 + 0.5,
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
    // digging removes the whole aligned 2x2 block
    assert_eq!(s.tile_at(Layer::Underground, e + 2, 2), EMPTY);
    assert_eq!(s.tile_at(Layer::Underground, e + 3, 2), EMPTY);
    assert_eq!(s.tile_at(Layer::Underground, e + 2, 3), EMPTY);
    assert_eq!(s.tile_at(Layer::Underground, e + 3, 3), EMPTY);
}
