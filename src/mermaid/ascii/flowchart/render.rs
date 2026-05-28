use super::ast::{EdgeArrow, EdgeStyle, Flowchart, NodeShape};
use super::layout::{EdgeRoute, Layout};
use crate::mermaid::ascii::canvas::Canvas;
use crate::mermaid::ascii::charset::get_charset;
use crate::mermaid::ascii::coord::Direction;
use crate::mermaid::ascii::{AsciiRenderError, CharsetKind, StyledRow};
use ratatui::style::Color;
use std::collections::HashMap;

pub fn render(
    flow: &Flowchart,
    layout: &Layout,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    render_to_canvas(flow, layout, max_width, charset).map(|c| c.into_lines())
}

pub fn render_styled(
    flow: &Flowchart,
    layout: &Layout,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    render_to_canvas(flow, layout, max_width, charset).map(|c| c.into_styled_lines())
}

fn render_to_canvas(
    flow: &Flowchart,
    layout: &Layout,
    _max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    if flow.nodes.is_empty() {
        return Ok(Canvas::new(1, 1, charset));
    }

    let width = layout.canvas_width.max(1);
    let height = layout.canvas_height.max(1);
    let mut canvas = Canvas::new(width, height, charset);

    let cs = get_charset(charset);

    // Precompute prefix sums for grid → drawing conversion.
    let max_col: i32 = layout.column_widths.keys().copied().max().unwrap_or(0);
    let max_row: i32 = layout.row_heights.keys().copied().max().unwrap_or(0);

    let off_x = layout.offset_x as i32;
    let off_y = layout.offset_y as i32;
    let grid_to_drawing = |gx: i32, gy: i32| -> (i32, i32) {
        let mut x: i32 = 0;
        for c in 0..gx {
            x += layout.column_widths.get(&c).copied().unwrap_or(0) as i32;
        }
        let mut y: i32 = 0;
        for r in 0..gy {
            y += layout.row_heights.get(&r).copied().unwrap_or(0) as i32;
        }
        let cw = layout.column_widths.get(&gx).copied().unwrap_or(0) as i32;
        let rh = layout.row_heights.get(&gy).copied().unwrap_or(0) as i32;
        (x + cw / 2 + off_x, y + rh / 2 + off_y)
    };

    // Draw subgraphs first (background). Outer-first so inner overlays.
    let mut sg_draw_order: Vec<String> = layout.subgraph_boxes.keys().cloned().collect();
    sg_draw_order.sort_by(|a, b| {
        // Outer subgraphs have larger area; draw them first.
        let ba = &layout.subgraph_boxes[a];
        let bb = &layout.subgraph_boxes[b];
        let area_a = (ba.max_x - ba.min_x) * (ba.max_y - ba.min_y);
        let area_b = (bb.max_x - bb.min_x) * (bb.max_y - bb.min_y);
        area_b.cmp(&area_a)
    });
    for id in &sg_draw_order {
        let sg = &layout.subgraph_boxes[id];
        let w = sg.max_x.saturating_sub(sg.min_x);
        let h = sg.max_y.saturating_sub(sg.min_y);
        if w == 0 || h == 0 {
            continue;
        }
        canvas.draw_box(sg.min_x, sg.min_y, sg.max_x, sg.max_y);
        canvas.paint_box(sg.min_x, sg.min_y, sg.max_x, sg.max_y, Color::Blue);
    }

    // Draw nodes.
    let mut node_box: HashMap<String, (usize, usize, usize, usize)> = HashMap::new();
    for (id, pos) in &layout.node_grid {
        let (ox, oy) = grid_to_drawing(pos.x, pos.y);
        let w = layout.column_widths.get(&pos.x).copied().unwrap_or(1)
            + layout.column_widths.get(&(pos.x + 1)).copied().unwrap_or(1);
        let h = layout.row_heights.get(&pos.y).copied().unwrap_or(1)
            + layout.row_heights.get(&(pos.y + 1)).copied().unwrap_or(1);
        let x1 = ox.max(0) as usize;
        let y1 = oy.max(0) as usize;
        let x2 = x1 + w;
        let y2 = y1 + h;
        let node = &flow.nodes[id];
        draw_node_shape(&mut canvas, charset, &node.shape, x1, y1, x2, y2);
        canvas.paint_box(x1, y1, x2, y2, Color::Cyan);
        // Label inside box.
        let label_text = if node.label.lines.is_empty() {
            vec![node.id.clone()]
        } else {
            node.label.lines.clone()
        };
        let inner_w = (x2 - x1).saturating_sub(1);
        let inner_h = (y2 - y1).saturating_sub(1);
        let content_h = label_text.len().max(1);
        let top_y = y1 + 1 + (inner_h.saturating_sub(content_h)) / 2;
        for (i, line) in label_text.iter().enumerate() {
            let line_w = unicode_width::UnicodeWidthStr::width(line.as_str());
            let line_x = x1 + 1 + (inner_w.saturating_sub(line_w)) / 2;
            let line_y = top_y + i;
            if line_y < y2 {
                canvas.put_str(line_x, line_y, line);
            }
        }
        node_box.insert(id.clone(), (x1, y1, x2, y2));
    }

    // Draw edges.
    for (i, edge) in flow.edges.iter().enumerate() {
        let route = &layout.edges[i];
        if route.path.is_empty() {
            continue;
        }
        draw_edge(&mut canvas, charset, route, edge, &grid_to_drawing);
    }

    // Draw subgraph titles centered on top border.
    for (id, sg) in &layout.subgraph_boxes {
        let title = flow
            .subgraphs
            .get(id)
            .map(|s| s.title.as_str())
            .unwrap_or("");
        if title.is_empty() {
            continue;
        }
        let w = sg.max_x.saturating_sub(sg.min_x);
        let tw = unicode_width::UnicodeWidthStr::width(title);
        if w <= 2 {
            continue;
        }
        let tx = sg.min_x + 1 + (w.saturating_sub(tw)) / 2;
        if sg.min_y + 1 < height {
            // Clear a bit of the border below the title.
            canvas.put_str(tx, sg.min_y, title);
            canvas.paint_text(tx, sg.min_y, title, Color::Blue);
        }
    }

    // Suppress unused warnings:
    let _ = cs;
    let _ = max_col;
    let _ = max_row;

    Ok(canvas)
}

