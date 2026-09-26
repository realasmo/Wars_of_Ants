use woa_core::{Caste, Command, Config, EntitySnap, FoodKind, Sim};

fn main() {
    let cfg = Config {
        spiders: 1,
        start_workers: 8,
        food_clusters: 4,
        ..Config::default()
    };
    let mut s = Sim::new(23, cfg);
    let spider = s
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Spider(p) => Some(p.id),
            _ => None,
        })
        .unwrap();
    let mut soldier_tick = None;
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
        if soldier_tick.is_none()
            && s
                .snapshot()
                .iter()
                .any(|e| matches!(e, EntitySnap::Ant(a) if a.caste == Caste::Soldier))
        {
            soldier_tick = Some(i);
        }
    }
    let snap = s.snapshot();
    let spiders = snap.iter().filter(|e| matches!(e, EntitySnap::Spider(_))).count();
    let super_piles: Vec<u32> = snap
        .iter()
        .filter_map(|e| match e {
            EntitySnap::Food(f) if f.kind == FoodKind::Super => Some(f.amount),
            _ => None,
        })
        .collect();
    let workers = snap
        .iter()
        .filter(|e| matches!(e, EntitySnap::Ant(a) if a.caste == Caste::Worker))
        .count();
    let soldiers = snap
        .iter()
        .filter(|e| matches!(e, EntitySnap::Ant(a) if a.caste == Caste::Soldier))
        .count();
    let eggs = snap.iter().filter(|e| matches!(e, EntitySnap::Egg(_))).count();
    println!(
        "soldier_first_seen={:?} spiders_alive={} super_piles={:?} workers={} soldiers={} eggs={} dead={} food={} super_store={} eggs_laid={} delivered={}",
        soldier_tick,
        spiders,
        super_piles,
        workers,
        soldiers,
        eggs,
        s.colony.dead,
        s.colony.carbs,
        s.colony.protein,
        s.colony.eggs_laid,
        s.colony.delivered
    );
}
