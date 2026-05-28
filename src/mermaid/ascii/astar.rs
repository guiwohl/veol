use super::coord::Coord;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

pub fn find_path<F: Fn(Coord) -> bool>(
    from: Coord,
    to: Coord,
    max_x: i32,
    max_y: i32,
    is_free: F,
) -> Option<Vec<Coord>> {
    let mut open: BinaryHeap<Reverse<(i32, Coord)>> = BinaryHeap::new();
    open.push(Reverse((0, from)));
    let mut cost: HashMap<Coord, i32> = HashMap::new();
    cost.insert(from, 0);
    let mut came: HashMap<Coord, Coord> = HashMap::new();
    let dirs = [
        Coord::new(1, 0),
        Coord::new(-1, 0),
        Coord::new(0, 1),
        Coord::new(0, -1),
    ];

    while let Some(Reverse((_, cur))) = open.pop() {
        if cur == to {
            let mut path = vec![cur];
            let mut c = cur;
            while let Some(&prev) = came.get(&c) {
                path.push(prev);
                c = prev;
            }
            path.reverse();
            return Some(path);
        }
        for d in dirs {
            let nx = cur.x + d.x;
            let ny = cur.y + d.y;
            if nx < 0 || ny < 0 || nx > max_x || ny > max_y {
                continue;
            }
            let next = Coord::new(nx, ny);
            if next != to && !is_free(next) {
                continue;
            }
            let new_cost = cost[&cur] + 1;
            if cost.get(&next).is_none_or(|&c| new_cost < c) {
                cost.insert(next, new_cost);
                let dx = (next.x - to.x).abs();
                let dy = (next.y - to.y).abs();
                let h = dx + dy + if dx != 0 && dy != 0 { 1 } else { 0 };
                let prio = new_cost + h;
                open.push(Reverse((prio, next)));
                came.insert(next, cur);
            }
        }
    }
    None
}
