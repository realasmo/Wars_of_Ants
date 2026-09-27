//! World generation: grids, rock scatter, hidden soil blocks, surface dust
//! patches, starting entities, sources (founding) or green clusters (legacy),
//! spiders.

use super::{Colony, Patch, Phase, Sim, Team};
use crate::components::{Caste, CollectibleVariant, FoodKind, Layer};
use crate::math::Vec2;
use crate::rng::Rng;
use crate::rules::GameRules;
use crate::world::tile_center;
use crate::world::{Grid, World, DIRT, EMPTY, ROCK, SOIL_NONE, SOIL_ORANGE, SOIL_SILVER};

impl Sim {
    pub(crate) fn build(seed: u64, rules: GameRules, founding: bool, team: Team) -> Sim {
        let w = rules.width;
        let h = rules.height;
        let e = w / 2;
        let mut rng = Rng::new(seed);

        let surface = Grid::filled(w, h, EMPTY);
        let mut underground = Grid::filled(w, h, DIRT);
        for x in 0..w {
            underground.set(x, 0, ROCK);
            underground.set(x, h - 1, ROCK);
        }
        for y in 0..h {
            underground.set(0, y, ROCK);
            underground.set(w - 1, y, ROCK);
        }
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                // legacy start keeps a rock-free zone around the fixed nest
                if !founding && y < 8 && (x as i32 - e as i32).abs() <= 4 {
                    continue;
                }
                if rng.f64() < rules.rock_chance {
                    underground.set(x, y, ROCK);
                }
            }
        }
        let entrance = if founding {
            None
        } else {
            for y in 1..=3 {
                for x in e.saturating_sub(1)..=e + 1 {
                    underground.set(x, y, EMPTY);
                }
            }
            underground.set(e, 0, EMPTY);
            Some((e, 0))
        };

        // soil quality: founding mode seeds orange/silver 2×2 blocks hidden
        // in the underground dirt, plus surface dust patches
        let mut soil_surface = vec![SOIL_NONE; (w * h) as usize];
        let mut soil_underground = vec![SOIL_NONE; (w * h) as usize];
        let mut patches: Vec<Patch> = Vec::new();
        if founding {
            // IMPORTANT: soil blocks must align with the dig grid (even
            // origins, x & !1) — a half-offset soil block straddles two dig
            // blocks and looks/tunnels wrong (reported: orange half under
            // dirt, neighbor undiggable behind its rock cells)
            for by in (2..h - 3).step_by(2) {
                for bx in (2..w - 3).step_by(2) {
                    let r = rng.f64();
                    let soil = if r < rules.orange_soil_chance {
                        SOIL_ORANGE
                    } else if r < rules.orange_soil_chance + rules.silver_soil_chance {
                        SOIL_SILVER
                    } else {
                        SOIL_NONE
                    };
                    if soil != SOIL_NONE {
                        for dy in 0..2 {
                            for dx in 0..2 {
                                soil_underground[((by + dy) * w + bx + dx) as usize] = soil;
                                // special soil must stay diggable: the rock
                                // scatter may never land inside it
                                underground.set(bx + dx, by + dy, DIRT);
                            }
                        }
                    }
                }
            }
            for soil in [SOIL_ORANGE, SOIL_SILVER] {
                for _ in 0..rules.patches_per_color {
                    for _ in 0..40 {
                        let pw = rules.patch_w + rng.irange(0, rules.patch_wobble * 2)
                            - rules.patch_wobble;
                        let ph = rules.patch_h + rng.irange(0, rules.patch_wobble * 2)
                            - rules.patch_wobble;
                        if pw < 4 || ph < 4 || pw > w - 8 || ph > h - 8 {
                            continue;
                        }
                        let x0 = rng.irange(4, w - pw - 4);
                        let y0 = rng.irange(4, h - ph - 4);
                        let (x1, y1) = (x0 + pw, y0 + ph);
                        let far = patches.iter().all(|p| {
                            let cx = ((x0 + x1) as i32 - (p.x0 + p.x1) as i32).abs() / 2;
                            let cy = ((y0 + y1) as i32 - (p.y0 + p.y1) as i32).abs() / 2;
                            cx.max(cy) >= rules.patch_min_dist as i32
                        });
                        if !far {
                            continue;
                        }
                        for y in y0..y1 {
                            for x in x0..x1 {
                                soil_surface[(y * w + x) as usize] = soil;
                            }
                        }
                        patches.push(Patch {
                            x0,
                            y0,
                            x1,
                            y1,
                            soil,
                        });
                        break;
                    }
                }
            }
        }

        let mut sim = Sim {
            world: World {
                surface,
                underground,
                entrance,
                soil_surface,
                soil_underground,
            },
            ecs: hecs::World::new(),
            rng,
            colony: Colony {
                carbs: rules.start_food,
                protein: 0,
                water: 0,
                honeydew: 0,
                delivered: 0,
                eggs_laid: 0,
                dead: false,
                queen_id: 0,
                ant_count: 0,
                starve_t: 0.0,
                eat_t: 0.0,
                hunger_t: 0.0,
                craving_i: 0,
                feeder_id: None,
                lay_cooldown: rules.lay_cooldown,
                dig_queue: Vec::new(),
                phase: if founding {
                    Phase::Flight
                } else {
                    Phase::Colony
                },
                phase_t: 0.0,
                team,
                founding,
                known: std::collections::BTreeSet::new(),
            },
            rules: rules.clone(),
            tick: 0,
            events: std::collections::VecDeque::new(),
            event_total: 0,
            patches,
            ids: std::collections::BTreeMap::new(),
            next_id: 0,
            dug_tiles: 0,
            food_tiles: std::collections::BTreeMap::new(),
            tiles_epoch: 1,
            soil_epoch: 1,
        };

        let (queen_id, anchor) = if founding {
            let c = tile_center(w / 2, h / 2);
            (sim.spawn_ant(Caste::Queen, c, Layer::Surface), w / 2)
        } else {
            let q = sim.spawn_ant(Caste::Queen, tile_center(e, 2), Layer::Underground);
            for i in 0..sim.rules.start_workers {
                let x = e.saturating_sub(1) + (i % 3);
                let y = 3 + (i / 3);
                sim.spawn_ant(Caste::Worker, tile_center(x, y), Layer::Underground);
            }
            (q, e)
        };
        sim.colony.queen_id = queen_id;

        if founding {
            // finite scattered sources: a couple of carb sources near the
            // founding center so the first workers can survive, everything
            // else spread wide — scouts have to find them
            let mut placed: Vec<(u32, u32)> = Vec::new();
            let mut near_count = 0u32;
            let cy0 = h as f64 / 2.0;
            for spec in rules.sources.iter() {
                for _ in 0..spec.count {
                    for _ in 0..60 {
                        let want_near =
                            spec.kind == FoodKind::Carbs && near_count < rules.sources_near_nest;
                        let (px, py) = if want_near {
                            let a = sim.rng.range(0.0, std::f64::consts::TAU);
                            let d = sim.rng.range(rules.near_ring_min, rules.near_ring_max);
                            (
                                (anchor as f64 + a.cos() * d).clamp(3.0, w as f64 - 4.0) as u32,
                                (cy0 + a.sin() * d).clamp(3.0, h as f64 - 4.0) as u32,
                            )
                        } else {
                            (sim.rng.irange(6, w - 7), sim.rng.irange(6, h - 7))
                        };
                        let too_close = placed.iter().any(|&(x, y)| {
                            let dx = (x.max(px) - x.min(px)) as i32;
                            let dy = (y.max(py) - y.min(py)) as i32;
                            dx.max(dy) < rules.source_min_gap as i32
                        });
                        if too_close {
                            continue;
                        }
                        let amount = sim.rng.irange(spec.amount.0, spec.amount.1);
                        sim.spawn_source(tile_center(px, py), spec, amount);
                        placed.push((px, py));
                        if want_near {
                            near_count += 1;
                        }
                        break;
                    }
                }
            }
            // nest-building collectibles: wet wood + dry wool scattered on
            // the surface, clear of the food sources (F2)
            for (count, variant) in [
                (rules.wood_count, CollectibleVariant::Wood),
                (rules.wool_count, CollectibleVariant::Wool),
            ] {
                'cplace: for _ in 0..count {
                    for _ in 0..60 {
                        let px = sim.rng.irange(4, w - 5);
                        let py = sim.rng.irange(4, h - 5);
                        let too_close = placed.iter().any(|&(x, y)| {
                            let dx = (x.max(px) - x.min(px)) as i32;
                            let dy = (y.max(py) - y.min(py)) as i32;
                            dx.max(dy) < 3
                        });
                        if too_close {
                            continue;
                        }
                        sim.spawn_collectible(Vec2::new(px as f64 + 0.5, py as f64 + 0.5), variant);
                        placed.push((px, py));
                        continue 'cplace;
                    }
                }
            }
        } else {
            for _ in 0..sim.rules.food_clusters {
                let cx = (anchor as i32 + sim.rng.irange(0, 41) as i32 - 20).clamp(2, w as i32 - 3)
                    as u32;
                let cy = sim.rng.irange(15, 41).min(h - 3);
                let piles = sim.rng.irange(4, 8);
                for _ in 0..piles {
                    let px =
                        (cx as i32 + sim.rng.irange(0, 5) as i32 - 2).clamp(1, w as i32 - 2) as u32;
                    let py =
                        (cy as i32 + sim.rng.irange(0, 5) as i32 - 2).clamp(1, h as i32 - 2) as u32;
                    sim.spawn_food(tile_center(px, py), rules.pile_amount, FoodKind::Green);
                }
            }
        }

        for _ in 0..sim.rules.spiders {
            let dx = sim.rng.range(-1.0, 1.0);
            let dy = sim.rng.range(-1.0, 1.0);
            let len = (dx * dx + dy * dy).sqrt().max(0.001);
            let dist = sim.rng.range(rules.spider_dist_min, rules.spider_dist_max);
            let sx = ((anchor as f64 + dx / len * dist) as i32).clamp(2, w as i32 - 3) as f64 + 0.5;
            let sy = if founding {
                ((h as f64 / 2.0 + dy / len * dist) as i32).clamp(2, h as i32 - 3) as f64 + 0.5
            } else {
                ((dy / len * dist) as i32).clamp(2, h as i32 - 3) as f64 + 0.5
            };
            sim.spawn_spider(Vec2::new(sx, sy));
        }
        sim
    }
}
