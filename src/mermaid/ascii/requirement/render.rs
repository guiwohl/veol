use super::ast::{
    Element, ReqRelation, ReqRelationKind, ReqType, Requirement, RequirementDiagram, RiskLevel,
    VerifyMethod,
};
use crate::mermaid::ascii::canvas::Canvas;
use crate::mermaid::ascii::charset::get_charset;
use crate::mermaid::ascii::{AsciiRenderError, CharsetKind, StyledRow};
use ratatui::style::Color;
use std::collections::HashMap;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn render(
    diag: &RequirementDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_lines())
}

pub fn render_styled(
    diag: &RequirementDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_styled_lines())
}

fn render_to_canvas(
    diag: &RequirementDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    if diag.requirements.is_empty() && diag.elements.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    let nodes = collect_nodes(diag);
    let placement = place_nodes(&nodes);
    let boxes = compute_boxes(&nodes, &placement);

    let mut canvas_w = boxes.values().map(|b| b.x2 + 1).max().unwrap_or(1);
    let mut canvas_h = boxes.values().map(|b| b.y2 + 1).max().unwrap_or(1);
    canvas_w += 6;
    canvas_h += 3;
    canvas_w = canvas_w.min(max_width as usize).max(1);

    let mut canvas = Canvas::new(canvas_w, canvas_h, charset);

    for n in &nodes {
        if let Some(b) = boxes.get(&n.name) {
            draw_node_box(&mut canvas, charset, n, b);
        }
    }

    for rel in &diag.relations {
        if let (Some(fb), Some(tb)) = (boxes.get(&rel.from), boxes.get(&rel.to)) {
            draw_relation(&mut canvas, charset, rel, fb, tb);
        }
    }

    Ok(canvas)
}

#[derive(Debug, Clone)]
struct Node {
    name: String,
    lines: Vec<String>,
    header_lines: usize,
}

#[derive(Debug, Clone, Copy)]
struct Box {
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
}

impl Box {
    fn mid_y(&self) -> usize {
        (self.y1 + self.y2) / 2
    }
}

fn collect_nodes(diag: &RequirementDiagram) -> Vec<Node> {
    let mut out: Vec<Node> = Vec::new();
    for (name, r) in &diag.requirements {
        out.push(requirement_node(name, r));
    }
    for (name, e) in &diag.elements {
        out.push(element_node(name, e));
    }
    out
}

fn requirement_kind_label(k: Option<ReqType>) -> &'static str {
    match k {
        Some(ReqType::Requirement) | None => "<<requirement>>",
        Some(ReqType::FunctionalRequirement) => "<<functionalRequirement>>",
        Some(ReqType::InterfaceRequirement) => "<<interfaceRequirement>>",
        Some(ReqType::PerformanceRequirement) => "<<performanceRequirement>>",
        Some(ReqType::PhysicalRequirement) => "<<physicalRequirement>>",
        Some(ReqType::DesignConstraint) => "<<designConstraint>>",
    }
}

fn risk_label(r: RiskLevel) -> &'static str {
    match r {
        RiskLevel::Low => "Low",
        RiskLevel::Medium => "Medium",
        RiskLevel::High => "High",
    }
}

fn verify_label(v: VerifyMethod) -> &'static str {
    match v {
        VerifyMethod::Analysis => "Analysis",
        VerifyMethod::Inspection => "Inspection",
        VerifyMethod::Test => "Test",
        VerifyMethod::Demonstration => "Demonstration",
    }
}

fn requirement_node(name: &str, r: &Requirement) -> Node {
    let mut lines: Vec<String> = Vec::new();
    lines.push(requirement_kind_label(r.kind).to_string());
    lines.push(name.to_string());
    let header_lines = lines.len();
    if let Some(id) = &r.id {
        lines.push(format!("id: {id}"));
    }
    if let Some(t) = &r.text {
        lines.push(format!("text: {t}"));
    }
    if let Some(risk) = r.risk {
        lines.push(format!("risk: {}", risk_label(risk)));
    }
    if let Some(v) = r.verify {
        lines.push(format!("verifymethod: {}", verify_label(v)));
    }
    Node {
        name: name.to_string(),
        lines,
        header_lines,
    }
}

