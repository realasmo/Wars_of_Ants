use woa_core::{Carry, Caste, Command, EntitySnap, GameRules, Layer, Sim};

fn main() {
    let mut s = Sim::new(31, GameRules::default());
    let w = s
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Ant(a) if a.caste == Caste::Worker => Some(a.id),
            _ => None,
        })
        .unwrap();
    let ok = s.issue(Command::UseEntrance { ant: w });
    println!("entrance cmd ok={ok}");
    let me = |s: &Sim| {
        s.snapshot().into_iter().find_map(|e| match e {
            EntitySnap::Ant(a) if a.id == w => Some(a),
            _ => None,
        })
    };
    for i in 0..2000 {
        s.tick();
        if i % 100 == 0 {
            match me(&s) {
                Some(a) => println!(
                    "pre t{i}: pos=({:.2},{:.2}) layer={:?} activity={:?}",
                    a.x, a.y, a.layer, a.activity
                ),
                None => println!("pre t{i}: worker GONE"),
            }
        }
        if let Some(a) = me(&s) {
            if a.layer == Layer::Surface && a.activity == woa_core::Activity::Idle {
                println!("surfaced+idle at tick {i}: ({}, {})", a.x, a.y);
                break;
            }
        }
    }
    let food = s
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Food(f) if f.amount > 0 => Some(f),
            _ => None,
        })
        .unwrap();
    println!("food at ({}, {}) amount {}", food.x, food.y, food.amount);
    s.issue(Command::Move {
        ant: w,
        x: food.x,
        y: food.y,
    });
    for i in 0..3000 {
        s.tick();
        if i % 200 == 0 {
            if let Some(a) = me(&s) {
                println!(
                    "t{i}: pos=({:.2},{:.2}) layer={:?} activity={:?} carry={:?}",
                    a.x, a.y, a.layer, a.activity, a.carry
                );
            }
        }
        if let Some(a) = me(&s) {
            if a.carry != Carry::None {
                println!("CARRYING at tick {i}");
                break;
            }
        }
    }
}
