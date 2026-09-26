//! Regression tests for the 2026-09 code-quality wave fixes (see
//! docs/AUDIT.md): friendly fire, the queen retry freeze, and attack
//! engagement semantics.

use woa_core::{AntSnap, Caste, Command, Config, DevSpawn, EntitySnap, Phase, Sim, Team, EMPTY};

fn queen(s: &Sim) -> AntSnap {
    s.snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Ant(a) if a.caste == Caste::Queen => Some(a),
            _ => None,
        })
        .unwrap()
}

fn founded(seed: u64) -> Sim {
    let mut s = Sim::new_founding(seed, Team::Red);
    let q = queen(&s);
    assert!(s.issue(Command::Land { ant: q.id, x: q.x, y: q.y }));
    assert!(s.issue(Command::FoundNest {
        ant: q.id,
        x: q.x,
        y: q.y
    }));
    s
}

#[test]
fn attack_on_own_colony_is_refused() {
    // no friendly fire: a worker may not be ordered to attack colony ants
    let mut s = Sim::new(7, Config::default());
    let ids: Vec<(u32, Caste)> = s
        .snapshot()
        .into_iter()
        .filter_map(|e| match e {
            EntitySnap::Ant(a) => Some((a.id, a.caste)),
            _ => None,
        })
        .collect();
    let attacker = ids
        .iter()
        .find(|(_, c)| *c == Caste::Worker)
        .map(|(id, _)| *id)
        .unwrap();
    for (id, caste) in &ids {
        if *id == attacker {
            continue;
        }
        assert!(
            !s.issue(Command::Attack {
                ant: attacker,
                target: *id
            }),
            "attacking our own {caste:?} must be refused"
        );
    }
    // predators remain valid targets
    let spider = s
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Spider(p) => Some(p.id),
            _ => None,
        })
        .unwrap();
    assert!(s.issue(Command::Attack {
        ant: attacker,
        target: spider
    }));
}

#[test]
fn queen_recovers_from_retry_backoff() {
    // regression: the queen's worker_ai branch skipped retry>0 ants without
    // decrementing (only the worker path decremented), so any set_retry
    // froze her walk-to-act intents forever. Prime the backoff directly,
    // then verify a distant dig still happens.
    let mut s = founded(42);
    // a far, reachable, fully-soft frontier block near the chamber
    let q0 = queen(&s);
    let (qx, qy) = (q0.x.floor() as i32, q0.y.floor() as i32);
    let mut target = None;
    let mut best_d = 0i32;
    'scan: for by in (40..58u32).step_by(2) {
        for bx in (40..58u32).step_by(2) {
            let soft = (0..2u32).all(|dy| {
                (0..2u32).all(|dx| {
                    let k = s.tile_at(woa_core::Layer::Underground, bx + dx, by + dy);
                    (1..=3).contains(&k)
                })
            });
            if !soft {
                continue;
            }
            let ring_empty = [
                (bx.wrapping_sub(1), by), (bx.wrapping_sub(1), by + 1),
                (bx + 2, by), (bx + 2, by + 1),
                (bx, by.wrapping_sub(1)), (bx + 1, by.wrapping_sub(1)),
                (bx, by + 2), (bx + 1, by + 2),
            ]
            .iter()
            .any(|&(x, y)| s.tile_at(woa_core::Layer::Underground, x, y) == EMPTY);
            if !ring_empty {
                continue;
            }
            let d = (bx as i32 - qx).abs() + (by as i32 - qy).abs();
            if d > best_d {
                best_d = d;
                target = Some((bx, by));
                if d >= 5 {
                    break 'scan;
                }
            }
        }
    }
    let (bx, by) = target.expect("a distant soft block");
    // prime the queen's retry backoff directly (the organic trigger needs
    // concurrent diggers that Wave B will add): in a founded sim the queen
    // is the only entity carrying WorkerAi
    for (_e, ai) in s.ecs.query::<&mut woa_core::WorkerAi>().iter() {
        ai.retry = 5;
    }
    assert!(s.issue(Command::Dig {
        ant: q0.id,
        tx: bx,
        ty: by
    }));
    for _ in 0..800 {
        s.tick();
    }
    assert_eq!(
        s.tile_at(woa_core::Layer::Underground, bx, by),
        EMPTY,
        "queen must dig the distant block after the backoff expires"
    );
}

#[test]
fn attack_intent_survives_a_routed_chase() {
    // attack_after now persists through Fighting so a routed (underground)
    // chase re-engages on arrival; on the surface the order still lands.
    let mut s = founded(42);
    // spawn a spider next to the nest entrance on the surface
    let (ex, ey) = s.world.entrance.unwrap();
    s.dev_spawn(DevSpawn::Spider, ex as f64 + 1.5, ey as f64 + 3.5);
    let spider = s
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Spider(p) => Some(p.id),
            _ => None,
        })
        .unwrap();
    // the queen is underground in Phase::Founding: the attack order routes
    // her up through the entrance (surface engage)
    let q = queen(&s);
    assert!(s.issue(Command::Attack {
        ant: q.id,
        target: spider
    }));
    for _ in 0..1200 {
        s.tick();
        // she must reach the surface and actually fight: the spider takes
        // damage (queen dmg is 0 — engagement is proven by the spider
        // targeting her back / her Fighting activity)
        if let Some(a) = s
            .snapshot()
            .into_iter()
            .find_map(|e| match e {
                EntitySnap::Ant(a) if a.id == q.id => Some(a),
                _ => None,
            })
        {
            if a.activity == woa_core::Activity::Fighting {
                assert_eq!(a.layer, woa_core::Layer::Surface);
                return;
            }
        }
    }
    panic!("queen never engaged the spider from underground");
}
