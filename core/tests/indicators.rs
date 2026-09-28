//! Activity indicators: the intent + blocked-reason projection on the ant
//! snapshot (wire v5). Every case mirrors a real gameplay situation — the
//! icons must never lie about what an ant is doing or why it is stuck.

use woa_core::{
    Blocked, Carry, Caste, Command, DevSpawn, EntitySnap, FoodKind, Intent, Layer, Sim, Team,
};

fn queen(s: &Sim) -> woa_core::AntSnap {
    s.snapshot()
        .into_iter()
        .find_map(|e| match e {
            EntitySnap::Ant(a) if a.caste == Caste::Queen => Some(a),
            _ => None,
        })
        .unwrap()
}

fn ants(s: &Sim) -> Vec<woa_core::AntSnap> {
    s.snapshot()
        .into_iter()
        .filter_map(|e| match e {
            EntitySnap::Ant(a) => Some(a),
            _ => None,
        })
        .collect()
}

fn founded(seed: u64) -> Sim {
    let mut s = Sim::new_founding(seed, Team::Red);
    let q = queen(&s);
    assert!(s.issue(Command::Land {
        ant: q.id,
        x: q.x,
        y: q.y
    }));
    assert!(s.issue(Command::FoundNest {
        ant: q.id,
        x: q.x,
        y: q.y
    }));
    s
}

/// Walk the queen to a chamber-edge tile with a soft neighbor (founding.rs's
/// dig_site pattern — the queen spawns mid-chamber, not at a wall).
fn queen_to_wall(s: &mut Sim) -> (u32, u32) {
    let q = queen(s);
    let (qx, qy) = (q.x.floor() as i32, q.y.floor() as i32);
    let mut site = None;
    'search: for r in 1..=4i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dy.abs() != r {
                    continue;
                }
                let (x, y) = (qx + dx, qy + dy);
                if x < 1 || y < 1 || x >= s.rules.width as i32 - 1 || y >= s.rules.height as i32 - 1
                {
                    continue;
                }
                let (x, y) = (x as u32, y as u32);
                if s.tile_at(Layer::Underground, x, y) != woa_core::EMPTY {
                    continue;
                }
                for (nx, ny) in [
                    (x as i32 + 1, y as i32),
                    (x as i32 - 1, y as i32),
                    (x as i32, y as i32 + 1),
                    (x as i32, y as i32 - 1),
                ] {
                    if nx < 0 || ny < 0 {
                        continue;
                    }
                    let k = s.tile_at(Layer::Underground, nx as u32, ny as u32);
                    if (1..=3).contains(&k) {
                        site = Some(((x, y), (nx as u32, ny as u32)));
                        break 'search;
                    }
                }
            }
        }
    }
    let ((sx, sy), target) = site.expect("a diggable wall within 4 tiles of the chamber");
    let q = queen(s);
    assert!(s.issue(Command::Move {
        ant: q.id,
        x: sx as f64 + 0.5,
        y: sy as f64 + 0.5
    }));
    for _ in 0..80 {
        s.tick();
    }
    target
}

#[test]
fn digging_queen_projects_the_dig_intent() {
    let mut s = founded(11);
    let q = queen(&s);
    let (tx, ty) = queen_to_wall(&mut s);
    assert!(s.issue(Command::Dig { ant: q.id, tx, ty }));
    s.tick();
    let q = queen(&s);
    assert_eq!(q.intent, Intent::Dig);
}

#[test]
fn dirt_in_the_mandibles_projects_haul_dirt() {
    let mut s = founded(11);
    let q = queen(&s);
    let (tx, ty) = queen_to_wall(&mut s);
    assert!(s.issue(Command::Dig { ant: q.id, tx, ty }));
    // queen_dig_time is 1s — two seconds is a full block
    for _ in 0..45 {
        s.tick();
    }
    let q = queen(&s);
    assert!(matches!(q.carry, Carry::Dirt { .. }));
    assert_eq!(q.intent, Intent::HaulDirt);
    assert_eq!(q.blocked, Blocked::None);
}

#[test]
fn honey_ants_project_produce_wherever_they_stand() {
    let mut s = founded(11);
    let q = queen(&s);
    let id = s.dev_spawn(DevSpawn::HoneyAnt, q.x, q.y);
    s.tick();
    let a = ants(&s).into_iter().find(|a| a.id == id).unwrap();
    assert_eq!(a.intent, Intent::Produce);
    assert_eq!(a.blocked, Blocked::None);
}

