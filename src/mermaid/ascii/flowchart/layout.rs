use super::ast::Flowchart;
use crate::mermaid::ascii::coord::{Coord, Direction};
use crate::mermaid::ascii::error::AsciiRenderError;
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone)]
pub struct EdgeRoute {
    pub path: Vec<Coord>,
    pub start_dir: Direction,
    pub end_dir: Direction,
    pub label_segment: Option<(Coord, Coord)>,
}

#[derive(Debug, Clone)]
pub struct SubgraphBox {
    pub min_x: usize,
    pub min_y: usize,
    pub max_x: usize,
    pub max_y: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Layout {
    pub node_grid: HashMap<String, GridPos>,
    pub column_widths: HashMap<i32, usize>,
    pub row_heights: HashMap<i32, usize>,
    pub edges: Vec<EdgeRoute>,
    pub subgraph_boxes: IndexMap<String, SubgraphBox>,
    pub canvas_width: usize,
    pub canvas_height: usize,
    pub offset_x: usize,
    pub offset_y: usize,
}

const PADDING_X: usize = 5;
const PADDING_Y: usize = 3;
const BOX_BORDER_PADDING: usize = 1;
const SUBGRAPH_PADDING: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutDir {
    LR,
    TD,
}

fn layout_dir(flow: &Flowchart) -> LayoutDir {
    use crate::mermaid::ascii::detect::FlowDirection::*;
    match flow.direction {
        LeftRight | RightLeft => LayoutDir::LR,
        TopDown | BottomTop => LayoutDir::TD,
    }
}

/// Direction offsets relative to the upper-left corner of a node's 3x3 footprint.
/// These match the Go `direction` constants: Up=(1,0), Down=(1,2), Left=(0,1), Right=(2,1).
fn attach_offset(d: Direction) -> (i32, i32) {
    match d {
        Direction::Up => (1, 0),
        Direction::Down => (1, 2),
        Direction::Left => (0, 1),
        Direction::Right => (2, 1),
        Direction::UpperLeft => (0, 0),
        Direction::UpperRight => (2, 0),
        Direction::LowerLeft => (0, 2),
        Direction::LowerRight => (2, 2),
        Direction::Middle => (1, 1),
    }
}

fn rel_direction(from: (i32, i32), to: (i32, i32)) -> Direction {
    Direction::from_to(Coord::new(from.0, from.1), Coord::new(to.0, to.1))
}

fn determine_start_end_dir(
    from: (i32, i32),
    to: (i32, i32),
    dir: LayoutDir,
    self_ref: bool,
) -> (Direction, Direction, Direction, Direction) {
    if self_ref {
        return match dir {
            LayoutDir::LR => (
                Direction::Right,
                Direction::Down,
                Direction::Down,
                Direction::Right,
            ),
            LayoutDir::TD => (
                Direction::Down,
                Direction::Right,
                Direction::Right,
                Direction::Down,
            ),
        };
    }
    let d = rel_direction(from, to);
    match d {
        Direction::LowerRight => match dir {
            LayoutDir::LR => (
                Direction::Down,
                Direction::Left,
                Direction::Right,
                Direction::Up,
            ),
            LayoutDir::TD => (
                Direction::Right,
                Direction::Up,
                Direction::Down,
                Direction::Left,
            ),
        },
        Direction::UpperRight => match dir {
            LayoutDir::LR => (
                Direction::Up,
                Direction::Left,
                Direction::Right,
                Direction::Down,
            ),
            LayoutDir::TD => (
                Direction::Right,
                Direction::Down,
                Direction::Up,
                Direction::Left,
            ),
        },
        Direction::LowerLeft => match dir {
            LayoutDir::LR => (
                Direction::Down,
                Direction::Down,
                Direction::Left,
                Direction::Up,
            ),
            LayoutDir::TD => (
                Direction::Left,
                Direction::Up,
                Direction::Down,
                Direction::Right,
            ),
        },
        Direction::UpperLeft => match dir {
            LayoutDir::LR => (
                Direction::Down,
                Direction::Down,
                Direction::Left,
                Direction::Down,
            ),
            LayoutDir::TD => (
                Direction::Right,
                Direction::Right,
                Direction::Up,
                Direction::Right,
            ),
        },
        Direction::Left if dir == LayoutDir::LR => (
            Direction::Down,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ),
        Direction::Up if dir == LayoutDir::TD => (
            Direction::Right,
            Direction::Right,
            Direction::Up,
            Direction::Down,
        ),
        _ => (d, d.opposite(), d, d.opposite()),
    }
}

fn reserve_spot(
    grid: &mut HashMap<(i32, i32), String>,
    node_id: &str,
    requested: (i32, i32),
    dir: LayoutDir,
) -> (i32, i32) {
    let mut cur = requested;
    loop {
        // Check 3x3 footprint
        let mut collision = false;
        'outer: for dx in 0..3 {
            for dy in 0..3 {
                if let Some(owner) = grid.get(&(cur.0 + dx, cur.1 + dy)) {
                    if owner != node_id {
                        collision = true;
                        break 'outer;
                    }
                }
            }
        }
        if !collision {
            break;
        }
        match dir {
            LayoutDir::LR => cur.1 += 4,
            LayoutDir::TD => cur.0 += 4,
        }
    }
    for dx in 0..3 {
        for dy in 0..3 {
            grid.insert((cur.0 + dx, cur.1 + dy), node_id.to_string());
        }
    }
    cur
}

