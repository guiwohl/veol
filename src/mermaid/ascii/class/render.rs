use super::ast::{Class, ClassDiagram, Member, Relation, RelationKind, Visibility};
use crate::mermaid::ascii::canvas::Canvas;
use crate::mermaid::ascii::charset::{get_charset, Charset};
use crate::mermaid::ascii::{AsciiRenderError, CharsetKind, StyledRow};
use ratatui::style::Color;
use std::collections::HashMap;
use unicode_width::UnicodeWidthStr;

pub fn render(
    diag: &ClassDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    if diag.classes.is_empty() {
        return Ok(Vec::new());
    }
    let canvas = render_to_canvas(diag, max_width, charset)?;
    Ok(canvas.into_lines())
}

pub fn render_styled(
    diag: &ClassDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    if diag.classes.is_empty() {
        return Ok(Vec::new());
    }
    let canvas = render_to_canvas(diag, max_width, charset)?;
    Ok(canvas.into_styled_lines())
}

fn render_to_canvas(
    diag: &ClassDiagram,
    _max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    let layout = layout_classes(diag);
    let positions = place_grid(&layout);
    let cell_w = layout.iter().map(|b| b.width).max().unwrap_or(0) + H_GAP;
    let cell_h = layout.iter().map(|b| b.height).max().unwrap_or(0) + V_GAP;

    let mut max_col = 0usize;
    let mut max_row = 0usize;
    for p in positions.values() {
        max_col = max_col.max(p.col);
        max_row = max_row.max(p.row);
    }
    let total_w = (max_col + 1) * cell_w + H_GAP;
    let total_h = (max_row + 1) * cell_h + V_GAP;

    let mut canvas = Canvas::new(total_w.max(1), total_h.max(1), charset);
    let cs = get_charset(charset);

    let mut class_rects: HashMap<String, (usize, usize, usize, usize)> = HashMap::new();
    for cls in &layout {
        let p = positions.get(&cls.name).unwrap();
        let cx = H_GAP + p.col * cell_w;
        let cy = V_GAP + p.row * cell_h;
        let x2 = cx + cls.width - 1;
        let y2 = cy + cls.height - 1;
        draw_class_box(&mut canvas, cs, cls, cx, cy);
        class_rects.insert(cls.name.clone(), (cx, cy, x2, y2));
    }

    for rel in &diag.relations {
        draw_relation(&mut canvas, cs, charset, rel, &class_rects);
    }

    Ok(canvas)
}

const H_GAP: usize = 6;
const V_GAP: usize = 3;

struct ClassBox {
    name: String,
    lines_top: Vec<String>,
    lines_attr: Vec<String>,
    lines_meth: Vec<String>,
    width: usize,
    height: usize,
}

#[derive(Clone, Copy)]
struct Pos {
    col: usize,
    row: usize,
}

fn layout_classes(diag: &ClassDiagram) -> Vec<ClassBox> {
    let mut out = Vec::new();
    for (name, cls) in &diag.classes {
        out.push(build_box(name, cls));
    }
    out
}

fn build_box(name: &str, cls: &Class) -> ClassBox {
    let mut header_text = name.to_string();
    if let Some(g) = &cls.generic {
        header_text = format!("{name}<{g}>");
    }
    let mut top: Vec<String> = Vec::new();
    if let Some(a) = &cls.annotation {
        top.push(format!("<<{}>>", a));
    }
    top.push(header_text);

    let attr_lines: Vec<String> = cls.attributes.iter().map(format_member).collect();
    let meth_lines: Vec<String> = cls.methods.iter().map(format_member).collect();

    let mut max_w = 0usize;
    for s in top.iter().chain(attr_lines.iter()).chain(meth_lines.iter()) {
        max_w = max_w.max(UnicodeWidthStr::width(s.as_str()));
    }

    let width = max_w + 4;
    let mut height = 2 + top.len();
    if !attr_lines.is_empty() {
        height += attr_lines.len() + 1;
    }
    if !meth_lines.is_empty() {
        height += meth_lines.len() + 1;
    }
    if attr_lines.is_empty() && meth_lines.is_empty() && top.len() <= 1 {
        height = 3;
    }

    ClassBox {
        name: name.to_string(),
        lines_top: top,
        lines_attr: attr_lines,
        lines_meth: meth_lines,
        width,
        height,
    }
}

