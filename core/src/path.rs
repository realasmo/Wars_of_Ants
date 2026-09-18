use crate::world::{tile_center, Grid};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

struct Node {
    f: u32,
    ctr: u32,
    i: u32,
}

impl Eq for Node {}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        (self.f, self.ctr) == (other.f, other.ctr)
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.f, self.ctr).cmp(&(other.f, other.ctr))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub fn find_path(
    g: &Grid,
    start: (u32, u32),
    goal: (u32, u32),
    soft_passable: bool,
) -> Option<Vec<(u32, u32)>> {
    if !g.in_bounds(start.0, start.1) || !g.in_bounds(goal.0, goal.1) {
        return None;
    }
    let w = g.w as usize;
    let n = (g.w * g.h) as usize;
    let idx = |x: u32, y: u32| (y as usize) * w + x as usize;
    let at = |i: usize| ((i % w) as u32, (i / w) as u32);
    // Octile distance on the same scale as pass_cost (straight = 1000):
    // exact on uniform terrain, so it never overestimates.
    let heuristic = |i: usize| -> u32 {
        let (x, y) = at(i);
        let dx = (x as i32 - goal.0 as i32).unsigned_abs();
        let dy = (y as i32 - goal.1 as i32).unsigned_abs();
        1000 * dx.max(dy) + 414 * dx.min(dy)
    };

    let s = idx(start.0, start.1);
    let t = idx(goal.0, goal.1);
    Grid::pass_cost(g.get(goal.0, goal.1), soft_passable)?;

    let mut g_score = vec![u32::MAX; n];
    let mut came = vec![u32::MAX; n];
    let mut open: BinaryHeap<Reverse<Node>> = BinaryHeap::new();
    let mut ctr = 0u32;
    g_score[s] = 0;
    open.push(Reverse(Node {
        f: heuristic(s),
        ctr,
        i: s as u32,
    }));
    ctr += 1;

    while let Some(Reverse(node)) = open.pop() {
        let cur = node.i as usize;
        if cur == t {
            let mut path = Vec::new();
            let mut i = cur;
            loop {
                path.push(at(i));
                if i == s {
                    break;
                }
                i = came[i] as usize;
            }
            path.reverse();
            return Some(path);
        }
        if node.f > g_score[cur].saturating_add(heuristic(cur)) {
            continue;
        }
        let (cx, cy) = at(cur);
        // 8-directional: diagonals cost ~sqrt(2) and are forbidden when they
        // would squeeze between two blocked orthogonal tiles (corner cutting).
        for (dx, dy) in [
            (0u32, u32::MAX),
            (0, 1),
            (u32::MAX, 0),
            (1, 0),
            (u32::MAX, u32::MAX),
            (u32::MAX, 1),
            (1, u32::MAX),
            (1, 1),
        ] {
            let nx = cx.wrapping_add(dx);
            let ny = cy.wrapping_add(dy);
            if !g.in_bounds(nx, ny) {
                continue;
            }
            let diagonal = dx != 0 && dy != 0;
            let cost = match Grid::pass_cost(g.get(nx, ny), soft_passable) {
                Some(c) => {
                    if diagonal {
                        let o1 = Grid::pass_cost(g.get(cx.wrapping_add(dx), cy), soft_passable);
                        let o2 = Grid::pass_cost(g.get(cx, cy.wrapping_add(dy)), soft_passable);
                        if o1.is_none() || o2.is_none() {
                            continue;
                        }
                        c.saturating_mul(1414) / 1000
                    } else {
                        c
                    }
                }
                None => continue,
            };
            let ni = idx(nx, ny);
            let ng = g_score[cur].saturating_add(cost);
            if ng < g_score[ni] {
                g_score[ni] = ng;
                came[ni] = cur as u32;
                open.push(Reverse(Node {
                    f: ng + heuristic(ni),
                    ctr,
                    i: ni as u32,
                }));
                ctr += 1;
            }
        }
    }
    None
}

pub fn tile_of(p: crate::math::Vec2) -> (u32, u32) {
    (p.x.floor() as u32, p.y.floor() as u32)
}

