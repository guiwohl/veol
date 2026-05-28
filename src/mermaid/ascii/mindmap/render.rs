use super::ast::{MindNode, Mindmap, NodeShape};
use crate::mermaid::ascii::canvas::Canvas;
use crate::mermaid::ascii::{AsciiRenderError, CharsetKind, StyledRow};
use ratatui::style::Color;
use unicode_width::UnicodeWidthStr;

pub fn render(
    diag: &Mindmap,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_lines())
}

pub fn render_styled(
    diag: &Mindmap,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_styled_lines())
}

fn render_to_canvas(
    diag: &Mindmap,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    let root = diag
        .root
        .as_ref()
        .ok_or_else(|| AsciiRenderError::Layout("mindmap has no root".into()))?;

    let glyphs = Glyphs::for_charset(charset);
    let max_w = (max_width as usize).max(20);

    let mut lines: Vec<LineSpec> = Vec::new();

    let root_marker = glyphs.shape_marker(root.shape);
    let root_label = display_label(root);
    let root_text = format!("{root_marker} {root_label}");
    let root_truncated = truncate_line(&root_text, max_w);
    // Marker glyph occupies 1 string char even when visually wide; the canvas
    // stores it at col 0 and the following label chars at col 1, 2, ....
    lines.push(LineSpec {
        text: root_truncated.clone(),
        depth: 0,
        marker_start: 0,
        marker_width: 1,
        label_start: 2,
        branch_ranges: Vec::new(),
    });

    let mut prefix_stack: Vec<bool> = Vec::new();
    let n = root.children.len();
    for (i, child) in root.children.iter().enumerate() {
        let is_last = i + 1 == n;
        render_node(
            child,
            1,
            &mut prefix_stack,
            is_last,
            &glyphs,
            max_w,
            &mut lines,
        );
    }

    let canvas_w = lines
        .iter()
        .map(|l| l.text.chars().count())
        .max()
        .unwrap_or(0)
        .max(1);
    let canvas_h = lines.len().max(1);
    let mut canvas = Canvas::new(canvas_w, canvas_h, charset);

    for (y, line) in lines.iter().enumerate() {
        put_chars_one_per_cell(&mut canvas, 0, y, &line.text);
        for &(bx1, bx2) in &line.branch_ranges {
            canvas.paint_hline(bx1, bx2, y, Color::DarkGray);
        }
        if line.marker_width > 0 {
            canvas.paint_hline(
                line.marker_start,
                line.marker_start + line.marker_width - 1,
                y,
                Color::Magenta,
            );
        }
        let text_chars = line.text.chars().count();
        if line.label_start < text_chars {
            let label_color = depth_color(line.depth);
            canvas.paint_hline(
                line.label_start,
                text_chars.saturating_sub(1),
                y,
                label_color,
            );
        }
    }

    Ok(canvas)
}

fn put_chars_one_per_cell(canvas: &mut Canvas, x: usize, y: usize, s: &str) {
    for (i, ch) in s.chars().enumerate() {
        canvas.put_char(x + i, y, ch);
    }
}

struct LineSpec {
    text: String,
    depth: usize,
    marker_start: usize,
    marker_width: usize,
    label_start: usize,
    branch_ranges: Vec<(usize, usize)>,
}

fn depth_color(depth: usize) -> Color {
    match depth {
        0 => Color::Yellow,
        1 => Color::Cyan,
        2 => Color::Green,
        3 => Color::Magenta,
        _ => Color::Blue,
    }
}