#[test]
fn medics_project_medic_from_standby_through_rescue() {
    let mut s = founded(11);
    let q = queen(&s);
    let _medic = s.dev_spawn(DevSpawn::MedicAnt, q.x, q.y);
    s.tick();
    let medic = ants(&s)
        .into_iter()
        .find(|a| a.caste == Caste::Medic)
        .unwrap();
    // no casualty yet: stand-by in the nest is still medic business
    assert_eq!(medic.intent, Intent::Medic);
}

fn kill_worldgen_spiders(s: &mut Sim) {
    let spiders: Vec<u32> = s
        .snapshot()
        .into_iter()
        .filter_map(|e| match e {
            EntitySnap::Spider(p) => Some(p.id),
            _ => None,
        })
        .collect();
    for id in spiders {
        s.dev_kill(id);
    }
}

/// Send an ant out through the entrance; once it surfaces, drop a spider on
/// it (founding.rs's ambush pattern — guaranteed contact, no wandering).
fn ambush_on_surface(s: &mut Sim, id: u32) -> u32 {
    assert!(s.issue(Command::UseEntrance { ant: id }));
    for _ in 0..20 * 40 {
        s.tick();
        let surfaced = ants(s)
            .into_iter()
            .find(|a| a.id == id)
            .map(|a| a.layer == Layer::Surface)
            .unwrap_or(false);
        if surfaced {
            let a = ants(s).into_iter().find(|a| a.id == id).unwrap();
            return s.dev_spawn(DevSpawn::Spider, a.x, a.y);
        }
    }
    panic!("ant never reached the surface");
}

fn hatch_founding_brood(s: &mut Sim) -> usize {
    // founding script out (60s window), paint orange under every egg, then
    // the full 180s incubation — returns the hatched worker count
    for _ in 0..1220 {
        s.tick();
    }
    for e in s.snapshot() {
        if let EntitySnap::Egg(e) = e {
            s.dev_set_soil(1, e.x.floor() as u32, e.y.floor() as u32, 1);
        }
    }
    for _ in 0..3620 {
        s.tick();
    }
    ants(s)
        .into_iter()
        .filter(|a| a.caste == Caste::Worker)
        .count()
}

#[test]
fn feeder_intent_and_the_empty_pantry_block() {
    let mut s = founded(42);
    kill_worldgen_spiders(&mut s);
    let workers = hatch_founding_brood(&mut s);
    assert!(
        workers >= 1,
        "the brood must have hatched for the feeder scenario"
    );
    // a feeder is designated the moment workers exist; the founding pantry
    // holds only the carb reserves, so the first craving (carbs) is covered
    let feeder = ants(&s)
        .into_iter()
        .find(|a| a.intent == Intent::Feeder)
        .expect("a feeder must be designated once workers exist");
    assert_eq!(feeder.id, s.colony.feeder_id.unwrap());
    // burn through the carb-only pantry until the craving the pantry cannot
    // cover comes up: hunger fuse lit, feeder stuck with empty hands
    let mut saw_blocked = false;
    for _ in 0..20 * 400 {
        s.tick();
        let feeder = ants(&s)
            .into_iter()
            .find(|a| a.id == feeder.id)
            .expect("feeder alive");
        if feeder.blocked == Blocked::EmptyPantry && !matches!(feeder.carry, Carry::Food(_)) {
            saw_blocked = true;
            // the queen herself is flagged starving the same moment
            let q = queen(&s);
            if q.hunger > 0.0 {
                assert_eq!(
                    q.blocked,
                    Blocked::QueenStarving,
                    "a burning fuse over an unsatisfiable craving flags the queen"
                );
            }
            break;
        }
    }
    assert!(
        saw_blocked,
        "the feeder must hit the empty-pantry block on a carb-only pantry \
         (craving rotates to protein/water)"
    );
}

#[test]
fn medic_waiting_on_pantry_water_projects_needs_water() {
    let mut s = founded(42);
    kill_worldgen_spiders(&mut s);
    s.rules.founding_eggs = 0; // isolate from the founding brood
    s.rules.bleed_time = 180.0; // room to catch the parked medic
    s.rules.eat_period = 1.0e9; // the queen never hungers mid-scenario
    let q = queen(&s);
    // NO pantry water: the healing can never start — that is the point
    let victim = s.dev_spawn(DevSpawn::Worker, q.x, q.y);
    let _medic = s.dev_spawn(DevSpawn::MedicAnt, q.x, q.y);
    let spider = ambush_on_surface(&mut s, victim);
    let mut medic_parked = false;
    for _ in 0..20 * 240 {
        s.tick();
        let snaps = ants(&s);
        let Some(m) = snaps.iter().find(|a| a.caste == Caste::Medic) else {
            continue;
        };
        let victim_down = snaps
            .iter()
            .find(|a| a.id == victim)
            .map(|a| a.downed.is_some())
            .unwrap_or(false);
        if !victim_down {
            continue;
        }
        if m.blocked == Blocked::NeedsWater {
            assert_eq!(m.intent, Intent::Medic);
            medic_parked = true;
            break;
        }
    }
    assert!(
        medic_parked,
        "a medic with a home-berthed patient and a waterless pantry must \
         report needs-water (spider {spider}, victim {victim})"
    );
}

