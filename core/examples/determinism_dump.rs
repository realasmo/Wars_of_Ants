// Runs a replay file (client/public/replays/*.json) headless on the native
// build and prints the canonical state. The e2e suite runs the same replay
// in the browser (WASM build) and compares the two strings — any difference
// is a cross-platform determinism break.
//
// Usage: cargo run --example determinism_dump -- [path/to/replay.json]

use std::fs;

use woa_core::{Command, Config, DevSpawn, Sim};

enum Step {
    Cmd(Command),
    Spawn(DevSpawn, f64, f64),
    SetFood(u32),
    SetSuper(u32),
    KillSpiders,
    Kill(u32),
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "client/public/replays/determinism.json".to_string());
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let replay: serde_json::Value = serde_json::from_str(&raw).expect("parse replay json");
    let seed = replay["seed"].as_u64().expect("seed") as u64;
    let ticks = replay["ticks"].as_u64().expect("ticks") as u64;
    let cmds: Vec<(u64, Step)> = replay["cmds"]
        .as_array()
        .expect("cmds")
        .iter()
        .filter_map(|v| {
            let obj = v.as_object()?;
            let t = obj.get("t")?.as_u64()?;
            let ant = || -> Option<u32> { Some(obj.get("ant")?.as_u64()? as u32) };
            let step = match obj.get("act")?.as_str()? {
                "move" => Step::Cmd(Command::Move {
                    ant: ant()?,
                    x: obj.get("x")?.as_f64()?,
                    y: obj.get("y")?.as_f64()?,
                }),
                "dig" => Step::Cmd(Command::Dig {
                    ant: ant()?,
                    tx: obj.get("tx")?.as_u64()? as u32,
                    ty: obj.get("ty")?.as_u64()? as u32,
                }),
                "attack" => Step::Cmd(Command::Attack {
                    ant: ant()?,
                    target: obj.get("target")?.as_u64()? as u32,
                }),
                "entrance" => Step::Cmd(Command::UseEntrance { ant: ant()? }),
                "dev-spawn" => Step::Spawn(
                    DevSpawn::parse(obj.get("kind")?.as_str()?)?,
                    obj.get("x")?.as_f64()?,
                    obj.get("y")?.as_f64()?,
                ),
                "dev-food" => Step::SetFood(obj.get("n")?.as_u64()? as u32),
                "dev-super" => Step::SetSuper(obj.get("n")?.as_u64()? as u32),
                "dev-kill-spiders" => Step::KillSpiders,
                "dev-kill" => Step::Kill(obj.get("target")?.as_u64()? as u32),
                _ => return None,
            };
            Some((t, step))
        })
        .collect();

    let mut sim = Sim::new(seed, Config::default());
    let mut i = 0usize;
    for _ in 0..ticks {
        sim.tick();
        while i < cmds.len() && cmds[i].0 <= sim.tick {
            match &cmds[i].1 {
                Step::Cmd(c) => {
                    sim.issue(c.clone());
                }
                Step::Spawn(what, x, y) => {
                    sim.dev_spawn(*what, *x, *y);
                }
                Step::SetFood(n) => sim.dev_set_food(*n),
                Step::SetSuper(n) => sim.dev_set_super(*n),
                Step::KillSpiders => {
                    for id in sim
                        .snapshot()
                        .iter()
                        .filter(|e| e.kind == 4)
                        .map(|e| e.id)
                        .collect::<Vec<_>>()
                    {
                        sim.dev_kill(id);
                    }
                }
                Step::Kill(id) => {
                    sim.dev_kill(*id);
                }
            }
            i += 1;
        }
    }
    println!("{}", sim.canonical_state());
}