fn render_node(
    node: &MindNode,
    depth: usize,
    prefix_stack: &mut Vec<bool>,
    is_last: bool,
    glyphs: &Glyphs,
    max_w: usize,
    out: &mut Vec<LineSpec>,
) {
    let mut prefix = String::new();
    let mut branch_ranges: Vec<(usize, usize)> = Vec::new();
    let mut col: usize = 0;
    for &parent_is_last in prefix_stack.iter() {
        if parent_is_last {
            prefix.push_str("    ");
            col += 4;
        } else {
            prefix.push(glyphs.vert);
            branch_ranges.push((col, col));
            prefix.push_str("   ");
            col += 4;
        }
    }

    let connector = if is_last {
        glyphs.connector_last
    } else {
        glyphs.connector_mid
    };

    let connector_w = connector.chars().count();
    let horiz_w = glyphs.horiz_run.chars().count();
    let branch_start = col;
    let branch_end = col + connector_w + horiz_w - 1;
    branch_ranges.push((branch_start, branch_end));
    col += connector_w + horiz_w;
    // space
    col += 1;

    let marker = glyphs.shape_marker(node.shape);
    let marker_start = col;
    let marker_w = 1usize;
    col += marker_w + 1;
    let label_start = col;

    let line = format!(
        "{prefix}{connector}{horiz} {marker} {label}",
        horiz = glyphs.horiz_run,
        marker = marker,
        label = display_label(node),
    );
    let truncated = truncate_line(&line, max_w);

    let truncated_chars = truncated.chars().count();
    let filtered_branches: Vec<(usize, usize)> = branch_ranges
        .into_iter()
        .filter(|(x1, _)| *x1 < truncated_chars)
        .map(|(x1, x2)| (x1, x2.min(truncated_chars.saturating_sub(1))))
        .collect();
    let marker_in_range = marker_start < truncated_chars;

    out.push(LineSpec {
        text: truncated,
        depth,
        marker_start: if marker_in_range { marker_start } else { 0 },
        marker_width: if marker_in_range { marker_w } else { 0 },
        label_start,
        branch_ranges: filtered_branches,
    });

    prefix_stack.push(is_last);
    let n = node.children.len();
    for (i, child) in node.children.iter().enumerate() {
        let last = i + 1 == n;
        render_node(child, depth + 1, prefix_stack, last, glyphs, max_w, out);
    }
    prefix_stack.pop();
}

fn display_label(node: &MindNode) -> String {
    if let Some(icon) = &node.icon {
        format!("[{}] {}", icon, node.label)
    } else {
        node.label.clone()
    }
}

