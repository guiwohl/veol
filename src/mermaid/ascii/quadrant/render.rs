use super::ast::QuadrantChart;
use crate::mermaid::ascii::{AsciiRenderError, Canvas, CharsetKind, StyledRow};
use ratatui::style::Color;
use unicode_width::UnicodeWidthStr;

const MIN_WIDTH: usize = 40;
const Y_LABEL_COL: usize = 16;
const RIGHT_PAD: usize = 2;
const PLOT_HEIGHT: usize = 18;

pub fn render(
    diag: &QuadrantChart,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    let (title_line, canvas) = render_to_canvas(diag, max_width, charset)?;
    let mut out: Vec<String> = Vec::new();
    if let Some(t) = title_line {
        out.push(t);
        out.push(String::new());
    }
    for line in canvas.into_lines() {
        out.push(line);
    }
    while out.last().map(|l| l.is_empty()).unwrap_or(false) {
        out.pop();
    }
    Ok(out)
}

pub fn render_styled(
    diag: &QuadrantChart,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    let (title_line, canvas) = render_to_canvas(diag, max_width, charset)?;
    let mut out: Vec<StyledRow> = Vec::new();
    if let Some(t) = title_line {
        out.push(crate::mermaid::ascii::styled_row(t, Color::White));
        out.push(Vec::new());
    }
    for row in canvas.into_styled_lines() {
        out.push(row);
    }
    while out.last().map(|r| r.is_empty()).unwrap_or(false) {
        out.pop();
    }
    Ok(out)
}

fn render_to_canvas(
    diag: &QuadrantChart,
    max_width: u16,
    charset: CharsetKind,
) -> Result<(Option<String>, Canvas), AsciiRenderError> {
    let total_width = (max_width as usize).max(MIN_WIDTH);
    let plot_left = Y_LABEL_COL;
    let plot_width = total_width
        .saturating_sub(plot_left)
        .saturating_sub(RIGHT_PAD)
        .max(20);
    let plot_right = plot_left + plot_width - 1;
    let plot_height = PLOT_HEIGHT;

    let title_line = diag.title.as_ref().map(|t| center_in(t, total_width));

    let mut canvas = Canvas::new(total_width, plot_height + 3, charset);

    let plot_top = 0usize;
    let plot_bot = plot_top + plot_height - 1;
    let cross_x = plot_left + plot_width / 2;
    let cross_y = plot_top + plot_height / 2;

    canvas.draw_hline(plot_left, plot_right, cross_y);
    canvas.paint_hline(plot_left, plot_right, cross_y, Color::DarkGray);
    canvas.draw_vline(cross_x, plot_top, plot_bot);
    canvas.paint_vline(cross_x, plot_top, plot_bot, Color::DarkGray);

    let cs = crate::mermaid::ascii::charset::get_charset(charset);
    canvas.put_char(cross_x, cross_y, cs.cross());
    canvas.set_color(cross_x, cross_y, Color::DarkGray);

    let q1_x_center = plot_left + (plot_width * 3 / 4);
    let q2_x_center = plot_left + (plot_width / 4);
    let q3_x_center = plot_left + (plot_width / 4);
    let q4_x_center = plot_left + (plot_width * 3 / 4);
    let top_label_y = plot_top + 1;
    let bot_label_y = cross_y + 1;

    if let Some(l) = &diag.q1_label {
        place_centered_colored(&mut canvas, q1_x_center, top_label_y, l, Color::Blue);
    }
    if let Some(l) = &diag.q2_label {
        place_centered_colored(&mut canvas, q2_x_center, top_label_y, l, Color::Blue);
    }
    if let Some(l) = &diag.q3_label {
        place_centered_colored(&mut canvas, q3_x_center, bot_label_y, l, Color::Blue);
    }
    if let Some(l) = &diag.q4_label {
        place_centered_colored(&mut canvas, q4_x_center, bot_label_y, l, Color::Blue);
    }

    let point_char = match charset {
        CharsetKind::Unicode => '●',
        CharsetKind::Ascii => '*',
    };

    for p in &diag.points {
        let x = p.x.clamp(0.0, 1.0);
        let y = p.y.clamp(0.0, 1.0);
        let px = plot_left + ((x * (plot_width - 1) as f64).round() as usize).min(plot_width - 1);
        let py = plot_top
            + (((1.0 - y) * (plot_height - 1) as f64).round() as usize).min(plot_height - 1);
        canvas.put_char(px, py, point_char);
        canvas.set_color(px, py, Color::Magenta);
        let lbl_x = px + 2;
        if lbl_x < total_width {
            canvas.put_str(lbl_x, py, &p.label);
        }
    }

    if let Some(t) = &diag.y_axis_top {
        let y = plot_top;
        let x = plot_left.saturating_sub(UnicodeWidthStr::width(t.as_str()) + 1);
        canvas.put_str_colored(x, y, t, Color::Cyan);
    }
    if let Some(b) = &diag.y_axis_bottom {
        let y = plot_bot;
        let x = plot_left.saturating_sub(UnicodeWidthStr::width(b.as_str()) + 1);
        canvas.put_str_colored(x, y, b, Color::Cyan);
    }

    let axis_y = plot_bot + 2;
    if let Some(l) = &diag.x_axis_left {
        canvas.put_str_colored(plot_left, axis_y, l, Color::Cyan);
    }
    if let Some(r) = &diag.x_axis_right {
        let w = UnicodeWidthStr::width(r.as_str());
        let x = plot_right.saturating_sub(w.saturating_sub(1));
        canvas.put_str_colored(x, axis_y, r, Color::Cyan);
    }

    Ok((title_line, canvas))
}

