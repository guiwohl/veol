use super::ast::{Orientation, Series, XyChart};
use crate::mermaid::ascii::canvas::Canvas;
use crate::mermaid::ascii::{AsciiRenderError, CharsetKind, StyledRow};
use ratatui::style::Color;
use unicode_width::UnicodeWidthStr;

const MIN_PLOT_HEIGHT: usize = 8;
const DEFAULT_PLOT_HEIGHT: usize = 12;
const MIN_BAR_WIDTH: usize = 1;
const Y_LABEL_GUTTER: usize = 1;

const SERIES_CYCLE: [Color; 6] = [
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    Color::Green,
    Color::Blue,
    Color::Red,
];

pub fn render(
    diag: &XyChart,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_lines())
}

pub fn render_styled(
    diag: &XyChart,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_styled_lines())
}

fn render_to_canvas(
    diag: &XyChart,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    if diag.series.is_empty() {
        return Err(AsciiRenderError::Empty);
    }
    let orientation = diag.orientation.unwrap_or(Orientation::Vertical);
    match orientation {
        Orientation::Vertical => render_vertical(diag, max_width, charset),
        Orientation::Horizontal => render_horizontal(diag, max_width, charset),
    }
}

fn series_values(s: &Series) -> &[f64] {
    match s {
        Series::Bar(v) | Series::Line(v) => v,
    }
}

fn is_bar(s: &Series) -> bool {
    matches!(s, Series::Bar(_))
}

fn compute_y_bounds(diag: &XyChart) -> (f64, f64) {
    if let Some((lo, hi)) = diag.y_range {
        return (lo, hi);
    }
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for s in &diag.series {
        for &v in series_values(s) {
            if v < lo {
                lo = v;
            }
            if v > hi {
                hi = v;
            }
        }
    }
    if !lo.is_finite() || !hi.is_finite() {
        return (0.0, 1.0);
    }
    if (hi - lo).abs() < 1e-9 {
        return (lo - 1.0, hi + 1.0);
    }
    if lo >= 0.0 {
        lo = 0.0;
    }
    (lo, hi)
}

fn data_len(diag: &XyChart) -> usize {
    diag.series
        .iter()
        .map(|s| series_values(s).len())
        .max()
        .unwrap_or(0)
}

fn format_y_label(v: f64) -> String {
    if (v - v.round()).abs() < 1e-6 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.1}")
    }
}