fn element_node(name: &str, e: &Element) -> Node {
    let mut lines: Vec<String> = Vec::new();
    lines.push("<<element>>".to_string());
    lines.push(name.to_string());
    let header_lines = lines.len();
    if let Some(t) = &e.element_type {
        lines.push(format!("type: {t}"));
    }
    if let Some(d) = &e.docref {
        lines.push(format!("docRef: {d}"));
    }
    Node {
        name: name.to_string(),
        lines,
        header_lines,
    }
}

fn node_dims(n: &Node) -> (usize, usize) {
    let max_text = n
        .lines
        .iter()
        .map(|s| s.as_str().width())
        .max()
        .unwrap_or(0);
    let inner_w = max_text + 4;
    let body_lines = n.lines.len();
    let has_body = body_lines > n.header_lines;
    let inner_h = body_lines + if has_body { 1 } else { 0 };
    (inner_w, inner_h)
}

fn place_nodes(nodes: &[Node]) -> HashMap<String, (usize, usize)> {
    let mut out = HashMap::new();
    let n = nodes.len();
    let cols = (n as f64).sqrt().ceil() as usize;
    let cols = cols.max(1);
    for (i, node) in nodes.iter().enumerate() {
        let c = i % cols;
        let r = i / cols;
        out.insert(node.name.clone(), (c, r));
    }
    out
}

fn compute_boxes(
    nodes: &[Node],
    placement: &HashMap<String, (usize, usize)>,
) -> HashMap<String, Box> {
    let mut col_w: HashMap<usize, usize> = HashMap::new();
    let mut row_h: HashMap<usize, usize> = HashMap::new();
    for n in nodes {
        let (w, h) = node_dims(n);
        if let Some(&(c, r)) = placement.get(&n.name) {
            let cw = col_w.entry(c).or_insert(0);
            if w + 2 > *cw {
                *cw = w + 2;
            }
            let rh = row_h.entry(r).or_insert(0);
            if h + 2 > *rh {
                *rh = h + 2;
            }
        }
    }
    let h_gap = 12usize;
    let v_gap = 4usize;
    let max_col = col_w.keys().copied().max().unwrap_or(0);
    let max_row = row_h.keys().copied().max().unwrap_or(0);
    let mut col_x: HashMap<usize, usize> = HashMap::new();
    let mut row_y: HashMap<usize, usize> = HashMap::new();
    let mut x = 0;
    for c in 0..=max_col {
        col_x.insert(c, x);
        x += col_w.get(&c).copied().unwrap_or(0) + h_gap;
    }
    let mut y = 0;
    for r in 0..=max_row {
        row_y.insert(r, y);
        y += row_h.get(&r).copied().unwrap_or(0) + v_gap;
    }

    let mut out = HashMap::new();
    for n in nodes {
        let (c, r) = match placement.get(&n.name) {
            Some(p) => *p,
            None => continue,
        };
        let (w, h) = node_dims(n);
        let x1 = col_x.get(&c).copied().unwrap_or(0);
        let y1 = row_y.get(&r).copied().unwrap_or(0);
        let x2 = x1 + w + 1;
        let y2 = y1 + h + 1;
        out.insert(n.name.clone(), Box { x1, y1, x2, y2 });
    }
    out
}

fn draw_node_box(canvas: &mut Canvas, charset: CharsetKind, n: &Node, b: &Box) {
    canvas.draw_box(b.x1, b.y1, b.x2, b.y2);
    canvas.paint_box(b.x1, b.y1, b.x2, b.y2, Color::Cyan);
    let cs = get_charset(charset);

    let inner_left = b.x1 + 2;
    let inner_right = b.x2 - 1;
    let inner_w = inner_right.saturating_sub(b.x1 + 1);

    let mut y = b.y1 + 1;
    for (i, line) in n.lines.iter().enumerate() {
        if y >= b.y2 {
            break;
        }
        if i < n.header_lines {
            let lw = line.as_str().width();
            let pad = inner_w.saturating_sub(lw) / 2;
            let x = b.x1 + 1 + pad;
            canvas.put_str(x, y, line);
            // Stereotype = first header line; name = subsequent header lines (typically just one).
            let color = if i == 0 { Color::Blue } else { Color::Yellow };
            canvas.paint_text(x, y, line, color);
        } else {
            canvas.put_str(inner_left, y, line);
        }
        y += 1;
        if i + 1 == n.header_lines && i + 1 < n.lines.len() && y < b.y2 {
            for x in (b.x1 + 1)..b.x2 {
                canvas.put_char(x, y, cs.horiz());
                canvas.set_color(x, y, Color::Cyan);
            }
            match charset {
                CharsetKind::Unicode => {
                    canvas.put_char(b.x1, y, cs.tee_right());
                    canvas.set_color(b.x1, y, Color::Cyan);
                    canvas.put_char(b.x2, y, cs.tee_left());
                    canvas.set_color(b.x2, y, Color::Cyan);
                }
                CharsetKind::Ascii => {
                    canvas.put_char(b.x1, y, '+');
                    canvas.set_color(b.x1, y, Color::Cyan);
                    canvas.put_char(b.x2, y, '+');
                    canvas.set_color(b.x2, y, Color::Cyan);
                }
            }
            y += 1;
        }
    }
}

