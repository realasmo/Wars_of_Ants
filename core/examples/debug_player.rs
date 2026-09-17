use woa_core::{Command, Config, Sim};

fn main() {
    let mut s = Sim::new(31, Config::default());
    let w = s.snapshot().iter().find(|e| e.kind == 1).unwrap().id;
    let ok = s.issue(Command::UseEntrance { ant: w });
    println!("entrance cmd ok={ok}");
    for i in 0..2000 {
        s.tick();
        if i % 100 == 0 {
            if let Some(e) = s.snapshot().iter().find(|e| e.id == w) {
                println!(
                    "pre t{}: pos=({:.2},{:.2}) layer={} state={}",
                    i, e.x, e.y, e.layer, e.state
                );
            } else {
                println!("pre t{}: worker GONE", i);
            }
        }
        if let Some(e) = s.snapshot().iter().find(|e| e.id == w) {
            if e.layer == 1 && e.state == 0 {
                println!("surfaced+idle at tick {}: ({}, {})", i, e.x, e.y);
                break;
            }
        }
    }
    let food = s
        .snapshot()
        .into_iter()
        .find(|e| e.kind == 2 && e.extra > 0.0)
        .unwrap();
    println!("food at ({}, {}) amount {}", food.x, food.y, food.extra);
    s.issue(Command::Move {
        ant: w,
        x: food.x,
        y: food.y,
    });
    for i in 0..3000 {
        s.tick();
        if i % 200 == 0 {
            if let Some(e) = s.snapshot().iter().find(|e| e.id == w) {
                println!(
                    "t{}: pos=({:.2},{:.2}) layer={} state={} carrying={}",
                    i, e.x, e.y, e.layer, e.state, e.extra
                );
            }
        }
        if let Some(e) = s.snapshot().iter().find(|e| e.id == w) {
            if e.extra > 0.5 {
                println!("CARRYING at tick {}", i);
                break;
            }
        }
    }
}