fn render_vertical(
    diag: &XyChart,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    let n = data_len(diag);
    if n == 0 {
        return Err(AsciiRenderError::Empty);
    }
    let (y_min, y_max) = compute_y_bounds(diag);

    let bar_fill = match charset {
        CharsetKind::Unicode => '█',
        CharsetKind::Ascii => '#',
    };
    let line_ch = match charset {
        CharsetKind::Unicode => '─',
        CharsetKind::Ascii => '-',
    };
    let point_ch = match charset {
        CharsetKind::Unicode => '●',
        CharsetKind::Ascii => '*',
    };
    let vert = match charset {
        CharsetKind::Unicode => '│',
        CharsetKind::Ascii => '|',
    };
    let horiz = match charset {
        CharsetKind::Unicode => '─',
        CharsetKind::Ascii => '-',
    };
    let corner = match charset {
        CharsetKind::Unicode => '└',
        CharsetKind::Ascii => '+',
    };

    let y_top_label = format_y_label(y_max);
    let y_mid_label = format_y_label((y_min + y_max) / 2.0);
    let y_bot_label = format_y_label(y_min);
    let y_label_w = y_top_label
        .len()
        .max(y_mid_label.len())
        .max(y_bot_label.len());

    let total_w = (max_width as usize).max(20);
    let avail_plot = total_w
        .saturating_sub(y_label_w + Y_LABEL_GUTTER)
        .max(n * (MIN_BAR_WIDTH + 1));
    let mut bar_step = (avail_plot / n).max(MIN_BAR_WIDTH + 1);
    if bar_step < 2 {
        bar_step = 2;
    }
    let plot_w = bar_step * n;
    let plot_h = DEFAULT_PLOT_HEIGHT.max(MIN_PLOT_HEIGHT);

    // Build the plot scratch grid first (chars + per-cell color for series),
    // then transfer to canvas.
    let mut plot: Vec<Vec<char>> = vec![vec![' '; plot_w]; plot_h];
    let mut plot_color: Vec<Vec<Option<Color>>> = vec![vec![None; plot_w]; plot_h];

    let mut bar_series_idx = 0usize;
    for (s_idx, s) in diag.series.iter().enumerate() {
        if !is_bar(s) {
            continue;
        }
        let color = SERIES_CYCLE[bar_series_idx % SERIES_CYCLE.len()];
        bar_series_idx += 1;
        let _ = s_idx;
        let values = series_values(s);
        for (i, &v) in values.iter().enumerate() {
            let h = value_to_height(v, y_min, y_max, plot_h);
            let bar_x = i * bar_step + bar_step / 2;
            if bar_x >= plot_w {
                continue;
            }
            for (row_idx, row) in plot.iter_mut().enumerate().skip(plot_h - h) {
                row[bar_x] = bar_fill;
                plot_color[row_idx][bar_x] = Some(color);
            }
        }
    }

    for s in &diag.series {
        if is_bar(s) {
            continue;
        }
        let values = series_values(s);
        let mut prev: Option<(usize, usize)> = None;
        for (i, &v) in values.iter().enumerate() {
            let h = value_to_height(v, y_min, y_max, plot_h);
            let py = plot_h.saturating_sub(h.max(1));
            let px = i * bar_step + bar_step / 2;
            if px < plot_w {
                plot[py][px] = point_ch;
                plot_color[py][px] = Some(Color::Magenta);
            }
            if let Some((qx, qy)) = prev {
                draw_segment(&mut plot, &mut plot_color, qx, qy, px, py, line_ch);
            }
            prev = Some((px, py));
        }
    }

    // Now assemble canvas.
    let mut row_strs: Vec<String> = Vec::new();
    let mut row_kinds: Vec<RowKind> = Vec::new();

    if let Some(title) = &diag.title {
        row_strs.push(title.clone());
        row_kinds.push(RowKind::Title);
        row_strs.push(String::new());
        row_kinds.push(RowKind::Blank);
    }

    for (ri, row) in plot.iter().enumerate() {
        let label = if ri == 0 {
            y_top_label.clone()
        } else if ri == plot_h / 2 {
            y_mid_label.clone()
        } else if ri == plot_h - 1 {
            y_bot_label.clone()
        } else {
            String::new()
        };
        let pad = y_label_w.saturating_sub(label.len());
        let row_str: String = row.iter().collect();
        let line = format!("{}{}{}{}", " ".repeat(pad), label, vert, row_str.trim_end());
        row_strs.push(line);
        row_kinds.push(RowKind::Plot {
            ri,
            label_w: label.len(),
        });
    }

    let axis: String = std::iter::repeat_n(horiz, plot_w).collect();
    row_strs.push(format!("{}{}{}", " ".repeat(y_label_w), corner, axis));
    row_kinds.push(RowKind::Axis);

    let label_row = build_x_label_row(diag, n, bar_step);
    if !label_row.trim().is_empty() {
        row_strs.push(format!("{}{}", " ".repeat(y_label_w + 1), label_row));
        row_kinds.push(RowKind::XLabels);
    }

    if let Some(y_title) = &diag.y_axis_title {
        row_strs.push(String::new());
        row_kinds.push(RowKind::Blank);
        row_strs.push(format!("y: {y_title}"));
        row_kinds.push(RowKind::AxisTitle);
    }
    if let Some(x_title) = &diag.x_axis_title {
        if diag.y_axis_title.is_none() {
            row_strs.push(String::new());
            row_kinds.push(RowKind::Blank);
        }
        row_strs.push(format!("x: {x_title}"));
        row_kinds.push(RowKind::AxisTitle);
    }

    let canvas_w = row_strs
        .iter()
        .map(|s| UnicodeWidthStr::width(s.as_str()))
        .max()
        .unwrap_or(1)
        .max(1);
    let canvas_h = row_strs.len().max(1);
    let mut canvas = Canvas::new(canvas_w, canvas_h, charset);

    for (y, (line, kind)) in row_strs.iter().zip(row_kinds.iter()).enumerate() {
        canvas.put_str(0, y, line);
        match kind {
            RowKind::Title => {
                canvas.paint_text(0, y, line, Color::White);
            }
            RowKind::Plot { ri, label_w } => {
                if *label_w > 0 {
                    let label_start = y_label_w - *label_w;
                    let label_text = &line[label_start..label_start + label_w];
                    canvas.paint_text(label_start, y, label_text, Color::Cyan);
                }
                // axis vert char position
                canvas.set_color(y_label_w, y, Color::DarkGray);
                // plot cells: color any cell that has a color in plot_color
                for (px, cell) in plot_color[*ri].iter().enumerate().take(plot_w) {
                    if let Some(c) = *cell {
                        canvas.set_color(y_label_w + 1 + px, y, c);
                    }
                }
            }
            RowKind::Axis => {
                // corner + horizontal axis chars
                for x in y_label_w..(y_label_w + 1 + plot_w) {
                    canvas.set_color(x, y, Color::DarkGray);
                }
            }
            RowKind::XLabels => {
                let label_offset = y_label_w + 1;
                canvas.paint_text(label_offset, y, &label_row, Color::Cyan);
            }
            RowKind::AxisTitle => {
                canvas.paint_text(0, y, line, Color::Cyan);
            }
            RowKind::Blank => {}
        }
    }

    Ok(canvas)
}