fn place_centered_colored(canvas: &mut Canvas, cx: usize, y: usize, text: &str, color: Color) {
    let w = UnicodeWidthStr::width(text);
    let half = w / 2;
    let x = cx.saturating_sub(half);
    canvas.put_str_colored(x, y, text, color);
}

fn center_in(s: &str, width: usize) -> String {
    let w = UnicodeWidthStr::width(s);
    if w >= width {
        return s.to_string();
    }
    let left = (width - w) / 2;
    let mut out = " ".repeat(left);
    out.push_str(s);
    out
}

#[cfg(test)]
mod tests {
    use crate::mermaid::ascii::StyledRun;
    use ratatui::style::Color;

    fn render(src: &str) -> String {
        crate::mermaid::ascii::quadrant::render(
            src,
            80,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap()
        .join("\n")
    }

    fn render_ascii(src: &str) -> String {
        crate::mermaid::ascii::quadrant::render(src, 80, crate::mermaid::ascii::CharsetKind::Ascii)
            .unwrap()
            .join("\n")
    }

    fn render_styled(src: &str) -> Vec<Vec<StyledRun>> {
        crate::mermaid::ascii::quadrant::render_styled(
            src,
            80,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap()
    }

    #[test]
    fn render_quadrant_minimal() {
        let src = "quadrantChart";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_with_title() {
        let src = "quadrantChart\n    title Reach and engagement of campaigns";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_with_axes() {
        let src = "quadrantChart\n    x-axis Low Reach --> High Reach\n    y-axis Low Engagement --> High Engagement";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_with_quadrant_labels() {
        let src = "quadrantChart\n    quadrant-1 We should expand\n    quadrant-2 Need to promote\n    quadrant-3 Re-evaluate\n    quadrant-4 May be improved";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_with_points() {
        let src = "quadrantChart\n    Campaign A: [0.3, 0.6]\n    Campaign B: [0.45, 0.23]";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_full() {
        let src = "quadrantChart\n    title Reach and engagement of campaigns\n    x-axis Low Reach --> High Reach\n    y-axis Low Engagement --> High Engagement\n    quadrant-1 We should expand\n    quadrant-2 Need to promote\n    quadrant-3 Re-evaluate\n    quadrant-4 May be improved\n    Campaign A: [0.3, 0.6]\n    Campaign B: [0.45, 0.23]\n    Campaign C: [0.57, 0.69]\n    Campaign D: [0.78, 0.34]";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_ascii_charset() {
        let src = "quadrantChart\n    title ASCII Mode\n    x-axis Low --> High\n    y-axis Low --> High\n    quadrant-1 Q1\n    quadrant-2 Q2\n    quadrant-3 Q3\n    quadrant-4 Q4\n    A: [0.7, 0.7]\n    B: [0.2, 0.2]";
        insta::assert_snapshot!(render_ascii(src));
    }

    #[test]
    fn render_has_four_quadrants() {
        let src = "quadrantChart\n    quadrant-1 Q1\n    quadrant-2 Q2\n    quadrant-3 Q3\n    quadrant-4 Q4";
        let out = render(src);
        assert!(out.contains("Q1"), "missing Q1:\n{out}");
        assert!(out.contains("Q2"), "missing Q2:\n{out}");
        assert!(out.contains("Q3"), "missing Q3:\n{out}");
        assert!(out.contains("Q4"), "missing Q4:\n{out}");
    }

    #[test]
    fn render_cross_axis_visible() {
        let src = "quadrantChart";
        let out = render(src);
        assert!(out.contains('┼'), "expected cross junction, out=\n{out}");
        assert!(out.contains('─'), "expected horiz axis, out=\n{out}");
        assert!(out.contains('│'), "expected vert axis, out=\n{out}");
    }

    #[test]
    fn render_points_placed_within_grid() {
        let src = "quadrantChart\n    Campaign A: [0.3, 0.6]\n    Campaign B: [0.8, 0.2]";
        let out = render(src);
        let dot_count = out.chars().filter(|&c| c == '●').count();
        assert_eq!(
            dot_count, 2,
            "expected 2 point dots, got {dot_count}:\n{out}"
        );
        assert!(out.contains("Campaign A"), "missing label A:\n{out}");
        assert!(out.contains("Campaign B"), "missing label B:\n{out}");
    }

    #[test]
    fn render_axis_labels_visible() {
        let src = "quadrantChart\n    x-axis Low Reach --> High Reach\n    y-axis Low Engagement --> High Engagement";
        let out = render(src);
        assert!(out.contains("Low Reach"), "missing x-left:\n{out}");
        assert!(out.contains("High Reach"), "missing x-right:\n{out}");
        assert!(out.contains("Low Engagement"), "missing y-bot:\n{out}");
        assert!(out.contains("High Engagement"), "missing y-top:\n{out}");
    }

    #[test]
    fn render_styled_attaches_colors() {
        let src = "quadrantChart\n    title T\n    x-axis Low --> High\n    Campaign A: [0.3, 0.6]";
        let rows = render_styled(src);
        let any_colored = rows
            .iter()
            .flat_map(|r| r.iter())
            .any(|run| run.color.is_some());
        assert!(any_colored, "expected colored runs");
    }

    #[test]
    fn render_styled_specific_element_colored() {
        let src = "quadrantChart\n    title MyTitle\n    x-axis LowReach --> HighReach\n    quadrant-1 Q1Lab\n    A: [0.3, 0.6]";
        let rows = render_styled(src);
        // Title (white)
        let white_text: String = rows
            .iter()
            .flat_map(|r| r.iter())
            .filter(|r| r.color == Some(Color::White))
            .map(|r| r.text.as_str())
            .collect();
        assert!(white_text.contains("MyTitle"), "expected white title");
        // Axis labels (cyan)
        let cyan_text: String = rows
            .iter()
            .flat_map(|r| r.iter())
            .filter(|r| r.color == Some(Color::Cyan))
            .map(|r| r.text.as_str())
            .collect();
        assert!(
            cyan_text.contains("LowReach") || cyan_text.contains("HighReach"),
            "expected cyan axis labels, got {cyan_text:?}"
        );
        // Quadrant label (blue)
        let blue_text: String = rows
            .iter()
            .flat_map(|r| r.iter())
            .filter(|r| r.color == Some(Color::Blue))
            .map(|r| r.text.as_str())
            .collect();
        assert!(
            blue_text.contains("Q1Lab"),
            "expected blue quadrant label, got {blue_text:?}"
        );
        // Point ● (magenta)
        let magenta_text: String = rows
            .iter()
            .flat_map(|r| r.iter())
            .filter(|r| r.color == Some(Color::Magenta))
            .map(|r| r.text.as_str())
            .collect();
        assert!(
            magenta_text.contains('●'),
            "expected magenta point glyph, got {magenta_text:?}"
        );
    }

    #[test]
    fn render_styled_no_alignment_drift() {
        let src = "quadrantChart\n    title MyTitle\n    x-axis Low --> High\n    y-axis Low --> High\n    quadrant-1 Q1\n    A: [0.3, 0.6]";
        let plain = crate::mermaid::ascii::quadrant::render(
            src,
            80,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap();
        let styled = render_styled(src);
        assert_eq!(plain.len(), styled.len(), "row count");
        for (i, row) in styled.iter().enumerate() {
            let flat: String = row.iter().map(|r| r.text.as_str()).collect();
            let flat_trimmed = flat.trim_end().to_string();
            assert_eq!(flat_trimmed, plain[i], "row {i} drift");
        }
    }
}