#[test]
fn squad_followers_project_the_follow_intent() {
    let mut s = founded(42);
    kill_worldgen_spiders(&mut s);
    let workers = hatch_founding_brood(&mut s);
    assert!(workers >= 2, "follow needs two workers");
    // recruitment needs a same-layer pair within sight (8 tiles); the brood
    // spreads across the nest mouth, so search for a valid pair explicitly
    let snaps = ants(&s);
    let mut pair = None;
    for a in &snaps {
        if a.caste != Caste::Worker {
            continue;
        }
        for b in &snaps {
            if b.caste != Caste::Worker || b.id == a.id || b.layer != a.layer {
                continue;
            }
            let d = (a.x.floor() as i32 - b.x.floor() as i32)
                .abs()
                .max((a.y.floor() as i32 - b.y.floor() as i32).abs());
            if d <= 8 {
                pair = Some((a.id, b.id));
                break;
            }
        }
        if pair.is_some() {
            break;
        }
    }
    let Some((leader, follower)) = pair else {
        panic!("no same-layer worker pair in sight after hatch");
    };
    let before: Vec<_> = ants(&s).into_iter().map(|a| (a.id, a.intent)).collect();
    assert!(
        s.issue(Command::Follow {
            leader,
            mode: woa_core::FollowMode::All
        }),
        "recruiting nearby workers must succeed"
    );
    s.tick();
    let after = ants(&s);
    let f = after
        .iter()
        .find(|a| a.id == follower)
        .expect("follower alive");
    assert_eq!(f.following, Some(leader));
    assert_eq!(f.intent, Intent::Follow);
    // releasing returns them to their previous intents
    assert!(s.issue(Command::Follow {
        leader,
        mode: woa_core::FollowMode::Release
    }));
    s.tick();
    let released = ants(&s);
    for (id, intent) in before {
        if id == leader {
            continue;
        }
        let now = released.iter().find(|a| a.id == id).unwrap();
        assert_eq!(now.intent, intent, "release must restore ant {id}'s intent");
    }
}

#[test]
fn forage_intent_carries_the_resource_kind() {
    let mut s = founded(42);
    kill_worldgen_spiders(&mut s);
    assert!(
        hatch_founding_brood(&mut s) >= 1,
        "the brood must hatch before foraging"
    );
    // a moss source (water) right above the nest: within discovery sight of
    // the colony from the moment it exists, so idle workers fetch it
    let q = queen(&s);
    let src = s.dev_spawn(DevSpawn::Source(1), q.x + 3.0, q.y - 1.0);
    let mut saw_forage = false;
    for _ in 0..20 * 240 {
        s.tick();
        for a in ants(&s) {
            if a.caste == Caste::Worker
                && matches!(a.intent, Intent::Forage(k) if k == FoodKind::Water)
            {
                saw_forage = true;
                break;
            }
        }
        if saw_forage {
            break;
        }
    }
    assert!(
        saw_forage,
        "a worker fetching the moss source must project forage-water (source {src})"
    );
}

#[test]
fn downed_ants_carry_no_intent() {
    let mut s = founded(42);
    kill_worldgen_spiders(&mut s);
    s.rules.founding_eggs = 0;
    s.rules.bleed_time = 180.0;
    s.rules.eat_period = 1.0e9;
    let q = queen(&s);
    let victim = s.dev_spawn(DevSpawn::Worker, q.x, q.y);
    let _spider = ambush_on_surface(&mut s, victim);
    let mut downed_seen = false;
    for _ in 0..20 * 120 {
        s.tick();
        if let Some(v) = ants(&s).into_iter().find(|a| a.id == victim) {
            if v.downed.is_some() {
                assert_eq!(v.intent, Intent::None);
                assert_eq!(v.blocked, Blocked::None);
                downed_seen = true;
                break;
            }
        }
    }
    assert!(downed_seen, "the spider must down the exposed worker");
}