fn format_member(m: &Member) -> String {
    let vis = match m.visibility {
        Some(Visibility::Public) => "+",
        Some(Visibility::Private) => "-",
        Some(Visibility::Protected) => "#",
        Some(Visibility::Package) => "~",
        None => "",
    };
    let mut body = m.name.clone();
    if m.is_method {
        body.push_str("()");
    }
    if let Some(t) = &m.type_hint {
        if m.is_method {
            body.push_str(&format!(": {t}"));
        } else {
            body = format!("{t} {body}");
        }
    }
    if m.is_static {
        body.push('$');
    }
    if m.is_abstract {
        body.push('*');
    }
    if vis.is_empty() {
        body
    } else {
        format!("{vis}{body}")
    }
}

fn place_grid(boxes: &[ClassBox]) -> HashMap<String, Pos> {
    let mut out: HashMap<String, Pos> = HashMap::new();
    if boxes.is_empty() {
        return out;
    }
    let n = boxes.len();
    let cols = (n as f64).sqrt().ceil() as usize;
    let cols = cols.max(1);
    for (i, b) in boxes.iter().enumerate() {
        out.insert(
            b.name.clone(),
            Pos {
                col: i % cols,
                row: i / cols,
            },
        );
    }
    out
}

fn draw_class_box(canvas: &mut Canvas, cs: &dyn Charset, cls: &ClassBox, x: usize, y: usize) {
    let x2 = x + cls.width - 1;
    let y2 = y + cls.height - 1;
    canvas.draw_box(x, y, x2, y2);
    canvas.paint_box(x, y, x2, y2, Color::Cyan);

    let mut cur_y = y + 1;
    for line in &cls.lines_top {
        let lw = UnicodeWidthStr::width(line.as_str());
        let lx = x + 1 + (cls.width.saturating_sub(2 + lw)) / 2;
        canvas.put_str(lx, cur_y, line);
        let header_color = if line.starts_with("<<") && line.ends_with(">>") {
            Color::Blue
        } else {
            Color::Yellow
        };
        canvas.paint_text(lx, cur_y, line, header_color);
        cur_y += 1;
    }

    if !cls.lines_attr.is_empty() {
        draw_divider(canvas, cs, x, x2, cur_y);
        cur_y += 1;
        for line in &cls.lines_attr {
            canvas.put_str(x + 2, cur_y, line);
            cur_y += 1;
        }
    }
    if !cls.lines_meth.is_empty() {
        draw_divider(canvas, cs, x, x2, cur_y);
        cur_y += 1;
        for line in &cls.lines_meth {
            canvas.put_str(x + 2, cur_y, line);
            canvas.paint_text(x + 2, cur_y, line, Color::Green);
            cur_y += 1;
        }
    }
}

fn draw_divider(canvas: &mut Canvas, cs: &dyn Charset, x1: usize, x2: usize, y: usize) {
    canvas.put_char(x1, y, cs.tee_right());
    canvas.put_char(x2, y, cs.tee_left());
    for x in (x1 + 1)..x2 {
        canvas.put_char(x, y, cs.horiz());
    }
    canvas.paint_hline(x1, x2, y, Color::Cyan);
}

