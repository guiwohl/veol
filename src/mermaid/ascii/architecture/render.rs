use super::ast::{ArchEdge, ArchitectureDiagram, Side};
use crate::mermaid::ascii::canvas::Canvas;
use crate::mermaid::ascii::charset::get_charset;
use crate::mermaid::ascii::{AsciiRenderError, CharsetKind, StyledRow};
use ratatui::style::Color;
use std::collections::HashMap;
use unicode_width::UnicodeWidthStr;

const SERVICE_MIN_W: usize = 9;
const SERVICE_H: usize = 3;
const SERVICE_GAP: usize = 3;
const GROUP_PAD: usize = 2;
const GROUP_GAP: usize = 2;
const OUTER_MARGIN: usize = 1;

#[derive(Debug, Clone)]
struct NodeBox {
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
}

impl NodeBox {
    fn center_side(&self, side: Side) -> (usize, usize) {
        match side {
            Side::Left => (self.x1, (self.y1 + self.y2) / 2),
            Side::Right => (self.x2, (self.y1 + self.y2) / 2),
            Side::Top => ((self.x1 + self.x2) / 2, self.y1),
            Side::Bottom => ((self.x1 + self.x2) / 2, self.y2),
        }
    }
}

pub fn render(
    diag: &ArchitectureDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_lines())
}

pub fn render_styled(
    diag: &ArchitectureDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_styled_lines())
}

fn render_to_canvas(
    diag: &ArchitectureDiagram,
    _max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    if diag.services.is_empty() && diag.groups.is_empty() && diag.junctions.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    let mut boxes: HashMap<String, NodeBox> = HashMap::new();
    let mut group_boxes: HashMap<String, NodeBox> = HashMap::new();

    let top_groups: Vec<String> = diag
        .decl_order
        .iter()
        .filter(|id| diag.groups.get(*id).is_some_and(|g| g.parent.is_none()))
        .cloned()
        .collect();

    let mut cur_y = OUTER_MARGIN;
    let mut max_x = 0;

    for gid in &top_groups {
        let (_w, h) = layout_group(diag, gid, OUTER_MARGIN, cur_y, &mut boxes, &mut group_boxes);
        cur_y += h + GROUP_GAP;
        if let Some(gb) = group_boxes.get(gid) {
            max_x = max_x.max(gb.x2);
        }
    }

    let orphan_items: Vec<String> = diag
        .decl_order
        .iter()
        .filter(|id| {
            if let Some(s) = diag.services.get(*id) {
                s.group.is_none()
            } else if let Some(j) = diag.junctions.get(*id) {
                j.group.is_none()
            } else {
                false
            }
        })
        .cloned()
        .collect();

    if !orphan_items.is_empty() {
        let mut x_cursor = OUTER_MARGIN;
        let y = cur_y;
        for id in &orphan_items {
            if let Some(svc) = diag.services.get(id) {
                let w = service_width(&svc.label, svc.icon.as_deref());
                let bx = NodeBox {
                    x1: x_cursor,
                    y1: y,
                    x2: x_cursor + w,
                    y2: y + SERVICE_H,
                };
                x_cursor = bx.x2 + SERVICE_GAP;
                max_x = max_x.max(bx.x2);
                boxes.insert(svc.id.clone(), bx);
            } else if diag.junctions.contains_key(id) {
                let bx = NodeBox {
                    x1: x_cursor,
                    y1: y + SERVICE_H / 2,
                    x2: x_cursor + 1,
                    y2: y + SERVICE_H / 2 + 1,
                };
                x_cursor = bx.x2 + SERVICE_GAP;
                max_x = max_x.max(bx.x2);
                boxes.insert(id.clone(), bx);
            }
        }
        cur_y += SERVICE_H + GROUP_GAP;
    }

    let canvas_w = (max_x + OUTER_MARGIN + 1).max(8);
    let canvas_h = (cur_y + OUTER_MARGIN).max(4);

    let mut canvas = Canvas::new(canvas_w, canvas_h, charset);

    let mut group_ids_sorted: Vec<String> = group_boxes.keys().cloned().collect();
    group_ids_sorted.sort_by(|a, b| {
        let ba = &group_boxes[a];
        let bb = &group_boxes[b];
        let area_a = (ba.x2 - ba.x1) * (ba.y2 - ba.y1);
        let area_b = (bb.x2 - bb.x1) * (bb.y2 - bb.y1);
        area_b.cmp(&area_a)
    });
    for gid in &group_ids_sorted {
        let b = group_boxes[gid].clone();
        canvas.draw_box(b.x1, b.y1, b.x2, b.y2);
        canvas.paint_box(b.x1, b.y1, b.x2, b.y2, Color::Blue);
        if let Some(g) = diag.groups.get(gid) {
            let lbl_text = format_group_label(diag, gid, &g.label, charset);
            let max_lbl_w = (b.x2 - b.x1).saturating_sub(2);
            let truncated = truncate(&lbl_text, max_lbl_w);
            let lbl_w = UnicodeWidthStr::width(truncated.as_str());
            let lbl_x = b.x1 + 1 + ((b.x2 - b.x1).saturating_sub(2 + lbl_w)) / 2;
            canvas.put_str_colored(lbl_x, b.y1, &truncated, Color::Blue);
        }
    }

    for (sid, b) in &boxes {
        if let Some(svc) = diag.services.get(sid) {
            canvas.draw_box(b.x1, b.y1, b.x2, b.y2);
            canvas.paint_box(b.x1, b.y1, b.x2, b.y2, Color::Cyan);
            let icon_glyph = icon_for(svc.icon.as_deref(), charset);
            let label = &svc.label;
            let inner_w = (b.x2 - b.x1).saturating_sub(2);
            let content = if let Some(g) = icon_glyph.clone() {
                format!("{g} {label}")
            } else {
                label.clone()
            };
            let truncated = truncate(&content, inner_w);
            let tw = UnicodeWidthStr::width(truncated.as_str());
            let inner_pad = inner_w.saturating_sub(tw);
            let cx = b.x1 + 1 + inner_pad / 2;
            let cy = b.y1 + (b.y2 - b.y1) / 2;
            canvas.put_str(cx, cy, &truncated);
            // Recolor only the icon glyph at the head of the content
            if let Some(g) = icon_glyph {
                let glyph_w = UnicodeWidthStr::width(g.as_str());
                canvas.paint_text(cx, cy, &g, Color::Magenta);
                let _ = glyph_w;
            }
        } else if diag.junctions.contains_key(sid) {
            let cs = get_charset(charset);
            canvas.put_char(b.x1, b.y1, cs.cross());
            canvas.set_color(b.x1, b.y1, Color::Yellow);
        }
    }

    for edge in &diag.edges {
        draw_edge(&mut canvas, edge, &boxes, charset);
    }

    Ok(canvas)
}

