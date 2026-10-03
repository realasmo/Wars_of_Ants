//! The shipped data files (`data/*.jsonc`) are the game's balance source
//! of truth. These tests keep the contract between the files and the
//! `GameRules` schema honest: every field covered exactly once, the
//! merge/strip machinery correct, and the admin/replay parse path
//! accepting the same annotated documents the files use.

use std::collections::{BTreeMap, BTreeSet};

use woa_core::{merge_rules, strip_jsonc, GameRules, Sim, Team, DATA_FILES};

#[test]
fn shipped_files_load_validate_and_round_trip() {
    let rules = woa_core::load_shipped_rules().expect("shipped files must load and validate");
    // the rules the sim actually runs under are exactly the shipped files
    let sim = Sim::new_founding(7, Team::Blue);
    assert_eq!(sim.rules_json(), serde_json::to_string(&rules).unwrap());
    // and a whole-document round trip through the admin/replay parse path
    // is digest-neutral
    let again = Sim::parse_rules(&serde_json::to_string(&rules).unwrap()).unwrap();
    assert_eq!(rules.digest(), again.digest());
}

#[test]
fn every_rules_field_lives_in_exactly_one_file() {
    let mut owner: BTreeMap<String, &str> = BTreeMap::new();
    let mut dups = Vec::new();
    for (name, text) in DATA_FILES {
        let value: serde_json::Value =
            serde_json::from_str(&strip_jsonc(text)).expect("data file parses after comment strip");
        for key in value.as_object().expect("data file is an object").keys() {
            if owner.insert(key.clone(), name).is_some() {
                dups.push(format!("{key} (in {name} and an earlier file)"));
            }
        }
    }
    assert!(
        dups.is_empty(),
        "each rules key in exactly one data file — dups: {dups:?}"
    );
    let serialized = serde_json::to_value(GameRules::default()).unwrap();
    let schema: BTreeSet<&String> = serialized.as_object().unwrap().keys().collect();
    let files: BTreeSet<&String> = owner.keys().collect();
    let missing: Vec<_> = schema.difference(&files).collect();
    let extra: Vec<_> = files.difference(&schema).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "files must cover the GameRules schema exactly — missing from files: {missing:?}, unknown in files: {extra:?}"
    );
}

#[test]
fn merge_rules_is_loud_about_every_failure_mode() {
    // duplicate key across files
    let errs = merge_rules(&[
        ("a.jsonc", r#"{"width": 96}"#),
        ("b.jsonc", r#"{"width": 48}"#),
    ])
    .unwrap_err();
    assert!(errs.len() == 1, "{errs:?}");
    assert!(
        errs[0].contains("b.jsonc") && errs[0].contains("width"),
        "{errs:?}"
    );
    // unknown key survives the merge and dies in the typed parse
    let errs = merge_rules(&[("a.jsonc", r#"{"nope": 1}"#)]).unwrap_err();
    assert!(errs[0].contains("unknown field `nope`"), "{errs:?}");
    // top level must be an object
    let errs = merge_rules(&[("a.jsonc", "[1, 2]")]).unwrap_err();
    assert!(
        errs[0].contains("top level must be a JSON object"),
        "{errs:?}"
    );
    // a broken file is reported by name
    let errs = merge_rules(&[("a.jsonc", "{")]).unwrap_err();
    assert!(errs[0].contains("a.jsonc"), "{errs:?}");
}

#[test]
fn parse_rules_accepts_annotated_documents() {
    // the drawer's JSON box and dev-rules replays go through the same
    // stripper as the shipped files: comments in, identical digest out
    let json = serde_json::to_string(&GameRules::default()).unwrap();
    let annotated = json.replace(
        "\"max_ants\":24,",
        "\"max_ants\":24 /* tuned */, // leave a note\n",
    );
    assert_ne!(json, annotated, "the surgery above must have landed");
    let parsed = Sim::parse_rules(&annotated).expect("commented document parses");
    assert_eq!(parsed.digest(), GameRules::default().digest());
}

#[test]
fn dirt_capacity_is_per_caste() {
    let rules = GameRules::default();
    // the two digging castes carry a real limit (exact value is tuned in
    // data/ants.jsonc); castes refused at the dig order carry the honest 0
    assert!(rules.worker.dirt_capacity >= 1);
    assert!(rules.queen.dirt_capacity >= 1);
    assert_eq!(rules.soldier.dirt_capacity, 0);
    assert_eq!(rules.honey.dirt_capacity, 0);
    assert_eq!(rules.medic.dirt_capacity, 0);
    // a digger with no carry limit would never trigger the haul-out
    let mut broken = rules.clone();
    broken.worker.dirt_capacity = 0;
    assert!(broken.validate().is_err());
    let mut broken = rules;
    broken.queen.dirt_capacity = 0;
    assert!(broken.validate().is_err());
}
