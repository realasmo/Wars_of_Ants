// Runs a replay file (client/public/replays/*.json) headless on the native
// build and prints the canonical state. The e2e suite runs the same replay
// in the browser (WASM build) and compares the two strings — any difference
// is a cross-platform determinism break.
//
// Usage: cargo run --example determinism_dump -- [path/to/replay.json]

use std::fs;

use woa_core::{Command, Config, DevSpawn, FollowMode, Sim, Team};

enum Step {
    Cmd(Command),
    Spawn(DevSpawn, f64, f64),
    SetFood(u32),
    SetSuper(u32),
    KillSpiders,
    Kill(u32),
    SetSoil(u32, u32, u32, u32),
    SetWater(u32),
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "client/public/replays/determinism.json".to_string());
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let replay: serde_json::Value = serde_json::from_str(&raw).expect("parse replay json");
    let seed = replay["seed"].as_u64().expect("seed") as u64;
    let ticks = replay["ticks"].as_u64().expect("ticks") as u64;
    // v2 replays are founding sessions (team recorded); v1 replays predate
    // the founding update and keep the legacy constructor
    let version = replay["version"].as_u64().unwrap_or(1);
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
                "land" => Step::Cmd(Command::Land {
                    ant: ant()?,
                    x: obj.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    y: obj.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0),
                }),
                "found" => Step::Cmd(Command::FoundNest {
                    ant: ant()?,
                    x: obj.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    y: obj.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0),
                }),
                "dump" | "drop" => Step::Cmd(Command::Drop {
                    ant: ant()?,
                    // surface dumps carry no meaningful target — default 0 like
                    // the client's replay applier (missing fields must not
                    // drop the whole command here)
                    tx: obj.get("tx").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    ty: obj.get("ty").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                }),
                "pick-egg" => Step::Cmd(Command::PickEgg {
                    ant: ant()?,
                    egg: obj.get("target")?.as_u64()? as u32,
                }),
                "follow-all" => Step::Cmd(Command::Follow {
                    leader: ant()?,
                    mode: FollowMode::All,
                }),
                "follow-one" => Step::Cmd(Command::Follow {
                    leader: ant()?,
                    mode: FollowMode::One,
                }),
                "follow-soldiers" => Step::Cmd(Command::Follow {
                    leader: ant()?,
                    mode: FollowMode::Soldiers,
                }),
                "follow-release" => Step::Cmd(Command::Follow {
                    leader: ant()?,
                    mode: FollowMode::Release,
                }),
                "dev-spawn" => Step::Spawn(
                    DevSpawn::parse(obj.get("kind")?.as_str()?)?,
                    obj.get("x")?.as_f64()?,
                    obj.get("y")?.as_f64()?,
                ),
                "dev-food" => Step::SetFood(obj.get("n").and_then(|v| v.as_u64()).unwrap_or(0) as u32),
                "dev-super" => Step::SetSuper(obj.get("n").and_then(|v| v.as_u64()).unwrap_or(0) as u32),
                "dev-water" => Step::SetWater(obj.get("n").and_then(|v| v.as_u64()).unwrap_or(0) as u32),
                "dev-kill-spiders" => Step::KillSpiders,
                "dev-kill" => Step::Kill(obj.get("target")?.as_u64()? as u32),
                "dev-soil" => Step::SetSoil(
                    obj.get("layer").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    obj.get("x").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    obj.get("y").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    obj.get("soil").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                ),
                // client-only acts the browser applier also skips: control
                // focus changes with no sim effect
                "select" | "cycle" => return None,
                // an unknown act must NEVER be silently skipped here: the
                // browser applier would still apply it and the canonical
                // comparison would diverge with no pointer to the cause
                other => panic!("determinism_dump: unknown replay act {other:?} at t={t}"),
            };
            let _ = || -> Option<u32> { Some(obj.get("ant")?.as_u64()? as u32) };
            let step = match step {
                Step::Cmd(c) => Step::Cmd(c),
                s => s,
            };
            Some((t, step))
        })
        .collect();

    let mut sim = if version >= 2 {
        let team = if replay["team"].as_u64() == Some(1) {
            Team::Blue
        } else {
            Team::Red
        };
        Sim::new_founding(seed, team)
    } else {
        Sim::new(seed, Config::default())
    };
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
                        .filter_map(|e| match e {
                            woa_core::EntitySnap::Spider(p) => Some(p.id),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                    {
                        sim.dev_kill(id);
                    }
                }
                Step::Kill(id) => {
                    sim.dev_kill(*id);
                }
                Step::SetSoil(l, x, y, soil) => {
                    sim.dev_set_soil(*l, *x, *y, *soil);
                }
                Step::SetWater(n) => {
                    sim.dev_set_water(*n);
                }
            }
            i += 1;
        }
    }
    println!("{}", sim.canonical_state());
}