/// Supercover line-of-sight: every tile the segment from `a` to `b` passes
/// through must be EMPTY. Exact corner grazes require all tiles around the
/// corner. When `allow_soft_end` is set the final tile may be non-empty
/// (the ant digs into its goal tile). Pure f64 arithmetic — deterministic
/// across platforms (guarded by the cross-platform determinism test).
pub fn line_clear(g: &Grid, ax: f64, ay: f64, bx: f64, by: f64, allow_soft_end: bool) -> bool {
    let clear = |x: i64, y: i64| -> bool {
        if x < 0 || y < 0 || x >= g.w as i64 || y >= g.h as i64 {
            return false;
        }
        g.get(x as u32, y as u32) == crate::world::EMPTY
    };
    let end_clear = |x: i64, y: i64| -> bool {
        if x < 0 || y < 0 || x >= g.w as i64 || y >= g.h as i64 {
            return false;
        }
        if allow_soft_end && x == bx.floor() as i64 && y == by.floor() as i64 {
            return true;
        }
        clear(x, y)
    };
    let (mut tx, mut ty) = (ax.floor() as i64, ay.floor() as i64);
    let (tex, tey) = (bx.floor() as i64, by.floor() as i64);
    if !clear(tx, ty) || !end_clear(tex, tey) {
        return false;
    }
    let (dx, dy) = (bx - ax, by - ay);
    let stepx = if dx > 0.0 {
        1
    } else if dx < 0.0 {
        -1
    } else {
        0
    };
    let stepy = if dy > 0.0 {
        1
    } else if dy < 0.0 {
        -1
    } else {
        0
    };
    let mut tmaxx = if stepx == 0 {
        f64::INFINITY
    } else {
        let boundary = if stepx > 0 {
            tx as f64 + 1.0
        } else {
            tx as f64
        };
        (boundary - ax) / dx
    };
    let mut tmaxy = if stepy == 0 {
        f64::INFINITY
    } else {
        let boundary = if stepy > 0 {
            ty as f64 + 1.0
        } else {
            ty as f64
        };
        (boundary - ay) / dy
    };
    let tdeltax = if stepx == 0 {
        f64::INFINITY
    } else {
        1.0 / dx.abs()
    };
    let tdeltay = if stepy == 0 {
        f64::INFINITY
    } else {
        1.0 / dy.abs()
    };
    let mut guard = 0i32;
    while (tx, ty) != (tex, tey) {
        guard += 1;
        if guard > 100_000 {
            return false;
        }
        // end_clear == clear everywhere except the goal tile, where a soft
        // tile is tolerated when allow_soft_end is set (dig-into-goal)
        if tmaxx < tmaxy {
            tx += stepx;
            tmaxx += tdeltax;
            if !end_clear(tx, ty) {
                return false;
            }
        } else if tmaxy < tmaxx {
            ty += stepy;
            tmaxy += tdeltay;
            if !end_clear(tx, ty) {
                return false;
            }
        } else {
            // exact corner crossing: all tiles meeting at the corner must be clear
            tx += stepx;
            ty += stepy;
            tmaxx += tdeltax;
            tmaxy += tdeltay;
            if !end_clear(tx, ty) || !clear(tx - stepx, ty) || !clear(tx, ty - stepy) {
                return false;
            }
        }
    }
    true
}

/// String-pull smoothing: from the ant's exact position, greedily skip to the
/// furthest tile center in line of sight. Intermediate tiles must be EMPTY,
/// so soft (diggable) tiles always remain waypoints — movement stops there
/// and digs, exactly like unsmoothed paths. Returns `[start, ...waypoints]`.
pub fn smooth_path(
    g: &Grid,
    start: crate::math::Vec2,
    path: &[(u32, u32)],
) -> Vec<crate::math::Vec2> {
    let mut out = vec![start];
    if path.is_empty() {
        return out;
    }
    let mut i = 0usize;
    loop {
        let a = *out.last().unwrap();
        let mut j = path.len() - 1;
        let last = path.len() - 1;
        while j > i {
            let b = tile_center(path[j].0, path[j].1);
            if line_clear(g, a.x, a.y, b.x, b.y, j == last) {
                break;
            }
            j -= 1;
        }
        out.push(tile_center(path[j].0, path[j].1));
        if j == last {
            break;
        }
        i = j + 1;
    }
    out
}

pub fn chebyshev(a: (u32, u32), b: (u32, u32)) -> u32 {
    (a.0 as i32 - b.0 as i32)
        .abs()
        .max((a.1 as i32 - b.1 as i32).abs()) as u32
}

