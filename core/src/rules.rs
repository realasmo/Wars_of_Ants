//! Game rules: every tunable gameplay number lives here as DATA on the sim,
//! not as compiled-in constants — one typed, validated, versioned struct
//! (`GameRules`) carried by `Sim`. This file graduated from the old const
//! balance table when tuning became a real workflow (the F4 brood costs were
//! the first rows that needed it; the admin tuning panel reads/writes this
//! struct next). Data only — no behavior — so balancing never touches sim
//! code, and two sims can be compared by their rules digest.
//!
//! Defaults are the shipped game balance. `validate()` guards the invariants
//! every consumer assumes (min ≤ max ranges, positive times, sane caps);
//! `digest()` folds the whole struct into the canonical state so a replay
//! can never silently run under different rules than it was recorded with.

use crate::components::{Caste, FoodKind};

/// Combat/movement stats per unit type. Times are in seconds, distances in tiles.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitStats {
    pub hp: f64,
    pub dmg: f64,
    pub atk_cd: f64,
    pub speed: f64,
    pub range: f64,
}

/// One finite map source type (see docs/WORLD-DESIGN.md).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSpec {
    /// Client visual type (1..N, mirrored in the client's source art switch).
    pub src: u8,
    pub kind: FoodKind,
    /// (min, max) units per source.
    pub amount: (u32, u32),
    /// Seconds to harvest one unit.
    pub harvest: f64,
    /// How many on the map.
    pub count: u32,
}

/// What the queen spends to lay one brood egg of a caste (F4). Costs are
/// paid from the physical pantry (stored piles) — the first data-driven
/// rows of the rules layer.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BroodCost {
    pub protein: u32,
    pub carbs: u32,
    pub water: u32,
    pub honeydew: u32,
    /// Seconds of incubation for this caste's eggs.
    pub egg_time: f64,
    /// Soldier: the order also consumes one existing worker (metamorphosis).
    pub consumes_worker: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BroodRules {
    pub worker: BroodCost,
    pub soldier: BroodCost,
    pub honey: BroodCost,
    pub medic: BroodCost,
}

impl BroodRules {
    pub fn for_caste(&self, caste: Caste) -> Option<&BroodCost> {
        match caste {
            Caste::Worker => Some(&self.worker),
            Caste::Soldier => Some(&self.soldier),
            Caste::Honey => Some(&self.honey),
            Caste::Medic => Some(&self.medic),
            Caste::Queen => None,
        }
    }
}