fn merge_path(path: Vec<Coord>) -> Vec<Coord> {
    if path.len() <= 2 {
        return path;
    }
    let mut out = vec![path[0]];
    for i in 1..path.len() - 1 {
        let prev = out[out.len() - 1];
        let cur = path[i];
        let next = path[i + 1];
        let prev_dir = Direction::from_to(prev, cur);
        let next_dir = Direction::from_to(cur, next);
        if prev_dir != next_dir {
            out.push(cur);
        }
    }
    out.push(*path.last().unwrap());
    out
}

fn is_node_column(x: i32, node_grid: &HashMap<String, GridPos>) -> bool {
    for pos in node_grid.values() {
        if x >= pos.x && x <= pos.x + 2 {
            return true;
        }
    }
    false
}

fn label_middle_x(segment: (Coord, Coord)) -> i32 {
    let (a, b) = segment;
    let (lo, hi) = if a.x <= b.x { (a.x, b.x) } else { (b.x, a.x) };
    lo + (hi - lo) / 2
}

fn segment_width(segment: (Coord, Coord), column_widths: &HashMap<i32, usize>) -> usize {
    let mut w = 0;
    w += column_widths.get(&segment.0.x).copied().unwrap_or(1);
    w += column_widths.get(&segment.1.x).copied().unwrap_or(1);
    w
}

fn determine_label_segment(
    path: &[Coord],
    label_len: usize,
    column_widths: &mut HashMap<i32, usize>,
    node_grid: &HashMap<String, GridPos>,
) -> Option<(Coord, Coord)> {
    if label_len == 0 || path.len() < 2 {
        return None;
    }

    let mut largest: Option<(Coord, Coord)> = None;
    let mut largest_size: usize = 0;
    let mut fallback: Option<(Coord, Coord)> = None;
    let mut fallback_size: usize = 0;

    for w in path.windows(2) {
        let seg = (w[0], w[1]);
        let lw = segment_width(seg, column_widths);
        if is_node_column(label_middle_x(seg), node_grid) {
            if lw > fallback_size {
                fallback_size = lw;
                fallback = Some(seg);
            }
            continue;
        }
        if lw >= label_len {
            largest = Some(seg);
            break;
        }
        if lw > largest_size {
            largest_size = lw;
            largest = Some(seg);
        }
    }

    let chosen = largest.or(fallback).or_else(|| {
        if path.len() >= 2 {
            Some((path[0], path[1]))
        } else {
            None
        }
    });
    if let Some(seg) = chosen {
        let mx = label_middle_x(seg);
        let cur = column_widths.get(&mx).copied().unwrap_or(0);
        let needed = label_len + 2;
        if needed > cur {
            column_widths.insert(mx, needed);
        }
    }
    chosen
}

