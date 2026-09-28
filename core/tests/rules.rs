//! The admin-panel rules surface: whole-object JSON round-trip, atomic
//! validation, live commit with entity-stat re-derivation, custom-rules
//! world generation, and the field-registry metadata.

use serde_json::Value;
use woa_core::{GameRules, Sim, Team};

fn founded(seed: u64) -> Sim {
    let mut s = Sim::new_founding(seed, Team::Red);
    let q = s
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            woa_core::EntitySnap::Ant(a) if a.caste == woa_core::Caste::Queen => Some(a.id),
            _ => None,
        })
        .unwrap();
    use woa_core::Command::{FoundNest, Land};
    assert!(s.issue(Land {
        ant: q,
        x: 48.0,
        y: 48.0
    }));
    assert!(s.issue(FoundNest {
        ant: q,
        x: 48.0,
        y: 48.0
    }));
    s
}

fn tweak(json: &str, field: &str, value: Value) -> String {
    set_path(json, field, value)
}

/// Set a possibly-dotted path ("worker.speed") in a rules JSON document.
fn set_path(json: &str, path: &str, value: Value) -> String {
    let mut v: Value = serde_json::from_str(json).expect("parse rules json");
    let mut target = &mut v;
    let parts: Vec<&str> = path.split('.').collect();
    for part in &parts[..parts.len() - 1] {
        target = target
            .get_mut(*part)
            .unwrap_or_else(|| panic!("rules json has no {part}"));
    }
    target[parts[parts.len() - 1]] = value;
    serde_json::to_string(&v).unwrap()
}

#[test]
fn rules_json_round_trips_with_a_stable_digest() {
    let mut s = founded(7);
    let json = s.rules_json();
    let digest_before = s.rules_digest_hex();
    let new_digest = s
        .set_rules(&json)
        .expect("round-tripping the current rules must validate");
    assert_eq!(new_digest, digest_before);
    // the canonical digest folds the rules — an unchanged commit keeps it
    let canon_before = s.canonical_state();
    s.set_rules(&json).unwrap();
    assert_eq!(s.canonical_state(), canon_before);
}

#[test]
fn invalid_commit_is_atomic_and_lists_every_error() {
    let mut s = founded(7);
    let json = s.rules_json();
    let digest = s.rules_digest_hex();
    let canon = s.canonical_state();
    // two cross-field violations at once, plus one unknown field style of
    // failure in a separate commit below
    let bad = tweak(
        &tweak(&json, "dig_time", serde_json::json!(0.0)),
        "near_ring_min",
        serde_json::json!(99.0),
    );
    let errs = s.set_rules(&bad).expect_err("must refuse invalid rules");
    let joined = errs.join(" | ");
    assert!(joined.contains("dig_time"), "errors: {joined}");
    assert!(joined.contains("near_ring_min"), "errors: {joined}");
    assert_eq!(
        s.rules_digest_hex(),
        digest,
        "refused commit must not apply"
    );
    assert_eq!(s.canonical_state(), canon);
    // a field the struct does not know is a parse error, not a silent skip
    let unknown = tweak(&json, "widthh", serde_json::json!(64));
    let errs = s.set_rules(&unknown).expect_err("unknown fields must fail");
    assert!(errs.join(" | ").contains("widthh"));
    // type violations refuse too (u32 field given a fraction)
    let frac = tweak(&json, "max_ants", serde_json::json!(24.5));
    assert!(s.set_rules(&frac).is_err());
}

#[test]
fn live_commit_resyncs_entity_stats_and_folds_the_digest() {
    let mut s = Sim::new(42, GameRules::default());
    // legacy start: workers exist with rules-copied speed
    let canon_before = s.canonical_state();
    let json = set_path(&s.rules_json(), "worker.speed", serde_json::json!(6.0));
    let json = set_path(&json, "max_ants", serde_json::json!(30));
    s.set_rules(&json).expect("valid commit");
    assert_ne!(s.rules_digest_hex(), digest_of(&canon_before));
    let canon_after = s.canonical_state();
    assert!(
        canon_after.contains("sp6.0000"),
        "worker speed must be re-derived live"
    );
    assert_ne!(canon_before, canon_after);
    // hp fraction is preserved across a max_hp change: healthy stays healthy
    let json = set_path(&json, "worker.hp", serde_json::json!(250.0));
    s.set_rules(&json).unwrap();
    assert!(
        s.canonical_state().contains("hp1.0000"),
        "full-health workers stay at fraction 1 after a max_hp bump"
    );
}

fn digest_of(canon: &str) -> String {
    let i = canon
        .find("rules=")
        .expect("canon carries the rules digest");
    canon[i + 6..i + 22].to_string()
}

#[test]
fn custom_rules_shape_the_generated_world() {
    let base = GameRules::default();
    let mut rules = base.clone();
    rules.width = 64;
    rules.height = 64;
    rules.spiders = 0;
    assert!(rules.validate().is_ok());
    let s = Sim::new_founding_with(42, Team::Red, rules);
    assert_eq!((s.rules.width, s.rules.height), (64, 64));
    assert!(s
        .snapshot()
        .iter()
        .all(|e| !matches!(e, woa_core::EntitySnap::Spider(_))));
}

#[test]
fn defaults_json_matches_a_default_sim() {
    let s = Sim::new_founding(1, Team::Blue);
    let defaults: Value = serde_json::from_str(&s.rules_json()).unwrap();
    let shipped: Value =
        serde_json::from_str(&serde_json::to_string(&GameRules::default()).unwrap()).unwrap();
    assert_eq!(defaults, shipped);
}

#[test]
fn meta_registry_covers_every_field_with_a_scope() {
    let meta: Value = serde_json::from_str(&GameRules::meta_json()).unwrap();
    let fields = meta["fields"].as_object().expect("fields map");
    let rules: Value =
        serde_json::from_str(&serde_json::to_string(&GameRules::default()).unwrap()).unwrap();
    for key in rules.as_object().unwrap().keys() {
        let f = fields
            .get(key)
            .unwrap_or_else(|| panic!("meta registry is missing field {key}"));
        let scope = f["scope"].as_str().unwrap();
        assert!(
            scope == "live" || scope == "new_game",
            "{key} has an unknown scope {scope}"
        );
    }
    // the two nested shapes are described for the form generator
    assert_eq!(
        meta["unit_fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["hp", "dmg", "atk_cd", "speed", "range"]
    );
}

#[test]
fn rules_change_is_deterministic_under_replay() {
    // the same commit bytes applied at the same tick produce identical
    // canonical state — the replay/determinism contract for admin commits
    let mut a = founded(3);
    let mut b = founded(3);
    let json = tweak(&a.rules_json(), "dig_time", serde_json::json!(0.5));
    for _ in 0..100 {
        a.tick();
        b.tick();
    }
    assert_eq!(a.set_rules(&json).unwrap(), b.set_rules(&json).unwrap());
    for _ in 0..100 {
        a.tick();
        b.tick();
    }
    assert_eq!(a.canonical_state(), b.canonical_state());
}
