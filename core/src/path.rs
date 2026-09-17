use crate::world::Grid;
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
    let heuristic = |i: usize| -> u32 {
        let (x, y) = at(i);
        (x as i32 - goal.0 as i32).unsigned_abs() + (y as i32 - goal.1 as i32).unsigned_abs()
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
        for (dx, dy) in [(0u32, u32::MAX), (0, 1), (u32::MAX, 0), (1, 0)] {
            let nx = cx.wrapping_add(dx);
            let ny = cy.wrapping_add(dy);
            if !g.in_bounds(nx, ny) {
                continue;
            }
            let cost = match Grid::pass_cost(g.get(nx, ny), soft_passable) {
                Some(c) => c,
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

pub fn chebyshev(a: (u32, u32), b: (u32, u32)) -> u32 {
    (a.0 as i32 - b.0 as i32)
        .abs()
        .max((a.1 as i32 - b.1 as i32).abs()) as u32
}

pub fn manhattan(a: (u32, u32), b: (u32, u32)) -> u32 {
    (a.0 as i32 - b.0 as i32).unsigned_abs() + (a.1 as i32 - b.1 as i32).unsigned_abs()
}