pub fn manhattan(a: (u32, u32), b: (u32, u32)) -> u32 {
    (a.0 as i32 - b.0 as i32).unsigned_abs() + (a.1 as i32 - b.1 as i32).unsigned_abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Grid, DIRT, EMPTY, ROCK};

    fn step_is_legal(g: &Grid, a: (u32, u32), b: (u32, u32), soft: bool) -> bool {
        let (dx, dy) = (b.0 as i32 - a.0 as i32, b.1 as i32 - a.1 as i32);
        if dx.abs() > 1 || dy.abs() > 1 {
            return false;
        }
        if dx != 0 && dy != 0 {
            let o1 = Grid::pass_cost(g.get((a.0 as i32 + dx) as u32, a.1), soft);
            let o2 = Grid::pass_cost(g.get(a.0, (a.1 as i32 + dy) as u32), soft);
            return o1.is_some() && o2.is_some();
        }
        true
    }

    #[test]
    fn takes_diagonal_shortcut() {
        let g = Grid::filled(10, 10, EMPTY);
        let p = find_path(&g, (2, 2), (5, 5), false).unwrap();
        assert_eq!(p.len(), 4, "3 diagonal steps + start, got {p:?}");
    }

    #[test]
    fn never_corner_cuts() {
        let mut g = Grid::filled(8, 8, EMPTY);
        g.set(3, 2, ROCK); // pillar straight ahead of a diagonal squeeze
        let p = find_path(&g, (2, 2), (4, 2), false).unwrap();
        assert_eq!(p.first(), Some(&(2, 2)));
        assert_eq!(p.last(), Some(&(4, 2)));
        for w in p.windows(2) {
            assert!(
                step_is_legal(&g, w[0], w[1], false),
                "illegal step {:?} -> {:?} in {p:?}",
                w[0],
                w[1]
            );
        }
    }

    #[test]
    fn dig_aware_costs_prefer_empty_detour() {
        // A straight soft lane vs a longer empty detour: the detour must win
        // (10 empty steps = 10000 < 2 soft steps = 20000).
        let mut g = Grid::filled(12, 5, ROCK);
        for x in 1..=10 {
            g.set(x, 2, DIRT); // soft lane through the middle
        }
        g.set(1, 1, EMPTY);
        g.set(10, 3, EMPTY);
        // carve an empty ring around the soft lane
        for x in 1..=10 {
            g.set(x, 1, EMPTY);
            g.set(x, 3, EMPTY);
        }
        g.set(1, 2, EMPTY); // start
        g.set(10, 2, EMPTY); // goal
        let p = find_path(&g, (1, 2), (10, 2), true).unwrap();
        assert!(
            p.iter().all(|&t| g.get(t.0, t.1) == EMPTY),
            "path crosses soft tiles: {p:?}"
        );
    }

    #[test]
    fn line_clear_basics() {
        let mut g = Grid::filled(10, 10, EMPTY);
        assert!(line_clear(&g, 2.5, 2.5, 6.5, 6.5, false)); // exact diagonal
        assert!(line_clear(&g, 2.5, 1.5, 6.5, 3.5, false)); // shallow slope
        g.set(4, 2, ROCK);
        assert!(!line_clear(&g, 2.5, 1.5, 6.5, 3.5, false)); // segment crosses (4,2)
        assert!(line_clear(&g, 2.5, 1.5, 3.5, 1.5, false)); // short of the rock
        assert!(line_clear(&g, 2.5, 1.5, 4.5, 1.5, true)); // soft end allowed at goal
    }

    #[test]
    fn smooth_collapses_open_straight_lines() {
        let g = Grid::filled(10, 10, EMPTY);
        let path = vec![(2u32, 2u32), (3, 3), (4, 4), (5, 5)];
        let s = smooth_path(&g, crate::math::Vec2::new(2.5, 2.5), &path);
        assert_eq!(s.len(), 2, "should collapse to [start, goal]: {s:?}");
        assert_eq!(s[1], crate::math::Vec2::new(5.5, 5.5));
    }

    #[test]
    fn smooth_keeps_dig_stops() {
        // Tunnel with one dirt tile mid-path: the dirt tile survives as a
        // waypoint (movement pauses there and digs before walking on — the
        // segment beyond it is only clear once dug, so exact waypoints here
        // are the route-time truth).
        let mut g = Grid::filled(10, 10, ROCK);
        for x in 1..=7 {
            g.set(x, 2, EMPTY);
        }
        g.set(4, 2, DIRT);
        let path = vec![(2u32, 2u32), (3, 2), (4, 2), (5, 2), (6, 2)];
        let s = smooth_path(&g, crate::math::Vec2::new(2.5, 2.5), &path);
        let want = vec![
            crate::math::Vec2::new(2.5, 2.5), // start
            crate::math::Vec2::new(3.5, 2.5), // walk to dirt boundary
            crate::math::Vec2::new(4.5, 2.5), // dig stop (dirt tile)
            crate::math::Vec2::new(5.5, 2.5), // forced hop (start tile still dirt)
            crate::math::Vec2::new(6.5, 2.5), // goal
        ];
        assert_eq!(s, want);
    }
}