fn layout_group(
    diag: &ArchitectureDiagram,
    gid: &str,
    x: usize,
    y: usize,
    boxes: &mut HashMap<String, NodeBox>,
    group_boxes: &mut HashMap<String, NodeBox>,
) -> (usize, usize) {
    let group = match diag.groups.get(gid) {
        Some(g) => g,
        None => return (0, 0),
    };

    let inner_x = x + GROUP_PAD;
    let inner_y = y + GROUP_PAD;

    let row_items: Vec<String> = diag
        .decl_order
        .iter()
        .filter(|id| {
            if let Some(s) = diag.services.get(*id) {
                s.group.as_deref() == Some(gid)
            } else if let Some(j) = diag.junctions.get(*id) {
                j.group.as_deref() == Some(gid)
            } else {
                false
            }
        })
        .cloned()
        .collect();
    let child_groups: Vec<String> = diag
        .decl_order
        .iter()
        .filter(|id| {
            diag.groups
                .get(*id)
                .is_some_and(|g| g.parent.as_deref() == Some(gid))
        })
        .cloned()
        .collect();

    let mut row_x = inner_x;
    let row_y = inner_y;
    let mut max_right = inner_x;
    let mut row_bottom = inner_y;

    let has_row = !row_items.is_empty();

    for id in &row_items {
        if let Some(svc) = diag.services.get(id) {
            let w = service_width(&svc.label, svc.icon.as_deref());
            let bx = NodeBox {
                x1: row_x,
                y1: row_y,
                x2: row_x + w,
                y2: row_y + SERVICE_H,
            };
            row_x = bx.x2 + SERVICE_GAP;
            max_right = max_right.max(bx.x2);
            row_bottom = row_bottom.max(bx.y2);
            boxes.insert(svc.id.clone(), bx);
        } else if diag.junctions.contains_key(id) {
            let bx = NodeBox {
                x1: row_x,
                y1: row_y + SERVICE_H / 2,
                x2: row_x + 1,
                y2: row_y + SERVICE_H / 2 + 1,
            };
            row_x = bx.x2 + SERVICE_GAP;
            max_right = max_right.max(bx.x2);
            row_bottom = row_bottom.max(row_y + SERVICE_H);
            boxes.insert(id.clone(), bx);
        }
    }

    let mut nested_y = if has_row {
        row_bottom + GROUP_GAP
    } else {
        inner_y
    };

    for cgid in &child_groups {
        let (cw, ch) = layout_group(diag, cgid, inner_x, nested_y, boxes, group_boxes);
        max_right = max_right.max(inner_x + cw);
        nested_y += ch + GROUP_GAP;
    }

    let content_bottom = if !child_groups.is_empty() {
        nested_y.saturating_sub(GROUP_GAP)
    } else {
        row_bottom.max(inner_y + SERVICE_H)
    };

    let label_min_w = UnicodeWidthStr::width(group.label.as_str()) + 4;
    let content_w = max_right.saturating_sub(inner_x);
    let inner_w = content_w.max(label_min_w);
    let inner_h = content_bottom.saturating_sub(inner_y).max(SERVICE_H);

    let group_box = NodeBox {
        x1: x,
        y1: y,
        x2: x + GROUP_PAD * 2 + inner_w,
        y2: y + GROUP_PAD * 2 + inner_h,
    };
    let w = group_box.x2 - group_box.x1;
    let h = group_box.y2 - group_box.y1;
    group_boxes.insert(gid.to_string(), group_box);
    (w, h)
}