enum RowKind {
    Title,
    Blank,
    Plot { ri: usize, label_w: usize },
    Axis,
    XLabels,
    AxisTitle,
}

fn value_to_height(v: f64, y_min: f64, y_max: f64, plot_h: usize) -> usize {
    if y_max <= y_min {
        return 0;
    }
    let frac = ((v - y_min) / (y_max - y_min)).clamp(0.0, 1.0);
    let h = (frac * plot_h as f64).round() as usize;
    h.min(plot_h)
}

fn draw_segment(
    plot: &mut [Vec<char>],
    plot_color: &mut [Vec<Option<Color>>],
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
    ch: char,
) {
    let plot_h = plot.len();
    if plot_h == 0 {
        return;
    }
    let plot_w = plot[0].len();
    let (sx, sy, ex, ey) = if x1 <= x2 {
        (x1, y1, x2, y2)
    } else {
        (x2, y2, x1, y1)
    };
    let dx = ex.saturating_sub(sx).max(1);
    let dy_total = ey as isize - sy as isize;
    for step in 1..dx {
        let x = sx + step;
        if x >= plot_w {
            break;
        }
        let y = sy as isize + (dy_total * step as isize) / dx as isize;
        let yu = y.max(0) as usize;
        if yu < plot_h && plot[yu][x] == ' ' {
            plot[yu][x] = ch;
            plot_color[yu][x] = Some(Color::Yellow);
        }
    }
}

fn build_x_label_row(diag: &XyChart, n: usize, bar_step: usize) -> String {
    let total_w = n * bar_step;
    let mut row = vec![' '; total_w];
    if !diag.x_categories.is_empty() {
        for (i, cat) in diag.x_categories.iter().enumerate().take(n) {
            let center = i * bar_step + bar_step / 2;
            let cw = UnicodeWidthStr::width(cat.as_str());
            let start = center.saturating_sub(cw / 2);
            for (j, c) in cat.chars().enumerate() {
                let pos = start + j;
                if pos < row.len() {
                    row[pos] = c;
                }
            }
        }
    } else if let Some((lo, hi)) = diag.x_range {
        let labels = [format_y_label(lo), format_y_label(hi)];
        for (idx, lbl) in labels.iter().enumerate() {
            let center = if idx == 0 {
                0
            } else {
                total_w.saturating_sub(1)
            };
            let cw = lbl.len();
            let start = center.saturating_sub(cw / 2);
            for (j, c) in lbl.chars().enumerate() {
                let pos = start + j;
                if pos < row.len() {
                    row[pos] = c;
                }
            }
        }
    }
    row.iter().collect::<String>().trim_end().to_string()
}