pub fn layout(flow: &Flowchart) -> Result<Layout, AsciiRenderError> {
    if flow.nodes.is_empty() {
        return Ok(Layout::default());
    }
    let dir = layout_dir(flow);

    // Build adjacency (children) and root list.
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    let mut has_incoming: HashSet<String> = HashSet::new();
    for edge in &flow.edges {
        if edge.from != edge.to {
            has_incoming.insert(edge.to.clone());
        }
        children
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
    }

    let roots: Vec<String> = flow
        .nodes
        .keys()
        .filter(|id| !has_incoming.contains(*id))
        .cloned()
        .collect();
    let roots = if roots.is_empty() {
        flow.nodes.keys().cloned().collect::<Vec<_>>()
    } else {
        roots
    };

    // Placement state.
    let mut grid_occ: HashMap<(i32, i32), String> = HashMap::new();
    let mut node_grid: HashMap<String, GridPos> = HashMap::new();
    let mut highest: HashMap<i32, i32> = HashMap::new();

    // Place roots at level 0.
    for root in &roots {
        let lvl0 = *highest.get(&0).unwrap_or(&0);
        let requested = match dir {
            LayoutDir::LR => (0, lvl0),
            LayoutDir::TD => (lvl0, 0),
        };
        let placed = reserve_spot(&mut grid_occ, root, requested, dir);
        node_grid.insert(
            root.clone(),
            GridPos {
                x: placed.0,
                y: placed.1,
            },
        );
        let new_high = match dir {
            LayoutDir::LR => placed.1 + 4,
            LayoutDir::TD => placed.0 + 4,
        };
        let cur = *highest.get(&0).unwrap_or(&0);
        highest.insert(0, cur.max(new_high));
    }

    // BFS placement.
    let mut queue: VecDeque<String> = VecDeque::new();
    for r in &roots {
        queue.push_back(r.clone());
    }
    let mut visited: HashSet<String> = HashSet::new();
    for r in &roots {
        visited.insert(r.clone());
    }

    while let Some(n) = queue.pop_front() {
        let pos = match node_grid.get(&n) {
            Some(p) => *p,
            None => continue,
        };
        let child_level = match dir {
            LayoutDir::LR => pos.x + 4,
            LayoutDir::TD => pos.y + 4,
        };
        let kids = children.get(&n).cloned().unwrap_or_default();
        for child in kids {
            if node_grid.contains_key(&child) {
                if !visited.contains(&child) {
                    visited.insert(child.clone());
                    queue.push_back(child);
                }
                continue;
            }
            let h = *highest.get(&child_level).unwrap_or(&0);
            let requested = match dir {
                LayoutDir::LR => (child_level, h),
                LayoutDir::TD => (h, child_level),
            };
            let placed = reserve_spot(&mut grid_occ, &child, requested, dir);
            node_grid.insert(
                child.clone(),
                GridPos {
                    x: placed.0,
                    y: placed.1,
                },
            );
            let new_high = match dir {
                LayoutDir::LR => placed.1 + 4,
                LayoutDir::TD => placed.0 + 4,
            };
            let cur = *highest.get(&child_level).unwrap_or(&0);
            highest.insert(child_level, cur.max(new_high));
            visited.insert(child.clone());
            queue.push_back(child);
        }
    }

    // Place any orphans (disconnected nodes not yet placed).
    for id in flow.nodes.keys() {
        if !node_grid.contains_key(id) {
            let lvl0 = *highest.get(&0).unwrap_or(&0);
            let requested = match dir {
                LayoutDir::LR => (0, lvl0),
                LayoutDir::TD => (lvl0, 0),
            };
            let placed = reserve_spot(&mut grid_occ, id, requested, dir);
            node_grid.insert(
                id.clone(),
                GridPos {
                    x: placed.0,
                    y: placed.1,
                },
            );
            let new_high = match dir {
                LayoutDir::LR => placed.1 + 4,
                LayoutDir::TD => placed.0 + 4,
            };
            let cur = *highest.get(&0).unwrap_or(&0);
            highest.insert(0, cur.max(new_high));
        }
    }

    // Column widths / row heights.
    let mut column_widths: HashMap<i32, usize> = HashMap::new();
    let mut row_heights: HashMap<i32, usize> = HashMap::new();

    for (id, pos) in &node_grid {
        let node = &flow.nodes[id];
        let col1 = 1;
        let col2 = 2 * BOX_BORDER_PADDING + node.label.width().max(node.id.len());
        let col3 = 1;
        let cols = [col1, col2, col3];
        let row1 = 1;
        let row2 = node.label.height() + 2 * BOX_BORDER_PADDING;
        let row3 = 1;
        let rows = [row1, row2, row3];

        for (i, &w) in cols.iter().enumerate() {
            let key = pos.x + i as i32;
            let cur = column_widths.get(&key).copied().unwrap_or(0);
            column_widths.insert(key, cur.max(w));
        }
        for (i, &h) in rows.iter().enumerate() {
            let key = pos.y + i as i32;
            let cur = row_heights.get(&key).copied().unwrap_or(0);
            row_heights.insert(key, cur.max(h));
        }
        if pos.x > 0 {
            let key = pos.x - 1;
            let cur = column_widths.get(&key).copied().unwrap_or(0);
            column_widths.insert(key, cur.max(PADDING_X));
        }
        if pos.y > 0 {
            let key = pos.y - 1;
            let cur = row_heights.get(&key).copied().unwrap_or(0);
            row_heights.insert(key, cur.max(PADDING_Y));
        }
    }

    // Edge routes.
    let mut edge_routes: Vec<EdgeRoute> = Vec::with_capacity(flow.edges.len());

    // Determine grid bounds for A*.
    let mut max_gx: i32 = 0;
    let mut max_gy: i32 = 0;
    for pos in node_grid.values() {
        max_gx = max_gx.max(pos.x + 2);
        max_gy = max_gy.max(pos.y + 2);
    }
    // A bit of corridor headroom for path routing.
    let path_bound_x = max_gx + 6;
    let path_bound_y = max_gy + 6;

    let mut edge_counts: HashMap<(String, String), i32> = HashMap::new();

    for edge in &flow.edges {
        let from_pos = match node_grid.get(&edge.from) {
            Some(p) => *p,
            None => {
                edge_routes.push(EdgeRoute {
                    path: Vec::new(),
                    start_dir: Direction::Middle,
                    end_dir: Direction::Middle,
                    label_segment: None,
                });
                continue;
            }
        };
        let to_pos = match node_grid.get(&edge.to) {
            Some(p) => *p,
            None => {
                edge_routes.push(EdgeRoute {
                    path: Vec::new(),
                    start_dir: Direction::Middle,
                    end_dir: Direction::Middle,
                    label_segment: None,
                });
                continue;
            }
        };
        let key = if edge.from <= edge.to {
            (edge.from.clone(), edge.to.clone())
        } else {
            (edge.to.clone(), edge.from.clone())
        };
        let dup_index = edge_counts.get(&key).copied().unwrap_or(0);

        let self_ref = edge.from == edge.to;
        let (pref_a, pref_b, alt_a, alt_b) = determine_start_end_dir(
            (from_pos.x, from_pos.y),
            (to_pos.x, to_pos.y),
            dir,
            self_ref,
        );

        // Try parallel directions for duplicates.
        let (effective_a, effective_b) = if dup_index > 0 {
            let from = (from_pos.x, from_pos.y);
            let to = (to_pos.x, to_pos.y);
            let d = rel_direction(from, to);
            let par = match dir {
                LayoutDir::LR if d == Direction::Right || d == Direction::Left => {
                    let opts = [
                        (Direction::Down, Direction::Down),
                        (Direction::Up, Direction::Up),
                    ];
                    opts.get((dup_index - 1) as usize).copied()
                }
                LayoutDir::TD if d == Direction::Down || d == Direction::Up => {
                    let opts = [
                        (Direction::Right, Direction::Right),
                        (Direction::Left, Direction::Left),
                    ];
                    opts.get((dup_index - 1) as usize).copied()
                }
                _ => None,
            };
            match par {
                Some((a, b)) => (a, b),
                None => (pref_a, pref_b),
            }
        } else {
            (pref_a, pref_b)
        };

        // Build occupancy map: cells inside any node's 3x3 are blocked except attach cell.
        let from_off = attach_offset(effective_a);
        let to_off = attach_offset(effective_b);
        let from_attach = Coord::new(from_pos.x + from_off.0, from_pos.y + from_off.1);
        let to_attach = Coord::new(to_pos.x + to_off.0, to_pos.y + to_off.1);

        let blocked: HashSet<(i32, i32)> = grid_occ
            .keys()
            .filter(|&&c| c != (from_attach.x, from_attach.y) && c != (to_attach.x, to_attach.y))
            .copied()
            .collect();

        let is_free = |c: Coord| -> bool { c.x >= 0 && c.y >= 0 && !blocked.contains(&(c.x, c.y)) };
        let pref_path = crate::mermaid::ascii::astar::find_path(
            from_attach,
            to_attach,
            path_bound_x,
            path_bound_y,
            is_free,
        );

        // Try alternative if different.
        let alt_used = !(alt_a == effective_a && alt_b == effective_b);
        let chosen = match (pref_path, alt_used) {
            (Some(p), true) => {
                let alt_from_off = attach_offset(alt_a);
                let alt_to_off = attach_offset(alt_b);
                let alt_from = Coord::new(from_pos.x + alt_from_off.0, from_pos.y + alt_from_off.1);
                let alt_to = Coord::new(to_pos.x + alt_to_off.0, to_pos.y + alt_to_off.1);
                let blocked2: HashSet<(i32, i32)> = grid_occ
                    .keys()
                    .filter(|&&c| c != (alt_from.x, alt_from.y) && c != (alt_to.x, alt_to.y))
                    .copied()
                    .collect();
                let is_free2 = |c: Coord| c.x >= 0 && c.y >= 0 && !blocked2.contains(&(c.x, c.y));
                let alt_path = crate::mermaid::ascii::astar::find_path(
                    alt_from,
                    alt_to,
                    path_bound_x,
                    path_bound_y,
                    is_free2,
                );
                let pref_merged = merge_path(p);
                match alt_path {
                    Some(ap) => {
                        let alt_merged = merge_path(ap);
                        if pref_merged.len() <= alt_merged.len() {
                            Some((pref_merged, effective_a, effective_b))
                        } else {
                            Some((alt_merged, alt_a, alt_b))
                        }
                    }
                    None => Some((pref_merged, effective_a, effective_b)),
                }
            }
            (Some(p), false) => Some((merge_path(p), effective_a, effective_b)),
            (None, true) => {
                let alt_from_off = attach_offset(alt_a);
                let alt_to_off = attach_offset(alt_b);
                let alt_from = Coord::new(from_pos.x + alt_from_off.0, from_pos.y + alt_from_off.1);
                let alt_to = Coord::new(to_pos.x + alt_to_off.0, to_pos.y + alt_to_off.1);
                let blocked2: HashSet<(i32, i32)> = grid_occ
                    .keys()
                    .filter(|&&c| c != (alt_from.x, alt_from.y) && c != (alt_to.x, alt_to.y))
                    .copied()
                    .collect();
                let is_free2 = |c: Coord| c.x >= 0 && c.y >= 0 && !blocked2.contains(&(c.x, c.y));
                let alt_path = crate::mermaid::ascii::astar::find_path(
                    alt_from,
                    alt_to,
                    path_bound_x,
                    path_bound_y,
                    is_free2,
                );
                alt_path.map(|ap| (merge_path(ap), alt_a, alt_b))
            }
            (None, false) => None,
        };

        match chosen {
            Some((path, sa, ea)) => {
                let label_segment = determine_label_segment(
                    &path,
                    edge.label.chars().count(),
                    &mut column_widths,
                    &node_grid,
                );
                edge_routes.push(EdgeRoute {
                    path,
                    start_dir: sa,
                    end_dir: ea,
                    label_segment,
                });
            }
            None => {
                edge_routes.push(EdgeRoute {
                    path: Vec::new(),
                    start_dir: Direction::Middle,
                    end_dir: Direction::Middle,
                    label_segment: None,
                });
            }
        }
        edge_counts.insert(key, dup_index + 1);
    }

    // Increase grid size for path cells (ensure columns/rows for routed cells).
    for er in &edge_routes {
        for c in &er.path {
            column_widths.entry(c.x).or_insert(PADDING_X / 2);
            row_heights.entry(c.y).or_insert(PADDING_Y / 2);
        }
    }

    // Compute canvas dimensions: sum widths + heights.
    let mut max_col: i32 = 0;
    let mut max_row: i32 = 0;
    for &c in column_widths.keys() {
        if c > max_col {
            max_col = c;
        }
    }
    for &r in row_heights.keys() {
        if r > max_row {
            max_row = r;
        }
    }
    let mut canvas_width: usize = 0;
    for c in 0..=max_col {
        canvas_width += column_widths.get(&c).copied().unwrap_or(0);
    }
    let mut canvas_height: usize = 0;
    for r in 0..=max_row {
        canvas_height += row_heights.get(&r).copied().unwrap_or(0);
    }
    canvas_width = canvas_width.max(1);
    canvas_height = canvas_height.max(1);

    // Compute drawing coord for a grid cell (helper).
    let drawing_of = |gx: i32, gy: i32| -> (i32, i32) {
        let mut x = 0i32;
        for c in 0..gx {
            x += column_widths.get(&c).copied().unwrap_or(0) as i32;
        }
        let mut y = 0i32;
        for r in 0..gy {
            y += row_heights.get(&r).copied().unwrap_or(0) as i32;
        }
        let cw = column_widths.get(&gx).copied().unwrap_or(0) as i32;
        let rh = row_heights.get(&gy).copied().unwrap_or(0) as i32;
        (x + cw / 2, y + rh / 2)
    };

    // Subgraph bounding boxes (in drawing coords, may be negative; we offset later).
    // Process inner-first (largest depth first) so outer subgraphs can wrap inner ones.
    let mut sg_depth: HashMap<String, usize> = HashMap::new();
    fn compute_depth(
        id: &str,
        subgraphs: &IndexMap<String, super::ast::Subgraph>,
        memo: &mut HashMap<String, usize>,
    ) -> usize {
        if let Some(&d) = memo.get(id) {
            return d;
        }
        let sg = match subgraphs.get(id) {
            Some(s) => s,
            None => return 0,
        };
        let d = match &sg.parent {
            Some(p) => 1 + compute_depth(p, subgraphs, memo),
            None => 0,
        };
        memo.insert(id.to_string(), d);
        d
    }
    for id in flow.subgraphs.keys() {
        compute_depth(id, &flow.subgraphs, &mut sg_depth);
    }
    let mut sg_order: Vec<String> = flow.subgraphs.keys().cloned().collect();
    sg_order.sort_by(|a, b| sg_depth[b].cmp(&sg_depth[a]));

    let mut raw_boxes: IndexMap<String, (i32, i32, i32, i32)> = IndexMap::new();
    for sg_id in &sg_order {
        let sg = &flow.subgraphs[sg_id];
        if sg.node_ids.is_empty() && sg.child_subgraphs.is_empty() {
            continue;
        }
        let mut min_x: i32 = i32::MAX;
        let mut min_y: i32 = i32::MAX;
        let mut max_x: i32 = i32::MIN;
        let mut max_y: i32 = i32::MIN;
        for child_id in &sg.child_subgraphs {
            if let Some(&(cminx, cminy, cmaxx, cmaxy)) = raw_boxes.get(child_id) {
                min_x = min_x.min(cminx);
                min_y = min_y.min(cminy);
                max_x = max_x.max(cmaxx);
                max_y = max_y.max(cmaxy);
            }
        }
        for nid in &sg.node_ids {
            let pos = match node_grid.get(nid) {
                Some(p) => *p,
                None => continue,
            };
            let (ox, oy) = drawing_of(pos.x, pos.y);
            let w = column_widths.get(&pos.x).copied().unwrap_or(0) as i32
                + column_widths.get(&(pos.x + 1)).copied().unwrap_or(0) as i32;
            let h = row_heights.get(&pos.y).copied().unwrap_or(0) as i32
                + row_heights.get(&(pos.y + 1)).copied().unwrap_or(0) as i32;
            min_x = min_x.min(ox);
            min_y = min_y.min(oy);
            max_x = max_x.max(ox + w);
            max_y = max_y.max(oy + h);
        }
        if min_x == i32::MAX {
            continue;
        }
        let pad = SUBGRAPH_PADDING as i32;
        let title_overhead = if sg.title.is_empty() { 0 } else { 2 };
        raw_boxes.insert(
            sg_id.clone(),
            (
                min_x - pad,
                min_y - pad - title_overhead,
                max_x + pad,
                max_y + pad,
            ),
        );
    }
    // Re-order raw_boxes in flow.subgraphs insertion order for output stability.
    let mut ordered_boxes: IndexMap<String, (i32, i32, i32, i32)> = IndexMap::new();
    for id in flow.subgraphs.keys() {
        if let Some(b) = raw_boxes.get(id) {
            ordered_boxes.insert(id.clone(), *b);
        }
    }
    let raw_boxes = ordered_boxes;

    // Compute global offset so all coordinates are >= 0.
    let mut min_offset_x: i32 = 0;
    let mut min_offset_y: i32 = 0;
    for &(mnx, mny, _, _) in raw_boxes.values() {
        min_offset_x = min_offset_x.min(mnx);
        min_offset_y = min_offset_y.min(mny);
    }
    let offset_x = (-min_offset_x).max(0) as usize;
    let offset_y = (-min_offset_y).max(0) as usize;

    let mut subgraph_boxes: IndexMap<String, SubgraphBox> = IndexMap::new();
    for (id, (mnx, mny, mxx, mxy)) in raw_boxes {
        subgraph_boxes.insert(
            id,
            SubgraphBox {
                min_x: (mnx + offset_x as i32).max(0) as usize,
                min_y: (mny + offset_y as i32).max(0) as usize,
                max_x: (mxx + offset_x as i32).max(0) as usize,
                max_y: (mxy + offset_y as i32).max(0) as usize,
            },
        );
    }

    // Expand canvas to include the global offset.
    canvas_width += offset_x;
    canvas_height += offset_y;

    // Expand canvas to fit subgraph boxes if needed.
    for sb in subgraph_boxes.values() {
        canvas_width = canvas_width.max(sb.max_x + 1);
        canvas_height = canvas_height.max(sb.max_y + 1);
    }

    Ok(Layout {
        node_grid,
        column_widths,
        row_heights,
        edges: edge_routes,
        subgraph_boxes,
        canvas_width,
        canvas_height,
        offset_x,
        offset_y,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mermaid::ascii::flowchart::parser::parse;

    fn lay(src: &str) -> Layout {
        let flow = parse(src).expect("parse");
        layout(&flow).expect("layout")
    }

    #[test]
    fn layout_single_node_minimal() {
        let l = lay("graph LR\nA");
        assert_eq!(l.node_grid.len(), 1);
        let pos = l.node_grid.get("A").unwrap();
        assert_eq!(pos.x, 0);
        assert_eq!(pos.y, 0);
        assert!(l.canvas_width > 0);
        assert!(l.canvas_height > 0);
    }

    #[test]
    fn layout_two_nodes_lr_places_horizontally() {
        let l = lay("graph LR\nA --> B");
        let a = l.node_grid.get("A").unwrap();
        let b = l.node_grid.get("B").unwrap();
        assert!(b.x > a.x);
        assert_eq!(a.y, b.y);
    }

    #[test]
    fn layout_two_nodes_td_places_vertically() {
        let l = lay("graph TD\nA --> B");
        let a = l.node_grid.get("A").unwrap();
        let b = l.node_grid.get("B").unwrap();
        assert!(b.y > a.y);
        assert_eq!(a.x, b.x);
    }

    #[test]
    fn layout_three_node_chain() {
        let l = lay("graph LR\nA --> B\nB --> C");
        let a = l.node_grid.get("A").unwrap();
        let b = l.node_grid.get("B").unwrap();
        let c = l.node_grid.get("C").unwrap();
        assert!(a.x < b.x && b.x < c.x);
    }

    #[test]
    fn layout_branch_one_to_two() {
        let l = lay("graph LR\nA --> B\nA --> C");
        let a = l.node_grid.get("A").unwrap();
        let b = l.node_grid.get("B").unwrap();
        let c = l.node_grid.get("C").unwrap();
        assert!(b.x > a.x);
        assert!(c.x > a.x);
        assert!(b.y != c.y);
    }

    #[test]
    fn layout_diamond_with_cycle_back() {
        let l = lay("graph LR\nA --> B\nB --> C\nC --> A");
        assert!(l.node_grid.contains_key("A"));
        assert!(l.node_grid.contains_key("B"));
        assert!(l.node_grid.contains_key("C"));
        assert_eq!(l.edges.len(), 3);
    }

    #[test]
    fn layout_subgraph_bounding_box_around_nodes() {
        let l = lay("graph LR\nsubgraph S1\n  A --> B\nend");
        assert!(l.subgraph_boxes.contains_key("S1"));
        let bb = &l.subgraph_boxes["S1"];
        assert!(bb.max_x > bb.min_x);
        assert!(bb.max_y > bb.min_y);
    }

    #[test]
    fn layout_column_widths_match_node_label_width() {
        let l = lay("graph LR\nA[Hello]");
        let pos = l.node_grid.get("A").unwrap();
        let mid_col = l.column_widths.get(&(pos.x + 1)).copied().unwrap_or(0);
        assert!(mid_col >= "Hello".len());
    }

    #[test]
    fn layout_row_heights_match_label_height() {
        let l = lay("graph LR\nA");
        let pos = l.node_grid.get("A").unwrap();
        let mid_row = l.row_heights.get(&(pos.y + 1)).copied().unwrap_or(0);
        assert!(mid_row >= 1);
    }

    #[test]
    fn layout_padding_between_siblings() {
        let l = lay("graph LR\nA --> B\nA --> C");
        let b = l.node_grid.get("B").unwrap();
        let c = l.node_grid.get("C").unwrap();
        let diff = (b.y - c.y).abs();
        assert!(diff >= 4);
    }

    #[test]
    fn layout_multi_root_graph() {
        let l = lay("graph LR\nA --> B\nC --> D");
        let a = l.node_grid.get("A").unwrap();
        let c = l.node_grid.get("C").unwrap();
        assert_eq!(a.x, 0);
        assert_eq!(c.x, 0);
        assert!(a.y != c.y);
    }

    #[test]
    fn layout_self_loop_routes() {
        let l = lay("graph LR\nA --> A");
        assert_eq!(l.edges.len(), 1);
        let _ = &l.edges[0];
    }

    #[test]
    fn layout_label_segment_chosen_on_widest_corridor() {
        let l = lay("graph LR\nA -->|hello world| B");
        let route = &l.edges[0];
        assert!(route.label_segment.is_some());
    }

    #[test]
    fn layout_disconnected_components() {
        let l = lay("graph LR\nA\nB");
        assert_eq!(l.node_grid.len(), 2);
        let a = l.node_grid.get("A").unwrap();
        let b = l.node_grid.get("B").unwrap();
        assert!(a != b);
    }

    #[test]
    fn layout_empty_flowchart_no_nodes() {
        let flow = Flowchart::default();
        let l = layout(&flow).expect("ok");
        assert!(l.node_grid.is_empty());
    }

    #[test]
    fn layout_node_with_multiline_label_grows_row() {
        let l = lay("graph LR\nA[line1<br>line2]");
        let pos = l.node_grid.get("A").unwrap();
        let mid_row = l.row_heights.get(&(pos.y + 1)).copied().unwrap_or(0);
        assert!(mid_row >= 2);
    }

    #[test]
    fn layout_returns_canvas_dimensions() {
        let l = lay("graph LR\nA --> B");
        assert!(l.canvas_width >= 4);
        assert!(l.canvas_height >= 3);
    }
}