fn draw_relation(
    canvas: &mut Canvas,
    cs: &dyn Charset,
    charset: CharsetKind,
    rel: &Relation,
    rects: &HashMap<String, (usize, usize, usize, usize)>,
) {
    let from = match rects.get(&rel.from) {
        Some(r) => *r,
        None => return,
    };
    let to = match rects.get(&rel.to) {
        Some(r) => *r,
        None => return,
    };

    let from_center = ((from.0 + from.2) / 2, (from.1 + from.3) / 2);
    let to_center = ((to.0 + to.2) / 2, (to.1 + to.3) / 2);

    let same_row =
        (from.1..=from.3).contains(&to_center.1) || (to.1..=to.3).contains(&from_center.1);

    let (start, end, orient_start, orient_end) = if same_row {
        let overlap_lo = from.1.max(to.1);
        let overlap_hi = from.3.min(to.3);
        let shared_y = (overlap_lo + overlap_hi) / 2;
        if from_center.0 < to_center.0 {
            let sx = from.2 + 1;
            let ex = to.0.saturating_sub(1);
            ((sx, shared_y), (ex, shared_y), 'R', 'L')
        } else {
            let sx = from.0.saturating_sub(1);
            let ex = to.2 + 1;
            ((sx, shared_y), (ex, shared_y), 'L', 'R')
        }
    } else {
        let sx = (from.0 + from.2) / 2;
        let ex = (to.0 + to.2) / 2;
        if from_center.1 < to_center.1 {
            let sy = from.3 + 1;
            let ey = to.1.saturating_sub(1);
            ((sx, sy), (ex, ey), 'D', 'U')
        } else {
            let sy = from.1.saturating_sub(1);
            let ey = to.3 + 1;
            ((sx, sy), (ex, ey), 'U', 'D')
        }
    };

    let dotted = rel.dotted
        || matches!(
            rel.kind,
            RelationKind::Dependency | RelationKind::Realization
        );
    let dot_char = if matches!(charset, CharsetKind::Unicode) {
        '┈'
    } else {
        '.'
    };
    let h_char = if dotted { dot_char } else { cs.horiz() };
    let v_char = if dotted {
        if matches!(charset, CharsetKind::Unicode) {
            '┊'
        } else {
            ':'
        }
    } else {
        cs.vert()
    };

    if same_row {
        let (lo, hi) = if start.0 <= end.0 {
            (start.0, end.0)
        } else {
            (end.0, start.0)
        };
        for x in lo..=hi {
            if canvas.get(x, start.1) == ' ' {
                canvas.put_char(x, start.1, h_char);
            }
        }
        canvas.paint_hline(lo, hi, start.1, Color::Yellow);
    } else {
        let mid_y = (start.1 + end.1) / 2;
        let (yl, yh) = if start.1 <= end.1 {
            (start.1, mid_y)
        } else {
            (mid_y, start.1)
        };
        for y in yl..=yh {
            if canvas.get(start.0, y) == ' ' {
                canvas.put_char(start.0, y, v_char);
            }
        }
        canvas.paint_vline(start.0, yl, yh, Color::Yellow);
        let (xl, xh) = if start.0 <= end.0 {
            (start.0, end.0)
        } else {
            (end.0, start.0)
        };
        for x in xl..=xh {
            if canvas.get(x, mid_y) == ' ' {
                canvas.put_char(x, mid_y, h_char);
            }
        }
        canvas.paint_hline(xl, xh, mid_y, Color::Yellow);
        let (yl2, yh2) = if mid_y <= end.1 {
            (mid_y, end.1)
        } else {
            (end.1, mid_y)
        };
        for y in yl2..=yh2 {
            if canvas.get(end.0, y) == ' ' {
                canvas.put_char(end.0, y, v_char);
            }
        }
        canvas.paint_vline(end.0, yl2, yh2, Color::Yellow);
        if !dotted && matches!(charset, CharsetKind::Unicode) && start.0 != end.0 {
            let (bend1, bend2) = match (start.1.cmp(&end.1), start.0.cmp(&end.0)) {
                (std::cmp::Ordering::Less, std::cmp::Ordering::Greater) => {
                    (cs.bot_right(), cs.top_left())
                }
                (std::cmp::Ordering::Less, std::cmp::Ordering::Less) => {
                    (cs.bot_left(), cs.top_right())
                }
                (std::cmp::Ordering::Greater, std::cmp::Ordering::Greater) => {
                    (cs.top_right(), cs.bot_left())
                }
                (std::cmp::Ordering::Greater, std::cmp::Ordering::Less) => {
                    (cs.top_left(), cs.bot_right())
                }
                _ => (cs.cross(), cs.cross()),
            };
            canvas.put_char(start.0, mid_y, bend1);
            canvas.put_char(end.0, mid_y, bend2);
            canvas.set_color(start.0, mid_y, Color::Yellow);
            canvas.set_color(end.0, mid_y, Color::Yellow);
        }
    }

    let from_marker = marker_for(rel.kind, charset, true, orient_start);
    let to_marker = marker_for(rel.kind, charset, false, orient_end);
    if let Some(m) = from_marker {
        canvas.put_char(start.0, start.1, m);
        canvas.set_color(start.0, start.1, Color::Magenta);
    }
    if let Some(m) = to_marker {
        canvas.put_char(end.0, end.1, m);
        canvas.set_color(end.0, end.1, Color::Magenta);
    }

    if !rel.label.is_empty() {
        let mid_x = (start.0 + end.0) / 2;
        let mid_y = (start.1 + end.1) / 2;
        let lw = UnicodeWidthStr::width(rel.label.as_str());
        let lx = mid_x.saturating_sub(lw / 2);
        let ly = if same_row {
            mid_y.saturating_sub(1).max(1)
        } else {
            mid_y
        };
        canvas.put_str(lx, ly, &rel.label);
        canvas.paint_text(lx, ly, &rel.label, Color::Green);
    }
    if !rel.from_card.is_empty() {
        let (lx, ly) = card_pos(start, orient_start, &rel.from_card, from);
        let card_text = format!("\"{}\"", rel.from_card);
        canvas.put_str(lx, ly, &card_text);
        canvas.paint_text(lx, ly, &card_text, Color::Green);
    }
    if !rel.to_card.is_empty() {
        let (lx, ly) = card_pos(end, orient_end, &rel.to_card, to);
        let card_text = format!("\"{}\"", rel.to_card);
        canvas.put_str(lx, ly, &card_text);
        canvas.paint_text(lx, ly, &card_text, Color::Green);
    }
}