fn render_horizontal(
    diag: &XyChart,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    let n = data_len(diag);
    if n == 0 {
        return Err(AsciiRenderError::Empty);
    }
    let (y_min, y_max) = compute_y_bounds(diag);
    let bar_fill = match charset {
        CharsetKind::Unicode => '█',
        CharsetKind::Ascii => '#',
    };
    let vert = match charset {
        CharsetKind::Unicode => '│',
        CharsetKind::Ascii => '|',
    };

    let label_w = diag
        .x_categories
        .iter()
        .map(|c| UnicodeWidthStr::width(c.as_str()))
        .max()
        .unwrap_or(0)
        .max(2);

    let total_w = (max_width as usize).max(30);
    let bar_max = total_w.saturating_sub(label_w + 4).max(10);

    let mut row_strs: Vec<String> = Vec::new();
    let mut title_present = false;
    if let Some(title) = &diag.title {
        row_strs.push(title.clone());
        row_strs.push(String::new());
        title_present = true;
    }

    let primary = diag
        .series
        .iter()
        .find(|s| is_bar(s))
        .or_else(|| diag.series.first())
        .unwrap();
    let values = series_values(primary);

    struct HorizRow {
        pad: usize,
        cat: String,
        bar_w: usize,
        value: String,
    }
    let mut horiz_rows: Vec<HorizRow> = Vec::new();
    for (i, &v) in values.iter().enumerate() {
        let frac = if y_max > y_min {
            ((v - y_min) / (y_max - y_min)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let bw = (frac * bar_max as f64).round() as usize;
        let cat = diag
            .x_categories
            .get(i)
            .cloned()
            .unwrap_or_else(|| format!("{}", i + 1));
        let cat_w = UnicodeWidthStr::width(cat.as_str());
        let pad = label_w.saturating_sub(cat_w);
        let bar: String = std::iter::repeat_n(bar_fill, bw).collect();
        row_strs.push(format!(
            "{}{} {} {} {}",
            " ".repeat(pad),
            cat,
            vert,
            bar,
            format_y_label(v)
        ));
        horiz_rows.push(HorizRow {
            pad,
            cat,
            bar_w: bw,
            value: format_y_label(v),
        });
    }

    let canvas_w = row_strs
        .iter()
        .map(|s| UnicodeWidthStr::width(s.as_str()))
        .max()
        .unwrap_or(1)
        .max(1);
    let canvas_h = row_strs.len().max(1);
    let mut canvas = Canvas::new(canvas_w, canvas_h, charset);

    let mut y_cursor = 0usize;
    if title_present {
        canvas.put_str(0, y_cursor, &row_strs[0]);
        canvas.paint_text(0, y_cursor, &row_strs[0], Color::White);
        y_cursor += 2;
    }

    for hr in horiz_rows {
        // layout: pad + cat + " " + vert + " " + bar + " " + value
        let line_x = 0usize;
        let cat_x = line_x + hr.pad;
        let vert_x = cat_x + UnicodeWidthStr::width(hr.cat.as_str()) + 1;
        let bar_x = vert_x + 2;
        let value_x = bar_x + hr.bar_w + 1;

        canvas.put_str_colored(cat_x, y_cursor, &hr.cat, Color::Cyan);
        canvas.put_char(vert_x, y_cursor, vert);
        canvas.set_color(vert_x, y_cursor, Color::DarkGray);
        let bar: String = std::iter::repeat_n(bar_fill, hr.bar_w).collect();
        canvas.put_str_colored(bar_x, y_cursor, &bar, SERIES_CYCLE[0]);
        canvas.put_str_colored(value_x, y_cursor, &hr.value, Color::Green);
        y_cursor += 1;
    }

    Ok(canvas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mermaid::ascii::xychart::parser::parse;

    fn render_src(src: &str) -> String {
        let diag = parse(src).unwrap();
        render(&diag, 60, CharsetKind::Unicode).unwrap().join("\n")
    }

    fn render_src_ascii(src: &str) -> String {
        let diag = parse(src).unwrap();
        render(&diag, 60, CharsetKind::Ascii).unwrap().join("\n")
    }

    #[test]
    fn render_bar_chart() {
        let src = "xychart-beta\nx-axis [a, b, c, d]\nbar [1, 2, 3, 4]";
        insta::assert_snapshot!(render_src(src));
    }

    #[test]
    fn render_line_chart() {
        let src = "xychart-beta\nx-axis [a, b, c, d]\nline [1, 4, 2, 3]";
        insta::assert_snapshot!(render_src(src));
    }

    #[test]
    fn render_mixed_bar_line() {
        let src = "xychart-beta\nx-axis [a, b, c, d]\nbar [3, 4, 2, 5]\nline [3, 4, 2, 5]";
        insta::assert_snapshot!(render_src(src));
    }

    #[test]
    fn render_with_title() {
        let src = "xychart-beta\ntitle \"My Chart\"\nx-axis [a, b, c]\nbar [10, 20, 30]";
        insta::assert_snapshot!(render_src(src));
    }

    #[test]
    fn render_horizontal_orientation() {
        let src = "xychart-beta horizontal\nx-axis [low, mid, high]\nbar [1, 5, 9]";
        insta::assert_snapshot!(render_src(src));
    }

    #[test]
    fn render_x_categories_labeled() {
        let src = "xychart-beta\nx-axis [jan, feb, mar]\nbar [1, 2, 3]";
        let out = render_src(src);
        assert!(out.contains("jan"), "expected jan label, got\n{out}");
        assert!(out.contains("feb"));
        assert!(out.contains("mar"));
    }

    #[test]
    fn render_y_range_labeled() {
        let src = "xychart-beta\ny-axis \"Rev\" 0 --> 100\nbar [10, 50, 100]";
        let out = render_src(src);
        assert!(out.contains("100"), "expected 100 top label, got\n{out}");
        assert!(out.contains('0'));
    }

    #[test]
    fn render_ascii_charset() {
        let src = "xychart-beta\nx-axis [a, b]\nbar [1, 2]";
        insta::assert_snapshot!(render_src_ascii(src));
    }

    #[test]
    fn render_axes_visible() {
        let src = "xychart-beta\nx-axis [a, b, c]\nbar [1, 2, 3]";
        let out = render_src(src);
        assert!(out.contains('│'), "expected vertical │, got\n{out}");
        assert!(out.contains('└'), "expected corner └, got\n{out}");
        assert!(out.contains('─'), "expected horizontal ─, got\n{out}");
    }

    #[test]
    fn render_bars_proportional() {
        let src = "xychart-beta\nx-axis [s, l]\nbar [1, 10]";
        let out = render_src(src);
        let total_fill: usize = out
            .lines()
            .map(|l| l.chars().filter(|&c| c == '█').count())
            .sum();
        assert!(total_fill > 1, "expected bars rendered, got\n{out}");
    }

    #[test]
    fn render_line_connects_points() {
        let src = "xychart-beta\nx-axis [a, b, c]\nline [1, 5, 1]";
        let out = render_src(src);
        assert!(
            out.contains('●') || out.contains('─'),
            "expected line markers, got\n{out}"
        );
    }

    #[test]
    fn render_y_scale_correct() {
        let src = "xychart-beta\ny-axis \"v\" 0 --> 10\nbar [0, 5, 10]";
        let out = render_src(src);
        let total_fill: usize = out
            .lines()
            .map(|l| l.chars().filter(|&c| c == '█').count())
            .sum();
        assert!(total_fill > 0, "expected fill, got\n{out}");
        assert!(
            out.lines().any(|l| l.contains("10") && l.contains('│')),
            "expected top axis row with 10, got\n{out}"
        );
    }

    fn render_styled_str(src: &str) -> Vec<StyledRow> {
        let d = parse(src).unwrap();
        render_styled(&d, 60, CharsetKind::Unicode).unwrap()
    }

    #[test]
    fn render_styled_attaches_colors() {
        let src = "xychart-beta\nx-axis [a, b]\nbar [1, 2]";
        let rows = render_styled_str(src);
        let any_colored = rows
            .iter()
            .flat_map(|r| r.iter())
            .any(|r| r.color.is_some());
        assert!(any_colored, "expected at least one colored run");
    }

    #[test]
    fn render_styled_specific_element_colored() {
        let src = "xychart-beta\ntitle \"T\"\nx-axis [a, b]\nbar [1, 2]";
        let rows = render_styled_str(src);
        let mut has_white_title = false;
        let mut has_darkgray_axis = false;
        let mut has_cyan_axis_label = false;
        let mut has_series_color = false;
        for row in &rows {
            for run in row {
                if run.color == Some(Color::White) && run.text.contains('T') {
                    has_white_title = true;
                }
                if run.color == Some(Color::DarkGray)
                    && (run.text.contains('│') || run.text.contains('└') || run.text.contains('─'))
                {
                    has_darkgray_axis = true;
                }
                if run.color == Some(Color::Cyan) {
                    has_cyan_axis_label = true;
                }
                if run.color == Some(Color::Cyan) && run.text.contains('█') {
                    has_series_color = true;
                }
                if run.color == Some(SERIES_CYCLE[0]) && run.text.contains('█') {
                    has_series_color = true;
                }
            }
        }
        assert!(has_white_title, "title should be white");
        assert!(has_darkgray_axis, "axis chars should be dark gray");
        assert!(has_cyan_axis_label, "axis label should be cyan");
        assert!(has_series_color, "bar fill should be colored");
    }

    #[test]
    fn render_styled_no_alignment_drift() {
        let src = "xychart-beta\ntitle \"T\"\nx-axis [a, b, c]\nbar [1, 2, 3]";
        let d = parse(src).unwrap();
        let plain = render(&d, 60, CharsetKind::Unicode).unwrap();
        let styled = render_styled(&d, 60, CharsetKind::Unicode).unwrap();
        assert_eq!(plain.len(), styled.len());
        for (i, row) in styled.iter().enumerate() {
            let flat: String = row.iter().map(|r| r.text.as_str()).collect();
            let flat_trimmed = flat.trim_end_matches(' ').to_string();
            assert_eq!(flat_trimmed, plain[i], "row {i} differs");
        }
    }
}