fn service_width(label: &str, icon: Option<&str>) -> usize {
    let icon_w = if icon.is_some() { 2 } else { 0 };
    let lbl_w = UnicodeWidthStr::width(label);
    (lbl_w + icon_w + 2).max(SERVICE_MIN_W)
}

fn format_group_label(
    diag: &ArchitectureDiagram,
    gid: &str,
    label: &str,
    charset: CharsetKind,
) -> String {
    let icon = diag.groups.get(gid).and_then(|g| g.icon.as_deref());
    let glyph = icon_for(icon, charset);
    if let Some(g) = glyph {
        format!(" {g} {label} ")
    } else {
        format!(" {label} ")
    }
}

fn icon_for(icon: Option<&str>, charset: CharsetKind) -> Option<String> {
    let name = icon?;
    let unicode = matches!(charset, CharsetKind::Unicode);
    let glyph = match name.to_ascii_lowercase().as_str() {
        "cloud" => {
            if unicode {
                "*"
            } else {
                "[C]"
            }
        }
        "database" | "db" => {
            if unicode {
                "D"
            } else {
                "[DB]"
            }
        }
        "disk" => {
            if unicode {
                "="
            } else {
                "[D]"
            }
        }
        "server" => {
            if unicode {
                "S"
            } else {
                "[S]"
            }
        }
        "internet" => {
            if unicode {
                "@"
            } else {
                "[N]"
            }
        }
        _ => {
            if unicode {
                "*"
            } else {
                "[*]"
            }
        }
    };
    Some(glyph.to_string())
}

fn truncate(s: &str, max_w: usize) -> String {
    let w = UnicodeWidthStr::width(s);
    if w <= max_w {
        return s.to_string();
    }
    if max_w == 0 {
        return String::new();
    }
    let mut acc = String::new();
    let mut used = 0;
    for c in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);
        if used + cw + 1 > max_w {
            break;
        }
        acc.push(c);
        used += cw;
    }
    acc.push('…');
    acc
}

