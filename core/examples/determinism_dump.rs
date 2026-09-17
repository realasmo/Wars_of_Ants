// Runs a replay file (client/public/replays/*.json) headless on the native
// build and prints the canonical state. The e2e suite runs the same replay
// in the browser (WASM build) and compares the two strings — any difference
// is a cross-platform determinism break.
//
// Usage: cargo run --example determinism_dump -- [path/to/replay.json]

use std::fs;

use woa_core::{Command, Config, Sim};

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "client/public/replays/determinism.json".to_string());
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let replay: serde_json::Value = serde_json::from_str(&raw).expect("parse replay json");
    let seed = replay["seed"].as_u64().expect("seed") as u64;
    let ticks = replay["ticks"].as_u64().expect("ticks") as u64;
    let cmds: Vec<(u64, Command)> = replay["cmds"]
        .as_array()
        .expect("cmds")
        .iter()
        .filter_map(|v| {
            let obj = v.as_object()?;
            let t = obj.get("t")?.as_u64()?;
            let ant = obj.get("ant")?.as_u64()? as u32;
            let cmd = match obj.get("act")?.as_str()? {
                "move" => Command::Move {
                    ant,
                    x: obj.get("x")?.as_f64()?,
                    y: obj.get("y")?.as_f64()?,
                },
                "dig" => Command::Dig {
                    ant,
                    tx: obj.get("tx")?.as_u64()? as u32,
                    ty: obj.get("ty")?.as_u64()? as u32,
                },
                "attack" => Command::Attack {
                    ant,
                    target: obj.get("target")?.as_u64()? as u32,
                },
                "entrance" => Command::UseEntrance { ant },
                _ => return None,
            };
            Some((t, cmd))
        })
        .collect();

    let mut sim = Sim::new(seed, Config::default());
    let mut i = 0usize;
    for _ in 0..ticks {
        sim.tick();
        while i < cmds.len() && cmds[i].0 <= sim.tick {
            sim.issue(cmds[i].1.clone());
            i += 1;
        }
    }
    println!("{}", sim.canonical_state());
}