/// The complete ruleset a sim runs under. Field names keep the historical
/// constant names (snake_cased) so graduating a value stays a mechanical,
/// reviewable change. Serde round-trips whole-object only (unknown fields
/// are rejected): a rules commit is atomic, all fields or nothing.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameRules {
    // --- world & start (absorbs the old `Config`) ---
    pub width: u32,
    pub height: u32,
    pub start_workers: u32,
    pub food_clusters: u32,
    pub max_ants: u32,
    pub spiders: u32,

    // --- worldgen scatter (every placement number, no stray literals) ---
    /// Chance an interior underground cell is rock.
    pub rock_chance: f64,
    /// Chance per 2×2 underground block of being orange (nursery) or silver
    /// (pantry) soil, hidden until dug.
    pub orange_soil_chance: f64,
    pub silver_soil_chance: f64,
    /// Surface dust patches of each color (indicator + founding bonus source).
    pub patches_per_color: u32,
    pub patch_w: u32,
    pub patch_h: u32,
    pub patch_wobble: u32,
    /// Minimum center distance between dust patches.
    pub patch_min_dist: u32,
    /// Founding inside a patch grants this many hidden soil blocks nearby.
    pub patch_grant_min: u32,
    pub patch_grant_max: u32,
    /// Carb sources placed near the founding center so the first workers can
    /// survive; the rest scatter wide.
    pub sources_near_nest: u32,
    /// Minimum chebyshev gap between placed sources.
    pub source_min_gap: u32,
    /// Ring radius (tiles) around the founding center for near-nest carb
    /// sources.
    pub near_ring_min: f64,
    pub near_ring_max: f64,
    /// Spider spawn distance from the nest center.
    pub spider_dist_min: f64,
    pub spider_dist_max: f64,
    /// Surface collectibles per founding map: wet wood (food-storage block)
    /// and dry wool (egg-friendly block).
    pub wood_count: u32,
    pub wool_count: u32,

    // --- resource economy ---
    pub sources: Vec<SourceSpec>,
    /// Source sighting distance (chebyshev tiles); also the squad
    /// recruitment range so "visible" is one consistent rule.
    pub sight_range: u32,
    /// Protein dropped by a killed spider.
    pub protein_per_spider: u32,
    /// Legacy worlds: "super" units dropped by a killed spider.
    pub super_per_spider: u32,

    // --- unit stats ---
    pub queen: UnitStats,
    pub worker: UnitStats,
    pub soldier: UnitStats,
    pub spider: UnitStats,
    pub honey: UnitStats,
    pub medic: UnitStats,
    /// Melee reach shared by all ant castes (kept uniform for now).
    pub ant_range: f64,

    // --- brood production (F4): queen X-menu egg orders ---
    pub brood: BroodRules,
    /// Seconds a Honey ant needs to secrete one honeydew unit.
    pub honey_period: f64,
    /// Seconds a downed ant bleeds out in (F4 medics).
    pub bleed_time: f64,
    /// Seconds a medic's healing takes once the patient is home.
    pub heal_time: f64,
    /// Stored water units consumed by one healing.
    pub water_per_heal: u32,

    // --- founding (game start) ---
    /// Flight speed of the founding queen over the surface (tiles/s).
    pub queen_fly_speed: f64,
    /// Seconds the founding queen has to excavate before laying the first brood.
    pub founding_time: f64,
    /// Eggs laid when the founding timer ends (up to free-tile availability).
    pub founding_eggs: u32,
    /// Seconds until founding eggs hatch into the first workers.
    pub founding_egg_hatch: f64,
    /// Seconds per dirt cell dug by the founding queen.
    pub queen_dig_time: f64,
    /// Dirt blocks an ant may carry before having to dump (dig two, haul once).
    pub dirt_capacity: u32,
    /// Starting carbs: founding reserves spawn as a physical stored pile.
    pub start_food: u32,

    // --- queen economy ---
    /// Time until the founding queen is hungry again after a meal. Hungry
    /// past this, dead at eat_period + starve_time without food (physical
    /// feeding, worker-priorities wave).
    pub eat_period: f64,
    pub starve_time: f64,
    /// The queen's craving cycle (rotating; explicit index in Colony — Carbs
    /// repeats, so position-by-search would alias).
    pub craving_cycle: Vec<FoodKind>,
    /// Seconds between egg orders/layings.
    pub lay_cooldown: f64,
    /// Seconds of incubation for a laid egg (all castes).
    pub egg_time: f64,

    // --- soil / food logistics ---
    /// Seconds per soft block dug by a worker.
    pub dig_time: f64,
    /// How far from the queen AI nest-expansion searches for the next soft
    /// block to dig. Radius 3 deadlocks: once that zone is dug out, the
    /// pantry stays full, deliveries halt, and the colony starves beside
    /// surface food.
    pub dig_expand_radius: u32,
    /// Max food units (all kinds combined) per cell.
    pub food_cell_cap: u32,
    /// Seconds before dropped food spoils on a non-silver cell.
    pub spoil_time: f64,
    /// Loose pile size for dev/test spawns and legacy clusters.
    pub pile_amount: u32,
    /// Taskless workers: random hops within this radius of where they idled.
    pub loiter_radius: u32,
    /// Loiter hops before heading home.
    pub loiter_hops: u32,
    /// Rest in the nest (ticks) between loiter cycles.
    pub home_rest_ticks: u32,
    pub spider_aggro: f64,
    pub spider_wander: f64,

    // --- legacy founded economy (auto-laying; founding uses brood orders) ---
    pub egg_cost: u32,
    pub soldier_cost_green: u32,
    pub soldier_cost_super: u32,
}