fn card_pos(
    _p: (usize, usize),
    orient: char,
    card: &str,
    box_rect: (usize, usize, usize, usize),
) -> (usize, usize) {
    let quoted_w = UnicodeWidthStr::width(card) + 2;
    let above = box_rect.1.saturating_sub(1);
    let below = box_rect.3 + 1;
    match orient {
        'R' => (box_rect.2.saturating_sub(quoted_w), above),
        'L' => (box_rect.0 + 1, above),
        'D' => (box_rect.2.saturating_sub(quoted_w), below),
        'U' => (box_rect.0 + 1, below),
        _ => (box_rect.0, above),
    }
}

fn marker_for(
    kind: RelationKind,
    charset: CharsetKind,
    is_source: bool,
    orient: char,
) -> Option<char> {
    let unicode = matches!(charset, CharsetKind::Unicode);
    match kind {
        RelationKind::Inheritance | RelationKind::Realization => {
            if is_source {
                Some(match orient {
                    'R' => {
                        if unicode {
                            '◁'
                        } else {
                            '<'
                        }
                    }
                    'L' => {
                        if unicode {
                            '▷'
                        } else {
                            '>'
                        }
                    }
                    'D' => {
                        if unicode {
                            '△'
                        } else {
                            '^'
                        }
                    }
                    'U' => {
                        if unicode {
                            '▽'
                        } else {
                            'v'
                        }
                    }
                    _ => '?',
                })
            } else {
                None
            }
        }
        RelationKind::Composition => {
            if is_source {
                Some(if unicode { '◆' } else { '*' })
            } else {
                None
            }
        }
        RelationKind::Aggregation => {
            if is_source {
                Some(if unicode { '◇' } else { 'o' })
            } else {
                None
            }
        }
        RelationKind::Association | RelationKind::Dependency => {
            if !is_source {
                Some(match orient {
                    'L' => {
                        if unicode {
                            '◁'
                        } else {
                            '<'
                        }
                    }
                    'R' => {
                        if unicode {
                            '▷'
                        } else {
                            '>'
                        }
                    }
                    'U' => {
                        if unicode {
                            '△'
                        } else {
                            '^'
                        }
                    }
                    'D' => {
                        if unicode {
                            '▽'
                        } else {
                            'v'
                        }
                    }
                    _ => '?',
                })
            } else {
                None
            }
        }
        RelationKind::Link => None,
    }
}