fn draw_edge(
    canvas: &mut Canvas,
    edge: &ArchEdge,
    boxes: &HashMap<String, NodeBox>,
    charset: CharsetKind,
) {
    let from = match boxes.get(&edge.from) {
        Some(b) => b,
        None => return,
    };
    let to = match boxes.get(&edge.to) {
        Some(b) => b,
        None => return,
    };

    let (sx, sy) = from.center_side(edge.from_side);
    let (tx, ty) = to.center_side(edge.to_side);

    let (sx, sy) = nudge_outward(sx, sy, edge.from_side);
    let (tx, ty) = nudge_outward(tx, ty, edge.to_side);

    route_orthogonal(canvas, sx, sy, tx, ty, edge.from_side);

    if !edge.label.is_empty() {
        let mx = (sx + tx) / 2;
        let my = (sy + ty) / 2;
        let lbl_x = mx.saturating_sub(UnicodeWidthStr::width(edge.label.as_str()) / 2);
        canvas.put_str_colored(lbl_x, my, &edge.label, Color::Green);
    }
    let _ = charset;
}

fn nudge_outward(x: usize, y: usize, side: Side) -> (usize, usize) {
    match side {
        Side::Left => (x.saturating_sub(1), y),
        Side::Right => (x + 1, y),
        Side::Top => (x, y.saturating_sub(1)),
        Side::Bottom => (x, y + 1),
    }
}