impl Default for GameRules {
    fn default() -> Self {
        GameRules {
            width: 96,
            height: 96,
            start_workers: 3,
            food_clusters: 6,
            max_ants: 24,
            spiders: 2,

            rock_chance: 0.08,
            orange_soil_chance: 0.035,
            silver_soil_chance: 0.035,
            patches_per_color: 2,
            patch_w: 10,
            patch_h: 12,
            patch_wobble: 2,
            patch_min_dist: 20,
            patch_grant_min: 3,
            patch_grant_max: 6,
            sources_near_nest: 2,
            source_min_gap: 8,
            near_ring_min: 7.0,
            near_ring_max: 14.0,
            spider_dist_min: 25.0,
            spider_dist_max: 45.0,
            wood_count: 4,
            wool_count: 4,

            sources: vec![
                SourceSpec {
                    src: 1,
                    kind: FoodKind::Water,
                    amount: (15, 20),
                    harvest: 10.0,
                    count: 10,
                },
                SourceSpec {
                    src: 2,
                    kind: FoodKind::Water,
                    amount: (21, 26),
                    harvest: 20.0,
                    count: 7,
                },
                SourceSpec {
                    src: 3,
                    kind: FoodKind::Carbs,
                    amount: (100, 110),
                    harvest: 6.0,
                    count: 6,
                },
                SourceSpec {
                    src: 4,
                    kind: FoodKind::Carbs,
                    amount: (40, 60),
                    harvest: 4.0,
                    count: 8,
                },
                SourceSpec {
                    src: 5,
                    kind: FoodKind::Protein,
                    amount: (8, 12),
                    harvest: 15.0,
                    count: 6,
                },
                SourceSpec {
                    src: 6,
                    kind: FoodKind::Protein,
                    amount: (25, 35),
                    harvest: 13.0,
                    count: 5,
                },
                // F4: honeydew source until aphid farming (Wave D)
                SourceSpec {
                    src: 7,
                    kind: FoodKind::Honeydew,
                    amount: (15, 22),
                    harvest: 10.0,
                    count: 4,
                },
            ],
            sight_range: 8,
            protein_per_spider: 8,
            super_per_spider: 8,

            queen: UnitStats {
                hp: 150.0,
                dmg: 0.0,
                atk_cd: 1.0,
                speed: 2.5,
                range: 0.9,
            },
            worker: UnitStats {
                hp: 100.0,
                dmg: 8.0,
                atk_cd: 1.0,
                speed: 3.0,
                range: 0.9,
            },
            soldier: UnitStats {
                hp: 130.0,
                dmg: 22.0,
                atk_cd: 1.0,
                speed: 2.6,
                range: 0.9,
            },
            spider: UnitStats {
                hp: 130.0,
                dmg: 15.0,
                atk_cd: 1.2,
                speed: 2.2,
                range: 0.8,
            },
            // specialists (F4): weak fighters — their value is the special role
            honey: UnitStats {
                hp: 90.0,
                dmg: 4.0,
                atk_cd: 1.0,
                speed: 2.8,
                range: 0.9,
            },
            medic: UnitStats {
                hp: 95.0,
                dmg: 5.0,
                atk_cd: 1.0,
                speed: 3.2,
                range: 0.9,
            },
            ant_range: 0.9,

            // F4 settled costs (docs/WORLD-DESIGN.md): worker 2p+1w, soldier
            // 6p+3w + consumes a worker, honey 1p+8h, medic 4p+3c
            brood: BroodRules {
                worker: BroodCost {
                    protein: 2,
                    carbs: 0,
                    water: 1,
                    honeydew: 0,
                    egg_time: 45.0,
                    consumes_worker: false,
                },
                soldier: BroodCost {
                    protein: 6,
                    carbs: 0,
                    water: 3,
                    honeydew: 0,
                    egg_time: 45.0,
                    consumes_worker: true,
                },
                honey: BroodCost {
                    protein: 1,
                    carbs: 0,
                    water: 0,
                    honeydew: 8,
                    egg_time: 45.0,
                    consumes_worker: false,
                },
                medic: BroodCost {
                    protein: 4,
                    carbs: 3,
                    water: 0,
                    honeydew: 0,
                    egg_time: 45.0,
                    consumes_worker: false,
                },
            },
            honey_period: 240.0,
            bleed_time: 60.0,
            heal_time: 10.0,
            water_per_heal: 2,

            queen_fly_speed: 5.0,
            founding_time: 60.0,
            founding_eggs: 4,
            founding_egg_hatch: 180.0,
            queen_dig_time: 1.0,
            dirt_capacity: 2,
            start_food: 5,

            eat_period: 25.0,
            starve_time: 90.0,
            craving_cycle: vec![
                FoodKind::Carbs,
                FoodKind::Protein,
                FoodKind::Carbs,
                FoodKind::Water,
            ],
            lay_cooldown: 3.0,
            egg_time: 45.0,

            dig_time: 1.2,
            dig_expand_radius: 8,
            food_cell_cap: 6,
            spoil_time: 300.0,
            pile_amount: 45,
            loiter_radius: 4,
            loiter_hops: 3,
            home_rest_ticks: 300,
            spider_aggro: 5.0,
            spider_wander: 8.0,

            egg_cost: 5,
            soldier_cost_green: 3,
            soldier_cost_super: 2,
        }
    }
}