#[cfg(test)]
mod tests {
    fn render_s(src: &str) -> String {
        crate::mermaid::ascii::class::render(src, 200, crate::mermaid::ascii::CharsetKind::Unicode)
            .unwrap()
            .join("\n")
    }

    fn render_a(src: &str) -> String {
        crate::mermaid::ascii::class::render(src, 200, crate::mermaid::ascii::CharsetKind::Ascii)
            .unwrap()
            .join("\n")
    }

    #[test]
    fn render_single_empty_class() {
        insta::assert_snapshot!(render_s("classDiagram\nclass Foo"));
    }

    #[test]
    fn render_class_with_attributes() {
        insta::assert_snapshot!(render_s(
            "classDiagram\nclass Foo {\n  +String name\n  -int age\n}"
        ));
    }

    #[test]
    fn render_class_with_methods() {
        insta::assert_snapshot!(render_s(
            "classDiagram\nclass Foo {\n  +greet()\n  +shout() String\n}"
        ));
    }

    #[test]
    fn render_class_with_attributes_and_methods() {
        insta::assert_snapshot!(render_s(
            "classDiagram\nclass Foo {\n  +String name\n  +greet()\n}"
        ));
    }

    #[test]
    fn render_class_with_visibility_markers() {
        insta::assert_snapshot!(render_s(
            "classDiagram\nclass Foo {\n  +pub\n  -priv\n  #prot\n  ~pkg\n}"
        ));
    }

    #[test]
    fn render_class_with_annotation_interface() {
        insta::assert_snapshot!(render_s(
            "classDiagram\nclass Foo {\n  <<interface>>\n  +do()\n}"
        ));
    }

    #[test]
    fn render_class_generic() {
        insta::assert_snapshot!(render_s("classDiagram\nclass List~T~"));
    }

    #[test]
    fn render_two_classes_inheritance() {
        insta::assert_snapshot!(render_s(
            "classDiagram\nclass Animal\nclass Dog\nAnimal <|-- Dog"
        ));
    }

    #[test]
    fn render_two_classes_composition() {
        insta::assert_snapshot!(render_s(
            "classDiagram\nclass Car\nclass Engine\nCar *-- Engine"
        ));
    }

    #[test]
    fn render_two_classes_aggregation() {
        insta::assert_snapshot!(render_s(
            "classDiagram\nclass Team\nclass Player\nTeam o-- Player"
        ));
    }

    #[test]
    fn render_two_classes_association_with_label() {
        insta::assert_snapshot!(render_s("classDiagram\nclass A\nclass B\nA --> B : uses"));
    }

    #[test]
    fn render_two_classes_with_cardinality() {
        insta::assert_snapshot!(render_s(
            "classDiagram\nclass A\nclass B\nA \"1\" --> \"*\" B : has"
        ));
    }

    #[test]
    fn render_dependency_dotted_arrow() {
        insta::assert_snapshot!(render_s("classDiagram\nclass A\nclass B\nA ..> B"));
    }

    #[test]
    fn render_realization_dotted_inheritance() {
        insta::assert_snapshot!(render_s("classDiagram\nclass I\nclass C\nI <|.. C"));
    }

