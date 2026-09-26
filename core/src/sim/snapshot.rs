//! The client-facing snapshot and the canonical state digest.

use super::Sim;
use crate::balance::{SPOIL_TIME, STARVE_TIME};
use crate::components::*;

#[derive(Clone, Debug, PartialEq)]
pub struct EntitySnap {
    pub id: u32,
    pub kind: u8,
    pub layer: u8,
    pub x: f64,
    pub y: f64,
    pub state: u8,
    pub extra: f64,
    pub hp: f64,
    pub aux: f64,
}

pub(crate) fn source_name(src: u8) -> &'static str {
    match src {
        1 => "moss",
        2 => "mushroom",
        3 => "raspberry",
        4 => "strawberry",
        5 => "cockroach",
        6 => "caterpillar",
        _ => "source",
    }
}

impl Sim {
    pub fn snapshot(&self) -> Vec<EntitySnap> {
        let mut rev = std::collections::HashMap::new();
        for (&id, &ent) in &self.ids {
            rev.insert(ent, id);
        }
        let mut v = Vec::new();
        for (ent, (ant, pos, state, carry, combat)) in self
            .ecs
            .query::<(&Ant, &Pos, &AntState, &Carry, &Combat)>()
            .iter()
        {
            let Some(&id) = rev.get(&ent) else { continue };
            let kind = match ant.caste {
                Caste::Queen => 0,
                Caste::Worker => 1,
                Caste::Soldier => 5,
            };
            let state = if ant.caste == Caste::Queen && self.colony.phase == super::Phase::Flight {
                4u8 // flying (founding queen airborne)
            } else {
                match state {
                    AntState::Idle => 0u8,
                    AntState::Moving { .. } => 1,
                    AntState::Digging { .. } => 2,
                    AntState::Fighting { .. } => 3,
                }
            };
            let extra = if ant.caste == Caste::Queen {
                self.colony.starve_t / STARVE_TIME
            } else {
                match *carry {
                    Carry::None => 0.0,
                    Carry::Dirt { blocks } => blocks as f64,
                    Carry::Egg | Carry::Food(_) => 1.0,
                }
            };
            v.push(EntitySnap {
                id,
                kind,
                layer: pos.layer as u8,
                x: pos.p.x,
                y: pos.p.y,
                state,
                extra,
                hp: (combat.hp / combat.max_hp).clamp(0.0, 1.0),
                // carried-item code while carrying, 0 when empty-handed (the
                // pre-Carry snapshot leaked the last kind forever)
                aux: match *carry {
                    Carry::None => 0.0,
                    Carry::Dirt { .. } => 2.0,
                    Carry::Egg => 3.0,
                    Carry::Food(k) => k as u8 as f64,
                },
            });
        }
        for (ent, (food, pos)) in self.ecs.query::<(&Food, &Pos)>().iter() {
            let Some(&id) = rev.get(&ent) else { continue };
            let is_source = food.harvest_t > 0.0;
            v.push(EntitySnap {
                id,
                kind: 2,
                layer: pos.layer as u8,
                x: pos.p.x,
                y: pos.p.y,
                // 0 loose unit, 1 pantry pile, 2 map source (hp = src type)
                state: if is_source { 2 } else if food.stored { 1 } else { 0 },
                extra: food.amount as f64,
                hp: if is_source {
                    food.src as f64
                } else {
                    food.spoil.map(|t| (t / SPOIL_TIME).clamp(0.0, 1.0)).unwrap_or(1.0)
                },
                aux: food.kind as u8 as f64,
            });
        }
        for (ent, (egg, pos)) in self.ecs.query::<(&Egg, &Pos)>().iter() {
            let Some(&id) = rev.get(&ent) else { continue };
            v.push(EntitySnap {
                id,
                kind: 3,
                layer: pos.layer as u8,
                x: pos.p.x,
                y: pos.p.y,
                state: if egg.carried_by.is_some() { 1 } else { 0 },
                extra: egg.hatch / egg.total,
                hp: 1.0,
                aux: if egg.caste == Caste::Soldier {
                    1.0
                } else {
                    0.0
                },
            });
        }
        for (ent, (pred, pos)) in self.ecs.query::<(&Predator, &Pos)>().iter() {
            let Some(&id) = rev.get(&ent) else { continue };
            v.push(EntitySnap {
                id,
                kind: 4,
                layer: pos.layer as u8,
                x: pos.p.x,
                y: pos.p.y,
                state: if pred.target.is_some() { 1 } else { 0 },
                extra: pred.hp / pred.max_hp,
                hp: pred.hp / pred.max_hp,
                aux: 0.0,
            });
        }
        v.sort_by_key(|s| s.id);
        v
    }

    /// Canonical, platform-independent digest of the full sim state.
    /// Native and WASM builds must produce byte-identical strings for the
    /// same seed + command script — this is what the cross-platform
    /// determinism test compares.
    pub fn canonical_state(&self) -> String {
        let entrance = self
            .world
            .entrance
            .map(|(x, y)| format!("{x},{y}"))
            .unwrap_or_else(|| "-".to_string());
        let mut s = format!(
            "t={};carbs={};protein={};water={};dead={};dug={};next_id={};phase={};phase_t={:.4};team={};ent={}",
            self.tick,
            self.colony.carbs,
            self.colony.protein,
            self.colony.water,
            self.colony.dead,
            self.dug_tiles,
            self.next_id,
            self.colony.phase as u8,
            self.colony.phase_t,
            self.colony.team as u8,
            entrance
        );
        for e in self.snapshot() {
            s.push_str(&format!(
                "|{},{},{},{},{:.4},{:.4},{:.4},{:.4},{:.4}",
                e.id, e.kind, e.layer, e.state, e.x, e.y, e.extra, e.hp, e.aux
            ));
        }
        s
    }
}
