//! World generation: grids, rock scatter, hidden soil blocks, surface dust
//! patches, starting entities, sources (founding) or green clusters (legacy),
//! spiders.

use super::{Colony, Config, Patch, Phase, Sim, Team};
use crate::balance::*;
use crate::components::{Caste, FoodKind, Layer};
use crate::math::Vec2;
use crate::world::tile_center;
use crate::rng::Rng;
use crate::world::{Grid, World, DIRT, EMPTY, ROCK, SOIL_NONE, SOIL_ORANGE, SOIL_SILVER};

impl Sim {
    pub(crate) fn build(seed: u64, config: Config, founding: bool, team: Team) -> Sim {
        let w = config.width;
        let h = config.height;
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
                if rng.f64() < 0.08 {
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
        let mut soil_surface = vec![SOIL_NONE as u8; (w * h) as usize];
        let mut soil_underground = vec![SOIL_NONE as u8; (w * h) as usize];
        let mut patches: Vec<Patch> = Vec::new();
        if founding {
            // IMPORTANT: soil blocks must align with the dig grid (even
            // origins, x & !1) — a half-offset soil block straddles two dig
            // blocks and looks/tunnels wrong (reported: orange half under
            // dirt, neighbor undiggable behind its rock cells)
            for by in (2..h - 3).step_by(2) {
                for bx in (2..w - 3).step_by(2) {
                    let r = rng.f64();
                    let soil = if r < ORANGE_SOIL_CHANCE {
                        SOIL_ORANGE
                    } else if r < ORANGE_SOIL_CHANCE + SILVER_SOIL_CHANCE {
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
                for _ in 0..PATCHES_PER_COLOR {
                    for _ in 0..40 {
                        let pw = PATCH_W + rng.irange(0, PATCH_WOBBLE * 2) - PATCH_WOBBLE;
                        let ph = PATCH_H + rng.irange(0, PATCH_WOBBLE * 2) - PATCH_WOBBLE;
                        if pw < 4 || ph < 4 || pw > w - 8 || ph > h - 8 {
                            continue;
                        }
                        let x0 = rng.irange(4, w - pw - 4);
                        let y0 = rng.irange(4, h - ph - 4);
                        let (x1, y1) = (x0 + pw, y0 + ph);
                        let far = patches.iter().all(|p| {
                            let cx = ((x0 + x1) as i32 - (p.x0 + p.x1) as i32).abs() / 2;
                            let cy = ((y0 + y1) as i32 - (p.y0 + p.y1) as i32).abs() / 2;
                            cx.max(cy) >= 20
                        });
                        if !far {
                            continue;
                        }
                        for y in y0..y1 {
                            for x in x0..x1 {
                                soil_surface[(y * w + x) as usize] = soil;
                            }
                        }
                        patches.push(Patch { x0, y0, x1, y1, soil });
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
                carbs: START_FOOD,
                protein: 0,
                water: 0,
                delivered: 0,
                eggs_laid: 0,
                dead: false,
                queen_id: 0,
                ant_count: 0,
                starve_t: 0.0,
                eat_t: 0.0,
                lay_cooldown: LAY_COOLDOWN,
                dig_queue: Vec::new(),
                phase: if founding { Phase::Flight } else { Phase::Colony },
                phase_t: 0.0,
                team,
                founding,
                known: std::collections::BTreeSet::new(),
                slowed: false,
            },
            config,
            tick: 0,
            events: Vec::new(),
            event_total: 0,
            patches,
            ids: std::collections::BTreeMap::new(),
            next_id: 0,
            dug_tiles: 0,
        };

        let (queen_id, anchor) = if founding {
            let c = tile_center(w / 2, h / 2);
            (sim.spawn_ant(Caste::Queen, c, Layer::Surface), w / 2)
        } else {
            let q = sim.spawn_ant(Caste::Queen, tile_center(e, 2), Layer::Underground);
            for i in 0..sim.config.start_workers {
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
            for spec in SOURCES.iter() {
                for _ in 0..spec.count {
                    for _ in 0..60 {
                        let want_near =
                            spec.kind == FoodKind::Carbs && near_count < SOURCES_NEAR_NEST;
                        let (px, py) = if want_near {
                            let a = sim.rng.range(0.0, std::f64::consts::TAU);
                            let d = sim.rng.range(7.0, 14.0);
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
                            dx.max(dy) < SOURCE_MIN_GAP as i32
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
        } else {
            for _ in 0..sim.config.food_clusters {
                let cx =
                    (anchor as i32 + sim.rng.irange(0, 41) as i32 - 20).clamp(2, w as i32 - 3) as u32;
                let cy = sim.rng.irange(15, 41).min(h - 3);
                let piles = sim.rng.irange(4, 8);
                for _ in 0..piles {
                    let px =
                        (cx as i32 + sim.rng.irange(0, 5) as i32 - 2).clamp(1, w as i32 - 2) as u32;
                    let py =
                        (cy as i32 + sim.rng.irange(0, 5) as i32 - 2).clamp(1, h as i32 - 2) as u32;
                    sim.spawn_food(tile_center(px, py), PILE_AMOUNT, FoodKind::Green);
                }
            }
        }

        for _ in 0..sim.config.spiders {
            let dx = sim.rng.range(-1.0, 1.0);
            let dy = sim.rng.range(-1.0, 1.0);
            let len = (dx * dx + dy * dy).sqrt().max(0.001);
            let dist = sim.rng.range(25.0, 45.0);
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