fn relation_label(k: ReqRelationKind) -> &'static str {
    match k {
        ReqRelationKind::Contains => "contains",
        ReqRelationKind::Copies => "copies",
        ReqRelationKind::Derives => "derives",
        ReqRelationKind::Satisfies => "satisfies",
        ReqRelationKind::Verifies => "verifies",
        ReqRelationKind::Refines => "refines",
        ReqRelationKind::Traces => "traces",
    }
}

fn arrow_head_horiz(charset: CharsetKind, dir_right: bool) -> char {
    match (charset, dir_right) {
        (CharsetKind::Unicode, true) => '▷',
        (CharsetKind::Unicode, false) => '◁',
        (CharsetKind::Ascii, true) => '>',
        (CharsetKind::Ascii, false) => '<',
    }
}

fn place_label(canvas: &mut Canvas, label: &str, mid_x: usize, label_y: usize) {
    let lw = label.width();
    let start_label = mid_x.saturating_sub(lw / 2);
    let mut cx = start_label;
    for ch in label.chars() {
        if cx < canvas.width() {
            canvas.put_char(cx, label_y, ch);
            canvas.set_color(cx, label_y, Color::Green);
        }
        cx += UnicodeWidthChar::width(ch).unwrap_or(1).max(1);
    }
}

fn draw_relation(canvas: &mut Canvas, charset: CharsetKind, rel: &ReqRelation, fb: &Box, tb: &Box) {
    let cs = get_charset(charset);

    let from_left_of_to = fb.x2 < tb.x1;
    let from_right_of_to = tb.x2 < fb.x1;
    let from_above = fb.y2 < tb.y1;
    let from_below = tb.y2 < fb.y1;

    let same_row = (fb.mid_y() as i32 - tb.mid_y() as i32).abs() <= 1;
    if same_row && (from_left_of_to || from_right_of_to) {
        let dir_right = from_left_of_to;
        let sy = fb.mid_y();
        let ey = tb.mid_y();
        let (sx, ex) = if dir_right {
            (fb.x2, tb.x1)
        } else {
            (fb.x1, tb.x2)
        };

        if sy == ey {
            let (lo, hi) = if sx <= ex {
                (sx + 1, ex.saturating_sub(1))
            } else {
                (ex + 1, sx.saturating_sub(1))
            };
            if lo <= hi {
                for x in lo..=hi {
                    canvas.put_char(x, sy, cs.horiz());
                    canvas.set_color(x, sy, Color::Yellow);
                }
            }
            let head_x = if dir_right {
                ex.saturating_sub(1)
            } else {
                ex + 1
            };
            canvas.put_char(head_x, ey, arrow_head_horiz(charset, dir_right));
            canvas.set_color(head_x, ey, Color::Yellow);
            let mid_x = (sx + ex) / 2;
            let label_y = if sy > 0 { sy - 1 } else { sy + 1 };
            place_label(canvas, relation_label(rel.kind), mid_x, label_y);
        }
        return;
    }

    // Non-same-row: route via top/bottom of boxes so we don't collide with same-row edges.
    if from_left_of_to || from_right_of_to || from_above || from_below {
        let sx = (fb.x1 + fb.x2) / 2;
        let ex = (tb.x1 + tb.x2) / 2;
        let (sy, ey, dir_down) = if from_above {
            (fb.y2, tb.y1, true)
        } else {
            (fb.y1, tb.y2, false)
        };
        if sx == ex {
            let (lo, hi) = if sy <= ey {
                (sy + 1, ey.saturating_sub(1))
            } else {
                (ey + 1, sy.saturating_sub(1))
            };
            if lo <= hi {
                for y in lo..=hi {
                    canvas.put_char(sx, y, cs.vert());
                    canvas.set_color(sx, y, Color::Yellow);
                }
            }
        } else {
            let bend_y = (sy + ey) / 2;
            let (vlo, vhi) = if sy <= bend_y {
                (sy + 1, bend_y)
            } else {
                (bend_y, sy.saturating_sub(1))
            };
            if vlo <= vhi {
                for y in vlo..=vhi {
                    canvas.put_char(sx, y, cs.vert());
                    canvas.set_color(sx, y, Color::Yellow);
                }
            }
            let (hlo, hhi) = if sx <= ex {
                (sx + 1, ex.saturating_sub(0))
            } else {
                (ex, sx.saturating_sub(1))
            };
            if hlo <= hhi {
                for x in hlo..=hhi {
                    canvas.put_char(x, bend_y, cs.horiz());
                    canvas.set_color(x, bend_y, Color::Yellow);
                }
            }
            let corner = match (sx <= ex, sy <= ey) {
                (true, true) => cs.bot_left(),
                (true, false) => cs.top_left(),
                (false, true) => cs.bot_right(),
                (false, false) => cs.top_right(),
            };
            canvas.put_char(sx, bend_y, corner);
            canvas.set_color(sx, bend_y, Color::Yellow);
            let (vlo2, vhi2) = if bend_y <= ey {
                (bend_y + 1, ey.saturating_sub(1))
            } else {
                (ey + 1, bend_y.saturating_sub(1))
            };
            if vlo2 <= vhi2 {
                for y in vlo2..=vhi2 {
                    canvas.put_char(ex, y, cs.vert());
                    canvas.set_color(ex, y, Color::Yellow);
                }
            }
            let corner2 = match (sx <= ex, sy <= ey) {
                (true, true) => cs.top_right(),
                (true, false) => cs.bot_right(),
                (false, true) => cs.top_left(),
                (false, false) => cs.bot_left(),
            };
            canvas.put_char(ex, bend_y, corner2);
            canvas.set_color(ex, bend_y, Color::Yellow);
        }
        let head_y = if dir_down {
            ey.saturating_sub(1)
        } else {
            ey + 1
        };
        let head = if dir_down {
            cs.arrow_down()
        } else {
            cs.arrow_up()
        };
        canvas.put_char(ex, head_y, head);
        canvas.set_color(ex, head_y, Color::Yellow);
        let mid_x = (sx + ex) / 2;
        let mid_y = (sy + ey) / 2;
        place_label(canvas, relation_label(rel.kind), mid_x, mid_y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mermaid::ascii::StyledRun;

    fn render_src(src: &str) -> String {
        crate::mermaid::ascii::requirement::render(src, 200, CharsetKind::Unicode)
            .unwrap()
            .join("\n")
    }

    fn render_ascii(src: &str) -> String {
        crate::mermaid::ascii::requirement::render(src, 200, CharsetKind::Ascii)
            .unwrap()
            .join("\n")
    }

    fn render_styled_src(src: &str) -> Vec<Vec<StyledRun>> {
        crate::mermaid::ascii::requirement::render_styled(src, 200, CharsetKind::Unicode).unwrap()
    }

    #[test]
    fn render_requirement_minimal() {
        insta::assert_snapshot!(render_src(
            "requirementDiagram\nrequirement r1 {\n  id: 1\n}"
        ));
    }

    #[test]
    fn render_requirement_with_all_fields() {
        insta::assert_snapshot!(render_src(
            "requirementDiagram\nrequirement test_req {\n  id: 1\n  text: the test text.\n  risk: high\n  verifymethod: test\n}"
        ));
    }

    #[test]
    fn render_element_box() {
        insta::assert_snapshot!(render_src(
            "requirementDiagram\nelement test_entity {\n  type: simulation\n}"
        ));
    }

    #[test]
    fn render_requirement_and_element_with_relation() {
        insta::assert_snapshot!(render_src(
            "requirementDiagram\nrequirement r1 {\n  id: 1\n}\nelement e1 {\n  type: doc\n}\ne1 - satisfies -> r1"
        ));
    }

    #[test]
    fn render_three_node_chain() {
        insta::assert_snapshot!(render_src(
            "requirementDiagram\nrequirement a {\n  id: 1\n}\nrequirement b {\n  id: 2\n}\nrequirement c {\n  id: 3\n}\na - contains -> b\nb - contains -> c"
        ));
    }

    #[test]
    fn render_ascii_charset() {
        insta::assert_snapshot!(render_ascii(
            "requirementDiagram\nrequirement r1 {\n  id: 1\n}\nelement e1 {\n  type: doc\n}\ne1 - satisfies -> r1"
        ));
    }

    #[test]
    fn render_kind_label_visible() {
        let out = render_src(
            "requirementDiagram\nfunctionalRequirement fr {\n  id: 1.1\n  text: hello\n}",
        );
        assert!(out.contains("<<functionalRequirement>>"));
        assert!(out.contains("fr"));
    }

    #[test]
    fn render_relation_arrow_visible() {
        let out = render_src(
            "requirementDiagram\nrequirement a {\n  id: 1\n}\nrequirement b {\n  id: 2\n}\na - traces -> b",
        );
        assert!(out.contains("traces"));
        assert!(out.contains('▷') || out.contains('▽') || out.contains('▼') || out.contains('◁'));
    }

    #[test]
    fn render_styled_attaches_colors() {
        let rows = render_styled_src(
            "requirementDiagram\nrequirement r1 {\n  id: 1\n}\nelement e1 {\n  type: doc\n}\ne1 - satisfies -> r1",
        );
        let any_colored = rows
            .iter()
            .flat_map(|r| r.iter())
            .any(|run| run.color.is_some());
        assert!(any_colored, "expected colored runs");
    }

    #[test]
    fn render_styled_specific_element_colored() {
        let rows = render_styled_src(
            "requirementDiagram\nrequirement r1 {\n  id: 1\n}\nelement e1 {\n  type: doc\n}\ne1 - satisfies -> r1",
        );
        // Stereotype <<requirement>> = blue
        let blue_text: String = rows
            .iter()
            .flat_map(|r| r.iter())
            .filter(|r| r.color == Some(Color::Blue))
            .map(|r| r.text.as_str())
            .collect();
        assert!(
            blue_text.contains("<<"),
            "expected blue stereotype, got blue={blue_text:?}"
        );
        // Name = yellow (e.g. "r1")
        let yellow_text: String = rows
            .iter()
            .flat_map(|r| r.iter())
            .filter(|r| r.color == Some(Color::Yellow))
            .map(|r| r.text.as_str())
            .collect();
        assert!(
            yellow_text.contains("r1") || yellow_text.contains("e1"),
            "expected yellow name, got yellow={yellow_text:?}"
        );
        // Box border = cyan
        let cyan_text: String = rows
            .iter()
            .flat_map(|r| r.iter())
            .filter(|r| r.color == Some(Color::Cyan))
            .map(|r| r.text.as_str())
            .collect();
        assert!(
            cyan_text.contains('─')
                || cyan_text.contains('│')
                || cyan_text.contains('┌')
                || cyan_text.contains('┘'),
            "expected cyan border chars, got cyan={cyan_text:?}"
        );
        // Relation label = green
        let green_text: String = rows
            .iter()
            .flat_map(|r| r.iter())
            .filter(|r| r.color == Some(Color::Green))
            .map(|r| r.text.as_str())
            .collect();
        assert!(
            green_text.contains("satisfies"),
            "expected green relation verb, got green={green_text:?}"
        );
    }

    #[test]
    fn render_styled_no_alignment_drift() {
        let src =
            "requirementDiagram\nrequirement r1 {\n  id: 1\n}\nelement e1 {\n  type: doc\n}\ne1 - satisfies -> r1";
        let plain =
            crate::mermaid::ascii::requirement::render(src, 200, CharsetKind::Unicode).unwrap();
        let styled = render_styled_src(src);
        assert_eq!(plain.len(), styled.len());
        for (i, row) in styled.iter().enumerate() {
            let flat: String = row.iter().map(|r| r.text.as_str()).collect();
            let flat_trimmed = flat.trim_end().to_string();
            assert_eq!(flat_trimmed, plain[i], "row {i} drift");
        }
    }
}