fn draw_node_shape(
    canvas: &mut Canvas,
    charset: CharsetKind,
    shape: &NodeShape,
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
) {
    let cs = get_charset(charset);
    canvas.draw_box(x1, y1, x2, y2);
    match shape {
        NodeShape::Square | NodeShape::Subroutine => {}
        NodeShape::Round | NodeShape::Stadium => {
            canvas.put_char(x1, y1, cs.round_top_left());
            canvas.put_char(x2, y1, cs.round_top_right());
            canvas.put_char(x1, y2, cs.round_bot_left());
            canvas.put_char(x2, y2, cs.round_bot_right());
        }
        NodeShape::Circle | NodeShape::DoubleCircle => {
            canvas.put_char(x1, y1, cs.round_top_left());
            canvas.put_char(x2, y1, cs.round_top_right());
            canvas.put_char(x1, y2, cs.round_bot_left());
            canvas.put_char(x2, y2, cs.round_bot_right());
        }
        NodeShape::Cylinder => {
            canvas.put_char(x1, y1, cs.round_top_left());
            canvas.put_char(x2, y1, cs.round_top_right());
            canvas.put_char(x1, y2, cs.round_bot_left());
            canvas.put_char(x2, y2, cs.round_bot_right());
        }
        NodeShape::Rhombus
        | NodeShape::Hexagon
        | NodeShape::Parallelogram
        | NodeShape::ParallelogramAlt
        | NodeShape::Trapezoid
        | NodeShape::TrapezoidAlt
        | NodeShape::Asymmetric => {
            // Use square with corner hints: replace corners with diagonals/angles.
            canvas.put_char(x1, y1, cs.diag_up());
            canvas.put_char(x2, y1, cs.diag_down());
            canvas.put_char(x1, y2, cs.diag_down());
            canvas.put_char(x2, y2, cs.diag_up());
        }
    }
}