impl GameRules {
    /// Combat/movement stats for a caste.
    pub fn stats_for(&self, caste: Caste) -> &UnitStats {
        match caste {
            Caste::Queen => &self.queen,
            Caste::Worker => &self.worker,
            Caste::Soldier => &self.soldier,
            Caste::Honey => &self.honey,
            Caste::Medic => &self.medic,
        }
    }

    /// The queen's current craving at a cycle index (clamped, like Colony's
    /// stored index).
    pub fn craving_at(&self, i: usize) -> FoodKind {
        self.craving_cycle[i.min(self.craving_cycle.len() - 1)]
    }

    /// Every invariant the sim assumes. Returns all violations at once so a
    /// rules editor can show them together (the admin panel's atomic commit
    /// refuses on any).
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errs = Vec::new();
        fn positive(errs: &mut Vec<String>, name: &str, v: f64) {
            if !(v.is_finite() && v > 0.0) {
                errs.push(format!("{name} must be > 0 (got {v})"));
            }
        }
        if self.width < 24 || self.height < 24 {
            errs.push(format!(
                "map too small: {}×{} (minimum 24×24 — founding chamber + border must fit)",
                self.width, self.height
            ));
        }
        if self.max_ants == 0 {
            errs.push("max_ants must be ≥ 1".into());
        }
        if !(0.0..=1.0).contains(&self.rock_chance) {
            errs.push(format!(
                "rock_chance must be 0..=1 (got {})",
                self.rock_chance
            ));
        }
        if self.near_ring_min > self.near_ring_max {
            errs.push("near_ring_min must be ≤ near_ring_max".into());
        }
        if self.spider_dist_min > self.spider_dist_max {
            errs.push("spider_dist_min must be ≤ spider_dist_max".into());
        }
        if self.patch_grant_min > self.patch_grant_max {
            errs.push("patch_grant_min must be ≤ patch_grant_max".into());
        }
        if self.sources.is_empty() {
            errs.push("sources must not be empty".into());
        }
        for s in &self.sources {
            if s.src == 0 {
                errs.push(format!(
                    "source {:?}: src 0 is reserved (0 = not a source)",
                    s
                ));
            }
            if s.amount.0 == 0 || s.amount.0 > s.amount.1 {
                errs.push(format!(
                    "source {}: amount min {} must be ≥ 1 and ≤ max {}",
                    s.src, s.amount.0, s.amount.1
                ));
            }
            if !s.harvest.is_finite() || s.harvest <= 0.0 {
                errs.push(format!(
                    "source {}: harvest must be > 0 (got {})",
                    s.src, s.harvest
                ));
            }
        }
        positive(&mut errs, "queen_fly_speed", self.queen_fly_speed);
        positive(&mut errs, "founding_time", self.founding_time);
        positive(&mut errs, "founding_egg_hatch", self.founding_egg_hatch);
        positive(&mut errs, "queen_dig_time", self.queen_dig_time);
        positive(&mut errs, "eat_period", self.eat_period);
        positive(&mut errs, "starve_time", self.starve_time);
        positive(&mut errs, "lay_cooldown", self.lay_cooldown);
        positive(&mut errs, "egg_time", self.egg_time);
        positive(&mut errs, "dig_time", self.dig_time);
        positive(&mut errs, "spoil_time", self.spoil_time);
        positive(&mut errs, "honey_period", self.honey_period);
        positive(&mut errs, "bleed_time", self.bleed_time);
        positive(&mut errs, "heal_time", self.heal_time);
        for (name, cost) in [
            ("worker", &self.brood.worker),
            ("soldier", &self.brood.soldier),
            ("honey", &self.brood.honey),
            ("medic", &self.brood.medic),
        ] {
            if !(cost.egg_time.is_finite() && cost.egg_time > 0.0) {
                errs.push(format!("{name} brood egg_time must be > 0"));
            }
            if cost.protein == 0 && cost.carbs == 0 && cost.water == 0 && cost.honeydew == 0 {
                errs.push(format!(
                    "{name} brood must cost something (free brood breaks the economy)"
                ));
            }
        }
        if self.craving_cycle.is_empty() {
            errs.push("craving_cycle must not be empty".into());
        }
        for (name, u) in [
            ("queen", &self.queen),
            ("worker", &self.worker),
            ("soldier", &self.soldier),
            ("spider", &self.spider),
            ("honey", &self.honey),
            ("medic", &self.medic),
        ] {
            if !(u.hp.is_finite() && u.hp > 0.0) {
                errs.push(format!("{name} hp must be > 0 (got {})", u.hp));
            }
            if !(u.speed.is_finite() && u.speed > 0.0) {
                errs.push(format!("{name} speed must be > 0 (got {})", u.speed));
            }
            if u.dmg < 0.0 || u.atk_cd <= 0.0 || u.range <= 0.0 {
                errs.push(format!(
                    "{name} combat stats must be non-negative with positive cooldown and range (dmg {}, cd {}, range {})",
                    u.dmg, u.atk_cd, u.range
                ));
            }
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs)
        }
    }

    /// Stable fingerprint of the whole ruleset, folded into the canonical
    /// state — two sims only compare equal when they also run under equal
    /// rules. FNV-1a over the Debug rendering (deterministic per Rust's
    /// shortest-roundtrip float formatting, like every other canonical term).
    pub fn digest(&self) -> u64 {
        crate::sim::snapshot::fnv(format!("{self:?}").as_bytes())
    }

    /// The admin panel's field registry, as JSON: ordered groups with their
    /// field paths, per-leaf scope ("live" applies to the running sim right
    /// away, "new_game" only takes effect on the next world generation), and
    /// the nested-shape hints (unit-stats leaves, brood castes, food kinds)
    /// the form generator needs. Presentation sections live here too so the
    /// panel is generated entirely from core data — one source of truth.
    pub fn meta_json() -> String {
        // Fields consumed exclusively by world generation / the founding
        // ritual. Everything else is read per-tick or per-command and is
        // live-tunable (entity-copied stats get re-derived on commit).
        const NEW_GAME: &[&str] = &[
            "width",
            "height",
            "start_workers",
            "food_clusters",
            "spiders",
            "rock_chance",
            "orange_soil_chance",
            "silver_soil_chance",
            "patches_per_color",
            "patch_w",
            "patch_h",
            "patch_wobble",
            "patch_min_dist",
            "patch_grant_min",
            "patch_grant_max",
            "sources_near_nest",
            "source_min_gap",
            "near_ring_min",
            "near_ring_max",
            "spider_dist_min",
            "spider_dist_max",
            "wood_count",
            "wool_count",
            "sources",
            "start_food",
        ];
        let groups: &[(&str, &[&str])] = &[
            (
                "World & start",
                &[
                    "width",
                    "height",
                    "start_workers",
                    "food_clusters",
                    "max_ants",
                    "spiders",
                ],
            ),
            (
                "Worldgen scatter",
                &[
                    "rock_chance",
                    "orange_soil_chance",
                    "silver_soil_chance",
                    "patches_per_color",
                    "patch_w",
                    "patch_h",
                    "patch_wobble",
                    "patch_min_dist",
                    "patch_grant_min",
                    "patch_grant_max",
                    "sources_near_nest",
                    "source_min_gap",
                    "near_ring_min",
                    "near_ring_max",
                    "spider_dist_min",
                    "spider_dist_max",
                    "wood_count",
                    "wool_count",
                ],
            ),
            ("Map sources", &["sources"]),
            (
                "Economy",
                &[
                    "sight_range",
                    "protein_per_spider",
                    "super_per_spider",
                    "food_cell_cap",
                    "spoil_time",
                    "pile_amount",
                ],
            ),
            (
                "Unit stats",
                &[
                    "queen",
                    "worker",
                    "soldier",
                    "spider",
                    "honey",
                    "medic",
                    "ant_range",
                ],
            ),
            ("Brood costs", &["brood"]),
            (
                "Specialists",
                &["honey_period", "bleed_time", "heal_time", "water_per_heal"],
            ),
            (
                "Founding",
                &[
                    "queen_fly_speed",
                    "founding_time",
                    "founding_eggs",
                    "founding_egg_hatch",
                    "queen_dig_time",
                    "dirt_capacity",
                    "start_food",
                ],
            ),
            (
                "Queen economy",
                &[
                    "eat_period",
                    "starve_time",
                    "craving_cycle",
                    "lay_cooldown",
                    "egg_time",
                ],
            ),
            (
                "Nest logistics",
                &[
                    "dig_time",
                    "dig_expand_radius",
                    "loiter_radius",
                    "loiter_hops",
                    "home_rest_ticks",
                ],
            ),
            ("Predators", &["spider_aggro", "spider_wander"]),
            (
                "Legacy economy",
                &["egg_cost", "soldier_cost_green", "soldier_cost_super"],
            ),
        ];
        let mut scopes = serde_json::Map::new();
        for (name, fields) in groups {
            for f in *fields {
                // whole-value leaves (craving_cycle, sources, nested structs)
                // carry one scope for the entire value
                let scope = if NEW_GAME.contains(f) {
                    "new_game"
                } else {
                    "live"
                };
                scopes.insert(
                    (*f).to_string(),
                    serde_json::json!({ "scope": scope, "group": name }),
                );
            }
        }
        serde_json::json!({
            "groups": groups
                .iter()
                .map(|(name, fields)| serde_json::json!({
                    "name": name,
                    "fields": fields,
                }))
                .collect::<Vec<_>>(),
            "fields": scopes,
            "unit_fields": ["hp", "dmg", "atk_cd", "speed", "range"],
            "unit_groups": ["queen", "worker", "soldier", "spider", "honey", "medic"],
            "brood_castes": ["worker", "soldier", "honey", "medic"],
            "brood_fields": ["protein", "carbs", "water", "honeydew", "egg_time", "consumes_worker"],
            "food_kinds": ["green", "super", "protein", "carbs", "water", "honeydew"],
        })
        .to_string()
    }
}