    #[test]
    fn render_three_class_chain() {
        insta::assert_snapshot!(render_s(
            "classDiagram\nclass A\nclass B\nclass C\nA --> B\nB --> C"
        ));
    }

    #[test]
    fn render_class_diamond_inheritance() {
        insta::assert_snapshot!(render_s(
            "classDiagram\nclass A\nclass B\nclass C\nclass D\nA <|-- B\nA <|-- C\nB <|-- D\nC <|-- D"
        ));
    }

    #[test]
    fn render_long_class_name_widens_box() {
        insta::assert_snapshot!(render_s("classDiagram\nclass AVeryLongClassName"));
    }

    #[test]
    fn render_ascii_charset() {
        insta::assert_snapshot!(render_a("classDiagram\nclass A\nclass B\nA --> B"));
    }

    #[test]
    fn render_each_class_has_three_sections() {
        let out = render_s("classDiagram\nclass Foo {\n  +String name\n  +greet()\n}");
        let divider_count = out.matches('├').count() + out.matches('┤').count();
        assert!(divider_count >= 2, "expected dividers in output:\n{out}");
    }

    #[test]
    fn render_arrow_markers_inheritance_open_triangle() {
        let out = render_s("classDiagram\nclass A\nclass B\nA <|-- B");
        assert!(
            out.contains('◁') || out.contains('▷') || out.contains('△') || out.contains('▽'),
            "expected open triangle marker:\n{out}"
        );
    }

    #[test]
    fn render_arrow_markers_composition_filled_diamond() {
        let out = render_s("classDiagram\nclass A\nclass B\nA *-- B");
        assert!(out.contains('◆'), "expected filled diamond:\n{out}");
    }

    #[test]
    fn render_arrow_markers_aggregation_open_diamond() {
        let out = render_s("classDiagram\nclass A\nclass B\nA o-- B");
        assert!(out.contains('◇'), "expected open diamond:\n{out}");
    }

    #[test]
    fn render_dependency_line_is_dotted() {
        let out = render_s("classDiagram\nclass A\nclass B\nA ..> B");
        assert!(
            out.contains('┈') || out.contains('┊'),
            "expected dotted line glyphs:\n{out}"
        );
    }

    #[test]
    fn render_cardinality_appears_near_endpoints() {
        let out = render_s("classDiagram\nclass A\nclass B\nA \"1\" --> \"*\" B");
        assert!(out.contains("\"1\""), "expected '1' card:\n{out}");
        assert!(out.contains("\"*\""), "expected '*' card:\n{out}");
    }

    #[test]
    fn render_styled_attaches_color_to_borders() {
        let rows = crate::mermaid::ascii::class::render_styled(
            "classDiagram\nclass Foo {\n  +String name\n  +greet()\n}",
            100,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap();
        assert!(rows
            .iter()
            .flatten()
            .any(|r| r.color == Some(ratatui::style::Color::Cyan)));
    }

    #[test]
    fn render_styled_has_multiple_colors() {
        let rows = crate::mermaid::ascii::class::render_styled(
            "classDiagram\nclass Foo {\n  +String name\n  +greet()\n}\nclass Bar\nFoo --> Bar : uses",
            100,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap();
        let colors: std::collections::HashSet<_> = rows.iter().flatten().map(|r| r.color).collect();
        assert!(
            colors.len() >= 2,
            "expected at least 2 distinct colors, got {colors:?}"
        );
    }

    #[test]
    fn render_styled_specific_glyph_colored() {
        let rows = crate::mermaid::ascii::class::render_styled(
            "classDiagram\nclass A\nclass B\nA *-- B",
            100,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap();
        let diamond_color = rows
            .iter()
            .flatten()
            .find(|r| r.text.contains('◆'))
            .map(|r| r.color);
        assert_eq!(
            diamond_color,
            Some(Some(ratatui::style::Color::Magenta)),
            "composition diamond should be magenta"
        );
    }
}