fn attach_offset_to(dir: Direction) -> (i32, i32) {
    match dir {
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

fn draw_edge<F: Fn(i32, i32) -> (i32, i32)>(
    canvas: &mut Canvas,
    charset: CharsetKind,
    route: &EdgeRoute,
    edge: &super::ast::Edge,
    grid_to_drawing: &F,
) {
    let cs = get_charset(charset);
    if route.path.len() < 2 {
        return;
    }

    // Path styling.
    let line_char_h = match edge.style {
        EdgeStyle::Solid | EdgeStyle::Thick => cs.horiz(),
        EdgeStyle::Dotted => '·',
        EdgeStyle::Invisible => ' ',
    };
    let line_char_v = match edge.style {
        EdgeStyle::Solid | EdgeStyle::Thick => cs.vert(),
        EdgeStyle::Dotted => '·',
        EdgeStyle::Invisible => ' ',
    };
    if matches!(edge.style, EdgeStyle::Invisible) {
        return;
    }

    let draw_points: Vec<(i32, i32)> = route
        .path
        .iter()
        .map(|c| grid_to_drawing(c.x, c.y))
        .collect();

    // Track each segment's points and direction.
    let mut segments_dir: Vec<Direction> = Vec::new();
    for w in route.path.windows(2) {
        segments_dir.push(Direction::from_to(w[0], w[1]));
    }

    // Draw line segments.
    for (i, w) in draw_points.windows(2).enumerate() {
        let from = w[0];
        let to = w[1];
        let dir = segments_dir[i];
        match dir {
            Direction::Right => {
                let lo = (from.0 + 1).max(0) as usize;
                let hi = to.0.max(0) as usize;
                if lo <= hi {
                    for x in lo..=hi {
                        place_horiz(canvas, x, from.1.max(0) as usize, line_char_h);
                    }
                    canvas.paint_hline(lo, hi, from.1.max(0) as usize, Color::Yellow);
                }
            }
            Direction::Left => {
                let lo = to.0.max(0) as usize;
                let hi = (from.0 - 1).max(0) as usize;
                if lo <= hi {
                    for x in lo..=hi {
                        place_horiz(canvas, x, from.1.max(0) as usize, line_char_h);
                    }
                    canvas.paint_hline(lo, hi, from.1.max(0) as usize, Color::Yellow);
                }
            }
            Direction::Down => {
                let lo = (from.1 + 1).max(0) as usize;
                let hi = to.1.max(0) as usize;
                if lo <= hi {
                    for y in lo..=hi {
                        place_vert(canvas, from.0.max(0) as usize, y, line_char_v);
                    }
                    canvas.paint_vline(from.0.max(0) as usize, lo, hi, Color::Yellow);
                }
            }
            Direction::Up => {
                let lo = to.1.max(0) as usize;
                let hi = (from.1 - 1).max(0) as usize;
                if lo <= hi {
                    for y in lo..=hi {
                        place_vert(canvas, from.0.max(0) as usize, y, line_char_v);
                    }
                    canvas.paint_vline(from.0.max(0) as usize, lo, hi, Color::Yellow);
                }
            }
            _ => {}
        }
    }

    // Draw corners.
    if route.path.len() > 2 {
        for i in 1..route.path.len() - 1 {
            let prev_dir = segments_dir[i - 1];
            let next_dir = segments_dir[i];
            let (cx, cy) = draw_points[i];
            let corner = match (prev_dir, next_dir) {
                (Direction::Right, Direction::Down) | (Direction::Up, Direction::Left) => {
                    cs.top_right()
                }
                (Direction::Right, Direction::Up) | (Direction::Down, Direction::Left) => {
                    cs.bot_right()
                }
                (Direction::Left, Direction::Down) | (Direction::Up, Direction::Right) => {
                    cs.top_left()
                }
                (Direction::Left, Direction::Up) | (Direction::Down, Direction::Right) => {
                    cs.bot_left()
                }
                _ => cs.cross(),
            };
            if cx >= 0 && cy >= 0 {
                let ux = cx as usize;
                let uy = cy as usize;
                let cur = canvas.get(ux, uy);
                let prev_line_char = match prev_dir {
                    Direction::Up | Direction::Down => line_char_v,
                    Direction::Left | Direction::Right => line_char_h,
                    _ => corner,
                };
                let final_char = if matches!(charset, CharsetKind::Unicode)
                    && crate::mermaid::ascii::canvas::is_junction_char(cur)
                    && crate::mermaid::ascii::canvas::is_junction_char(corner)
                    && cur != prev_line_char
                {
                    crate::mermaid::ascii::canvas::merge_junctions(cur, corner)
                } else {
                    corner
                };
                canvas.put_char(ux, uy, final_char);
                canvas.set_color(ux, uy, Color::Yellow);
            }
        }
    }

    // Box-start char: on the node border where the path begins.
    if route.path.len() >= 2 {
        let first_dir = segments_dir[0];
        let (sx, sy) = draw_points[0];
        let unicode = matches!(charset, CharsetKind::Unicode);
        if unicode {
            match first_dir {
                Direction::Up => {
                    if sy >= 0 {
                        canvas.put_char(sx.max(0) as usize, sy.max(0) as usize, cs.tee_up());
                        canvas.set_color(sx.max(0) as usize, sy.max(0) as usize, Color::Cyan);
                    }
                }
                Direction::Down => {
                    canvas.put_char(sx.max(0) as usize, sy.max(0) as usize, cs.tee_down());
                    canvas.set_color(sx.max(0) as usize, sy.max(0) as usize, Color::Cyan);
                }
                Direction::Left => {
                    canvas.put_char(sx.max(0) as usize, sy.max(0) as usize, cs.tee_left());
                    canvas.set_color(sx.max(0) as usize, sy.max(0) as usize, Color::Cyan);
                }
                Direction::Right => {
                    canvas.put_char(sx.max(0) as usize, sy.max(0) as usize, cs.tee_right());
                    canvas.set_color(sx.max(0) as usize, sy.max(0) as usize, Color::Cyan);
                }
                _ => {}
            }
        }
    }

    // Arrowhead at end.
    let end_dir = *segments_dir.last().unwrap();
    let (ex, ey) = *draw_points.last().unwrap();
    if ex >= 0 && ey >= 0 {
        let head = match (edge.arrow, end_dir) {
            (EdgeArrow::Bidirectional, _) | (EdgeArrow::Open, _) | (EdgeArrow::Closed, _) => {
                match end_dir {
                    Direction::Up => cs.arrow_up(),
                    Direction::Down => cs.arrow_down(),
                    Direction::Left => cs.arrow_left(),
                    Direction::Right => cs.arrow_right(),
                    _ => cs.arrow_right(),
                }
            }
            (EdgeArrow::Circle, _) => 'o',
            (EdgeArrow::Cross, _) => 'x',
        };
        canvas.put_char(ex as usize, ey as usize, head);
        canvas.set_color(ex as usize, ey as usize, Color::Magenta);
    }

    // Bidirectional: arrow on start as well.
    if matches!(edge.arrow, EdgeArrow::Bidirectional) {
        let (sx, sy) = draw_points[0];
        if sx >= 0 && sy >= 0 {
            let head = match segments_dir[0] {
                Direction::Up => cs.arrow_down(),
                Direction::Down => cs.arrow_up(),
                Direction::Left => cs.arrow_right(),
                Direction::Right => cs.arrow_left(),
                _ => cs.arrow_left(),
            };
            canvas.put_char(sx as usize, sy as usize, head);
            canvas.set_color(sx as usize, sy as usize, Color::Magenta);
        }
    }

    // Label.
    if !edge.label.is_empty() {
        if let Some((a, b)) = route.label_segment {
            let pa = grid_to_drawing(a.x, a.y);
            let pb = grid_to_drawing(b.x, b.y);
            let mid_x = (pa.0 + pb.0) / 2;
            let mid_y = (pa.1 + pb.1) / 2;
            let label_w = unicode_width::UnicodeWidthStr::width(edge.label.as_str()) as i32;
            let start_x = mid_x - label_w / 2;
            let label_y = mid_y;
            if label_y >= 0 {
                let mut x = start_x.max(0) as usize;
                for ch in edge.label.chars() {
                    canvas.put_char(x, label_y as usize, ch);
                    canvas.set_color(x, label_y as usize, Color::Green);
                    let w = unicode_width::UnicodeWidthChar::width(ch)
                        .unwrap_or(1)
                        .max(1);
                    x += w;
                }
            }
        }
    }

    // Suppress unused warning.
    let _ = attach_offset_to;
}

fn place_horiz(canvas: &mut Canvas, x: usize, y: usize, c: char) {
    let cur = canvas.get(x, y);
    if cur == ' ' {
        canvas.put_char(x, y, c);
        return;
    }
    if crate::mermaid::ascii::canvas::is_junction_char(cur)
        && crate::mermaid::ascii::canvas::is_junction_char(c)
    {
        canvas.put_char(x, y, crate::mermaid::ascii::canvas::merge_junctions(cur, c));
    } else {
        canvas.put_char(x, y, c);
    }
}

fn place_vert(canvas: &mut Canvas, x: usize, y: usize, c: char) {
    let cur = canvas.get(x, y);
    if cur == ' ' {
        canvas.put_char(x, y, c);
        return;
    }
    if crate::mermaid::ascii::canvas::is_junction_char(cur)
        && crate::mermaid::ascii::canvas::is_junction_char(c)
    {
        canvas.put_char(x, y, crate::mermaid::ascii::canvas::merge_junctions(cur, c));
    } else {
        canvas.put_char(x, y, c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_src(src: &str) -> String {
        crate::mermaid::ascii::flowchart::render(src, 200, CharsetKind::Unicode)
            .unwrap()
            .join("\n")
    }

    fn render_ascii(src: &str) -> String {
        crate::mermaid::ascii::flowchart::render(src, 200, CharsetKind::Ascii)
            .unwrap()
            .join("\n")
    }

    #[test]
    fn render_two_node_lr_unicode() {
        insta::assert_snapshot!(render_src("graph LR\nA --> B"));
    }

    #[test]
    fn render_two_node_td_unicode() {
        insta::assert_snapshot!(render_src("graph TD\nA --> B"));
    }

    #[test]
    fn render_three_node_chain_lr() {
        insta::assert_snapshot!(render_src("graph LR\nA --> B\nB --> C"));
    }

    #[test]
    fn render_branch_lr() {
        insta::assert_snapshot!(render_src("graph LR\nA --> B\nA --> C"));
    }

    #[test]
    fn render_diamond_decision_with_labels() {
        insta::assert_snapshot!(render_src(
            "graph LR\nA{Decide} -->|yes| B\nA -->|no| C\nB --> D\nC --> D"
        ));
    }

    #[test]
    fn render_subgraph_simple() {
        insta::assert_snapshot!(render_src("graph LR\nsubgraph S1[Group]\n  A --> B\nend"));
    }

    #[test]
    fn render_subgraph_nested() {
        insta::assert_snapshot!(render_src(
            "graph LR\nsubgraph S1[Outer]\n  subgraph S2[Inner]\n    A --> B\n  end\nend"
        ));
    }

    #[test]
    fn render_self_loop() {
        insta::assert_snapshot!(render_src("graph LR\nA --> A"));
    }

    #[test]
    fn render_round_node_shape() {
        insta::assert_snapshot!(render_src("graph LR\nA(Round) --> B"));
    }

    #[test]
    fn render_rhombus_node_shape() {
        insta::assert_snapshot!(render_src("graph LR\nA{Decide} --> B"));
    }

    #[test]
    fn render_stadium_node_shape() {
        insta::assert_snapshot!(render_src("graph LR\nA([Stadium]) --> B"));
    }

    #[test]
    fn render_circle_node_shape() {
        insta::assert_snapshot!(render_src("graph LR\nA((Circle)) --> B"));
    }

    #[test]
    fn render_hexagon_node_shape() {
        insta::assert_snapshot!(render_src("graph LR\nA{{Hex}} --> B"));
    }

    #[test]
    fn render_cylinder_node_shape() {
        insta::assert_snapshot!(render_src("graph LR\nA[(Cyl)] --> B"));
    }

    #[test]
    fn render_chain_with_ampersand() {
        insta::assert_snapshot!(render_src("graph LR\nA & B --> C"));
    }

    #[test]
    fn render_ascii_charset_for_two_nodes() {
        insta::assert_snapshot!(render_ascii("graph LR\nA --> B"));
    }

    #[test]
    fn render_bidirectional_edge_lr() {
        insta::assert_snapshot!(render_src("graph LR\nA <--> B"));
    }

    #[test]
    fn render_dotted_edge_lr() {
        insta::assert_snapshot!(render_src("graph LR\nA -.-> B"));
    }

    #[test]
    fn render_thick_edge_lr() {
        insta::assert_snapshot!(render_src("graph LR\nA ==> B"));
    }

    #[test]
    fn render_node_with_multiline_label() {
        insta::assert_snapshot!(render_src("graph LR\nA[line1<br>line2] --> B"));
    }

    #[test]
    fn render_empty_label_edge() {
        insta::assert_snapshot!(render_src("graph LR\nA --> B"));
    }

    #[test]
    fn render_long_label_node() {
        insta::assert_snapshot!(render_src("graph LR\nA[A very long label here] --> B"));
    }

    #[test]
    fn render_two_independent_components() {
        insta::assert_snapshot!(render_src("graph LR\nA --> B\nC --> D"));
    }

    #[test]
    fn render_classdef_assignment_does_not_break_render() {
        insta::assert_snapshot!(render_src(
            "graph LR\nclassDef important fill:red\nA:::important --> B"
        ));
    }

    #[test]
    fn render_lr_then_td_consistency_simple() {
        let lr = render_src("graph LR\nA --> B");
        let td = render_src("graph TD\nA --> B");
        assert_ne!(lr, td);
        insta::assert_snapshot!(format!("LR:\n{}\n---\nTD:\n{}", lr, td));
    }

    // Non-snapshot unit tests.

    #[test]
    fn render_returns_at_least_one_line_for_two_nodes() {
        let lines = crate::mermaid::ascii::flowchart::render(
            "graph LR\nA --> B",
            200,
            CharsetKind::Unicode,
        )
        .unwrap();
        assert!(!lines.is_empty());
    }

    #[test]
    fn render_each_line_is_at_most_max_width_when_constrained_high() {
        let lines = crate::mermaid::ascii::flowchart::render(
            "graph LR\nA --> B",
            200,
            CharsetKind::Unicode,
        )
        .unwrap();
        for l in &lines {
            assert!(unicode_width::UnicodeWidthStr::width(l.as_str()) <= 200);
        }
    }

    #[test]
    fn render_ascii_uses_no_unicode_box_chars() {
        let lines =
            crate::mermaid::ascii::flowchart::render("graph LR\nA --> B", 200, CharsetKind::Ascii)
                .unwrap();
        for l in &lines {
            for c in l.chars() {
                assert!(
                    !crate::mermaid::ascii::canvas::is_junction_char(c),
                    "found unicode box char {c} in ASCII output: {l}"
                );
            }
        }
    }

    #[test]
    fn render_unicode_uses_box_chars() {
        let lines = crate::mermaid::ascii::flowchart::render(
            "graph LR\nA --> B",
            200,
            CharsetKind::Unicode,
        )
        .unwrap();
        let joined: String = lines.join("\n");
        assert!(joined.contains('─') || joined.contains('│') || joined.contains('┌'));
    }

    #[test]
    fn render_handles_unknown_shape_gracefully() {
        // Round shape is harmless; verify the renderer doesn't panic on every shape kind.
        let lines = crate::mermaid::ascii::flowchart::render(
            "graph LR\nA[/Trapezoid/] --> B",
            200,
            CharsetKind::Unicode,
        )
        .unwrap();
        assert!(!lines.is_empty());
    }

    #[test]
    fn render_styled_attaches_color_to_borders() {
        let rows = crate::mermaid::ascii::flowchart::render_styled(
            "graph LR\nA --> B",
            80,
            CharsetKind::Unicode,
        )
        .unwrap();
        assert!(
            rows.iter()
                .flatten()
                .any(|r| r.color == Some(ratatui::style::Color::Cyan)),
            "expected a Cyan border run, got {rows:?}"
        );
    }

    #[test]
    fn render_styled_arrowhead_is_magenta() {
        let rows = crate::mermaid::ascii::flowchart::render_styled(
            "graph LR\nA --> B",
            80,
            CharsetKind::Unicode,
        )
        .unwrap();
        assert!(
            rows.iter()
                .flatten()
                .any(|r| r.color == Some(ratatui::style::Color::Magenta)),
            "expected a Magenta arrowhead run, got {rows:?}"
        );
    }

    #[test]
    fn render_styled_label_uncolored() {
        let rows = crate::mermaid::ascii::flowchart::render_styled(
            "graph LR\nA[Start] --> B[End]",
            80,
            CharsetKind::Unicode,
        )
        .unwrap();
        let plain_has_label = rows
            .iter()
            .flatten()
            .any(|r| r.color.is_none() && (r.text.contains("Start") || r.text.contains("End")));
        assert!(
            plain_has_label,
            "expected an uncolored run containing label chars, got {rows:?}"
        );
    }

    #[test]
    fn styled_output_for_simple_flowchart_has_at_least_three_colors() {
        use std::collections::HashSet;
        let rows = crate::mermaid::ascii::flowchart::render_styled(
            "graph LR\n    A[Start] --> B{Q?}\n    B -->|yes| C[End]",
            80,
            CharsetKind::Unicode,
        )
        .unwrap();
        let colors: HashSet<Option<ratatui::style::Color>> =
            rows.iter().flatten().map(|r| r.color).collect();
        assert!(
            colors.len() >= 3,
            "expected at least 3 distinct colors, got {colors:?}"
        );
    }
}
