use woa_core::{GameRules, Sim};
fn main() {
    let cfg = GameRules {
        food_clusters: 12,
        max_ants: 200,
        start_workers: 4,
        ..GameRules::default()
    };
    let mut s = Sim::new(11, cfg);
    for _ in 0..40_000 {
        s.tick();
    }
    println!(
        "eggs={} dug={} ants={} delivered={} food={} dead={}",
        s.colony.eggs_laid,
        s.tiles_dug(),
        s.colony.ant_count,
        s.colony.delivered,
        s.colony.carbs,
        s.colony.dead
    );
    let eggs = s
        .snapshot()
        .iter()
        .filter(|e| matches!(e, woa_core::EntitySnap::Egg(_)))
        .count();
    println!("live eggs={}", eggs);
}