fn truncate_line(line: &str, max_w: usize) -> String {
    let w = UnicodeWidthStr::width(line);
    if w <= max_w {
        return line.to_string();
    }
    if max_w == 0 {
        return String::new();
    }
    let mut acc = String::new();
    let mut used = 0;
    for c in line.chars() {
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

struct Glyphs {
    vert: char,
    horiz_run: &'static str,
    connector_mid: &'static str,
    connector_last: &'static str,
    unicode: bool,
}

impl Glyphs {
    fn for_charset(kind: CharsetKind) -> Self {
        match kind {
            CharsetKind::Unicode => Glyphs {
                vert: '│',
                horiz_run: "──",
                connector_mid: "├",
                connector_last: "└",
                unicode: true,
            },
            CharsetKind::Ascii => Glyphs {
                vert: '|',
                horiz_run: "--",
                connector_mid: "|",
                connector_last: "`",
                unicode: false,
            },
        }
    }

    fn shape_marker(&self, shape: NodeShape) -> char {
        if self.unicode {
            match shape {
                NodeShape::Default => '·',
                NodeShape::Square => '■',
                NodeShape::Round => '●',
                NodeShape::Circle => '●',
                NodeShape::Cloud => '☁',
                NodeShape::Bang => '❗',
                NodeShape::Hexagon => '⬡',
            }
        } else {
            match shape {
                NodeShape::Default => '.',
                NodeShape::Square => '#',
                NodeShape::Round => 'o',
                NodeShape::Circle => 'O',
                NodeShape::Cloud => '~',
                NodeShape::Bang => '!',
                NodeShape::Hexagon => '*',
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mermaid::ascii::StyledRun;
    use ratatui::style::Color;

    fn render(s: &str) -> String {
        crate::mermaid::ascii::mindmap::render(s, 100, crate::mermaid::ascii::CharsetKind::Unicode)
            .unwrap()
            .join("\n")
    }

    fn render_ascii(s: &str) -> String {
        crate::mermaid::ascii::mindmap::render(s, 100, crate::mermaid::ascii::CharsetKind::Ascii)
            .unwrap()
            .join("\n")
    }

    fn render_styled(s: &str) -> Vec<Vec<StyledRun>> {
        crate::mermaid::ascii::mindmap::render_styled(
            s,
            100,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap()
    }

    #[test]
    fn render_root_only() {
        let src = "mindmap\n  root";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_two_children() {
        let src = "mindmap\n  root\n    A\n    B";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_three_levels() {
        let src =
            "mindmap\n  root\n    Origins\n      Long history\n      Popularisation\n    Research";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_circle_root() {
        let src = "mindmap\n  root((mindmap))\n    A\n    B";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_square_node() {
        let src = "mindmap\n  root\n    box[Square Node]\n    plain";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_cloud_node() {
        let src = "mindmap\n  root\n    c)Cloud thing(";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_bang_node() {
        let src = "mindmap\n  root\n    b))Important((";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_long_label_wraps() {
        let src = "mindmap\n  root\n    On effectiveness<br/>and features";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_ascii_charset() {
        let src = "mindmap\n  root((m))\n    A\n      B\n    C";
        insta::assert_snapshot!(render_ascii(src));
    }

    #[test]
    fn render_root_at_top() {
        let src = "mindmap\n  root\n    child";
        let out = crate::mermaid::ascii::mindmap::render(
            src,
            100,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap();
        assert!(
            out[0].contains("root"),
            "root should be on first line, got:\n{}",
            out.join("\n")
        );
    }

    #[test]
    fn render_branch_chars_present() {
        let src = "mindmap\n  root\n    A\n    B\n    C";
        let out = crate::mermaid::ascii::mindmap::render(
            src,
            100,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap()
        .join("\n");
        assert!(
            out.contains('├') || out.contains('└'),
            "expected branch chars, got:\n{out}"
        );
    }

    #[test]
    fn render_indentation_proportional_to_depth() {
        let src = "mindmap\n  root\n    A\n      B\n        C";
        let out = crate::mermaid::ascii::mindmap::render(
            src,
            100,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap();
        let pos_a = out
            .iter()
            .find_map(|l| l.find('A').map(|i| (l.clone(), i)))
            .unwrap();
        let pos_b = out
            .iter()
            .find_map(|l| l.find('B').map(|i| (l.clone(), i)))
            .unwrap();
        let pos_c = out
            .iter()
            .find_map(|l| l.find('C').map(|i| (l.clone(), i)))
            .unwrap();
        assert!(
            pos_a.1 < pos_b.1,
            "A col {} should be < B col {}\n{}",
            pos_a.1,
            pos_b.1,
            out.join("\n")
        );
        assert!(
            pos_b.1 < pos_c.1,
            "B col {} should be < C col {}\n{}",
            pos_b.1,
            pos_c.1,
            out.join("\n")
        );
    }

    #[test]
    fn render_shape_glyphs_unicode_vs_ascii_differ() {
        let src = "mindmap\n  root((R))\n    A";
        let u = crate::mermaid::ascii::mindmap::render(
            src,
            100,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap()
        .join("\n");
        let a = crate::mermaid::ascii::mindmap::render(
            src,
            100,
            crate::mermaid::ascii::CharsetKind::Ascii,
        )
        .unwrap()
        .join("\n");
        assert_ne!(
            u, a,
            "unicode and ascii renderings should differ\nu:\n{u}\na:\n{a}"
        );
    }

    #[test]
    fn render_styled_attaches_colors() {
        let src = "mindmap\n  root((R))\n    A\n    B";
        let rows = render_styled(src);
        let any_colored = rows
            .iter()
            .flat_map(|r| r.iter())
            .any(|run| run.color.is_some());
        assert!(any_colored, "expected at least one colored run");
    }

    #[test]
    fn render_styled_specific_element_colored() {
        let src = "mindmap\n  root((R))\n    A";
        let rows = render_styled(src);
        // Root shape glyph (●) should be magenta on row 0.
        let row0 = &rows[0];
        let magenta_text: String = row0
            .iter()
            .filter(|r| r.color == Some(Color::Magenta))
            .map(|r| r.text.as_str())
            .collect();
        assert!(
            magenta_text.contains('●'),
            "expected magenta root glyph, runs={row0:?}"
        );
        // Branch line ├── or └── should appear in DarkGray on a child row.
        let dark_present = rows.iter().flat_map(|r| r.iter()).any(|run| {
            run.color == Some(Color::DarkGray)
                && (run.text.contains('└') || run.text.contains('├') || run.text.contains('─'))
        });
        assert!(dark_present, "expected dark gray branch chars");
    }

    #[test]
    fn render_styled_no_alignment_drift() {
        let src = "mindmap\n  root((R))\n    Origins\n      History\n    Research";
        let plain = crate::mermaid::ascii::mindmap::render(
            src,
            100,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap();
        let styled = render_styled(src);
        assert_eq!(plain.len(), styled.len(), "row count mismatch");
        for (i, row) in styled.iter().enumerate() {
            let flat: String = row.iter().map(|r| r.text.as_str()).collect();
            let flat_trimmed = flat.trim_end().to_string();
            assert_eq!(flat_trimmed, plain[i], "row {i} drift");
        }
    }
}
