use woa_core::{Command, Config, Sim};

fn main() {
    let cfg = Config {
        spiders: 1,
        start_workers: 8,
        food_clusters: 4,
        ..Config::default()
    };
    let mut s = Sim::new(23, cfg);
    let spider = s.snapshot().iter().find(|e| e.kind == 4).unwrap().id;
    let mut soldier_tick = None;
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
        if soldier_tick.is_none() && s.snapshot().iter().any(|e| e.kind == 5) {
            soldier_tick = Some(i);
        }
    }
    let snap = s.snapshot();
    let spiders = snap.iter().filter(|e| e.kind == 4).count();
    let super_piles: Vec<f64> = snap
        .iter()
        .filter(|e| e.kind == 2 && e.aux == 1.0)
        .map(|e| e.extra)
        .collect();
    let workers = snap.iter().filter(|e| e.kind == 1).count();
    let soldiers = snap.iter().filter(|e| e.kind == 5).count();
    let eggs = snap.iter().filter(|e| e.kind == 3).count();
    println!(
        "soldier_first_seen={:?} spiders_alive={} super_piles={:?} workers={} soldiers={} eggs={} dead={} food={} super_store={} eggs_laid={} delivered={}",
        soldier_tick,
        spiders,
        super_piles,
        workers,
        soldiers,
        eggs,
        s.colony.dead,
        s.colony.food,
        s.colony.food_super,
        s.colony.eggs_laid,
        s.colony.delivered
    );
}