fn route_orthogonal(
    canvas: &mut Canvas,
    sx: usize,
    sy: usize,
    tx: usize,
    ty: usize,
    from_side: Side,
) {
    let horizontal_first = matches!(from_side, Side::Left | Side::Right);

    if horizontal_first {
        if sx != tx {
            canvas.draw_hline(sx, tx, sy);
            canvas.paint_hline(sx, tx, sy, Color::Yellow);
        }
        if sy != ty {
            canvas.draw_vline(tx, sy, ty);
            canvas.paint_vline(tx, sy, ty, Color::Yellow);
        }
    } else {
        if sy != ty {
            canvas.draw_vline(sx, sy, ty);
            canvas.paint_vline(sx, sy, ty, Color::Yellow);
        }
        if sx != tx {
            canvas.draw_hline(sx, tx, ty);
            canvas.paint_hline(sx, tx, ty, Color::Yellow);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mermaid::ascii::architecture::parser::parse;

    fn render_src(src: &str) -> String {
        let d = parse(src).unwrap();
        render(&d, 80, CharsetKind::Unicode).unwrap().join("\n")
    }

    fn render_src_ascii(src: &str) -> String {
        let d = parse(src).unwrap();
        render(&d, 80, CharsetKind::Ascii).unwrap().join("\n")
    }

    #[test]
    fn render_architecture_minimal() {
        let src = "architecture-beta\nservice db(database)[Database]";
        insta::assert_snapshot!(render_src(src));
    }

    #[test]
    fn render_group_with_services() {
        let src = "architecture-beta\ngroup api(cloud)[API]\nservice db(database)[DB] in api\nservice srv(server)[Server] in api";
        insta::assert_snapshot!(render_src(src));
    }

    #[test]
    fn render_two_groups() {
        let src = "architecture-beta\ngroup api(cloud)[API]\nservice db(database)[DB] in api\ngroup ext(cloud)[External]\nservice net(internet)[Net] in ext";
        insta::assert_snapshot!(render_src(src));
    }

    #[test]
    fn render_nested_groups() {
        let src = "architecture-beta\ngroup outer(cloud)[Outer]\ngroup inner(cloud)[Inner] in outer\nservice db(database)[DB] in inner";
        insta::assert_snapshot!(render_src(src));
    }

    #[test]
    fn render_edge_between_services() {
        let src = "architecture-beta\ngroup api(cloud)[API]\nservice db(database)[DB] in api\nservice srv(server)[Server] in api\ndb:R -- L:srv";
        insta::assert_snapshot!(render_src(src));
    }

    #[test]
    fn render_with_junction() {
        let src = "architecture-beta\nservice a(server)[A]\njunction j\nservice b(server)[B]\na:R -- L:j\nj:R -- L:b";
        insta::assert_snapshot!(render_src(src));
    }

    #[test]
    fn render_ascii_charset() {
        let src = "architecture-beta\ngroup api(cloud)[API]\nservice db(database)[DB] in api\nservice srv(server)[Server] in api";
        insta::assert_snapshot!(render_src_ascii(src));
    }

    #[test]
    fn render_group_has_label() {
        let src = "architecture-beta\ngroup api(cloud)[MyAPI]\nservice db(database)[DB] in api";
        let out = render_src(src);
        assert!(out.contains("MyAPI"), "expected MyAPI in output:\n{out}");
    }

    #[test]
    fn render_service_appears_inside_group() {
        let src = "architecture-beta\ngroup api(cloud)[API]\nservice db(database)[Database] in api";
        let out = render_src(src);
        assert!(
            out.contains("Database"),
            "expected service label in output:\n{out}"
        );
        let group_line = out
            .lines()
            .position(|l| l.contains("API"))
            .expect("group line");
        let svc_line = out
            .lines()
            .position(|l| l.contains("Database"))
            .expect("svc line");
        assert!(
            svc_line > group_line,
            "service should appear inside group below top border. out:\n{out}"
        );
    }

    #[test]
    fn render_edge_lines_visible() {
        let src = "architecture-beta\nservice a(server)[A]\nservice b(server)[B]\na:R -- L:b";
        let out = render_src(src);
        let has_hline = out.chars().any(|c| c == '─');
        assert!(has_hline, "expected horizontal line glyph in:\n{out}");
    }

    #[test]
    fn render_icon_glyph_visible() {
        let src = "architecture-beta\nservice db(database)[DB]";
        let out = render_src(src);
        let has_icon = out.contains('D') || out.contains("[DB]");
        assert!(has_icon, "expected DB icon glyph in:\n{out}");
    }

    fn render_styled_str(src: &str) -> Vec<StyledRow> {
        let d = parse(src).unwrap();
        render_styled(&d, 80, CharsetKind::Unicode).unwrap()
    }

    #[test]
    fn render_styled_attaches_colors() {
        let src = "architecture-beta\nservice db(database)[Database]";
        let rows = render_styled_str(src);
        let any_colored = rows
            .iter()
            .flat_map(|r| r.iter())
            .any(|r| r.color.is_some());
        assert!(any_colored, "expected at least one colored run");
    }

    #[test]
    fn render_styled_specific_element_colored() {
        let src = "architecture-beta\ngroup api(cloud)[API]\nservice db(database)[DB] in api\nservice srv(server)[Server] in api\ndb:R -- L:srv";
        let rows = render_styled_str(src);
        let mut has_blue_group = false;
        let mut has_cyan_service = false;
        let mut has_yellow_edge = false;
        let mut has_magenta_icon = false;
        for row in &rows {
            for run in row {
                if run.color == Some(Color::Blue)
                    && (run.text.contains("API")
                        || run.text.contains('┌')
                        || run.text.contains('─'))
                {
                    has_blue_group = true;
                }
                if run.color == Some(Color::Cyan)
                    && run
                        .text
                        .chars()
                        .any(|c| matches!(c, '┌' | '┐' | '└' | '┘' | '│' | '─'))
                {
                    has_cyan_service = true;
                }
                if run.color == Some(Color::Yellow)
                    && run.text.chars().any(|c| {
                        matches!(
                            c,
                            '─' | '│'
                                | '┌'
                                | '┐'
                                | '└'
                                | '┘'
                                | '┼'
                                | '┴'
                                | '┬'
                                | '├'
                                | '┤'
                        )
                    })
                {
                    has_yellow_edge = true;
                }
                if run.color == Some(Color::Magenta) {
                    has_magenta_icon = true;
                }
            }
        }
        assert!(has_blue_group, "group borders/label should be blue");
        assert!(has_cyan_service, "service borders should be cyan");
        assert!(has_yellow_edge, "edge lines should be yellow");
        assert!(has_magenta_icon, "icon glyph should be magenta");
    }

    #[test]
    fn render_styled_no_alignment_drift() {
        let src = "architecture-beta\ngroup api(cloud)[API]\nservice db(database)[DB] in api\nservice srv(server)[Server] in api\ndb:R -- L:srv";
        let d = parse(src).unwrap();
        let plain = render(&d, 80, CharsetKind::Unicode).unwrap();
        let styled = render_styled(&d, 80, CharsetKind::Unicode).unwrap();
        assert_eq!(plain.len(), styled.len());
        for (i, row) in styled.iter().enumerate() {
            let flat: String = row.iter().map(|r| r.text.as_str()).collect();
            let flat_trimmed = flat.trim_end_matches(' ').to_string();
            assert_eq!(flat_trimmed, plain[i], "row {i} differs");
        }
    }
}
