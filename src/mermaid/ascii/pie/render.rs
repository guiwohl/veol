use super::ast::PieChart;
use crate::mermaid::ascii::canvas::Canvas;
use crate::mermaid::ascii::{AsciiRenderError, CharsetKind, StyledRow};
use ratatui::style::Color;
use unicode_width::UnicodeWidthStr;

const PERCENT_COL_WIDTH: usize = 7;
const GAP_BEFORE_BAR: usize = 2;
const GAP_AFTER_BAR: usize = 2;
const MIN_BAR_WIDTH: usize = 10;
const MAX_LABEL_COL: usize = 20;
const MIN_TOTAL_WIDTH: usize = 30;

const SLICE_PALETTE: [Color; 6] = [
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    Color::Green,
    Color::Blue,
    Color::Red,
];

pub fn render(
    diag: &PieChart,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_lines())
}

pub fn render_styled(
    diag: &PieChart,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_styled_lines())
}

fn render_to_canvas(
    diag: &PieChart,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    if diag.slices.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    let total: f64 = diag.slices.iter().map(|s| s.value).sum();
    if total <= 0.0 {
        return Err(AsciiRenderError::Layout("pie total must be > 0".into()));
    }

    let (filled_ch, empty_ch) = match charset {
        CharsetKind::Unicode => ('█', '░'),
        CharsetKind::Ascii => ('#', '.'),
    };

    let widest_label_raw = diag
        .slices
        .iter()
        .map(|s| UnicodeWidthStr::width(s.label.as_str()))
        .max()
        .unwrap_or(0);
    let label_col = widest_label_raw.min(MAX_LABEL_COL);

    let widest_value = if diag.show_data {
        diag.slices
            .iter()
            .map(|s| format_value(s.value).len())
            .max()
            .unwrap_or(0)
    } else {
        0
    };

    let trailing_col_width = if diag.show_data {
        GAP_AFTER_BAR + PERCENT_COL_WIDTH + 2 + widest_value + 1
    } else {
        GAP_AFTER_BAR + PERCENT_COL_WIDTH
    };

    let effective_width = (max_width as usize).max(MIN_TOTAL_WIDTH);
    let overhead = label_col + GAP_BEFORE_BAR + trailing_col_width;
    let bar_width = if overhead + MIN_BAR_WIDTH > effective_width {
        MIN_BAR_WIDTH
    } else {
        effective_width - overhead
    };

    let title_rows = if diag.title.is_some() { 2 } else { 0 };
    let total_row_width = label_col + GAP_BEFORE_BAR + bar_width + trailing_col_width;
    let title_width = diag
        .title
        .as_ref()
        .map(|t| UnicodeWidthStr::width(t.as_str()))
        .unwrap_or(0);
    let canvas_width = total_row_width.max(title_width).max(1);
    let canvas_height = title_rows + diag.slices.len();
    let canvas_height = canvas_height.max(1);

    let mut canvas = Canvas::new(canvas_width, canvas_height, charset);

    let mut y = 0usize;
    if let Some(title) = &diag.title {
        canvas.put_str_colored(0, y, title, Color::White);
        y += 2;
    }

    for (i, slice) in diag.slices.iter().enumerate() {
        let pct = slice.value / total * 100.0;
        let fill_n = ((slice.value / total) * bar_width as f64).round() as usize;
        let fill_n = fill_n.min(bar_width);
        let empty_n = bar_width - fill_n;

        let label = pad_or_truncate(&slice.label, label_col);
        canvas.put_str(0, y, &label);

        let bar_x = label_col + GAP_BEFORE_BAR;
        let slice_color = SLICE_PALETTE[i % SLICE_PALETTE.len()];
        for k in 0..fill_n {
            canvas.put_char(bar_x + k, y, filled_ch);
            canvas.set_color(bar_x + k, y, slice_color);
        }
        for k in 0..empty_n {
            canvas.put_char(bar_x + fill_n + k, y, empty_ch);
            canvas.set_color(bar_x + fill_n + k, y, Color::DarkGray);
        }

        let pct_str = format!("{:>5.1}%", pct);
        let pct_x = bar_x + bar_width + GAP_AFTER_BAR;
        canvas.put_str_colored(pct_x, y, &pct_str, Color::Green);

        if diag.show_data {
            let raw = format!("  ({})", format_value(slice.value));
            let raw_x = pct_x + pct_str.chars().count();
            canvas.put_str_colored(raw_x, y, &raw, Color::DarkGray);
        }

        y += 1;
    }

    Ok(canvas)
}

fn format_value(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn pad_or_truncate(s: &str, width: usize) -> String {
    let w = UnicodeWidthStr::width(s);
    if w <= width {
        let pad = width - w;
        let mut out = s.to_string();
        out.push_str(&" ".repeat(pad));
        return out;
    }
    if width == 0 {
        return String::new();
    }
    let mut acc = String::new();
    let mut used = 0;
    for c in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);
        if used + cw + 1 > width {
            break;
        }
        acc.push(c);
        used += cw;
    }
    acc.push('…');
    used += 1;
    while used < width {
        acc.push(' ');
        used += 1;
    }
    acc
}

#[cfg(test)]
mod tests {
    use crate::mermaid::ascii::StyledRun;
    use ratatui::style::Color;

    fn render(src: &str) -> String {
        crate::mermaid::ascii::pie::render(src, 60, crate::mermaid::ascii::CharsetKind::Unicode)
            .unwrap()
            .join("\n")
    }

    fn render_ascii(src: &str) -> String {
        crate::mermaid::ascii::pie::render(src, 60, crate::mermaid::ascii::CharsetKind::Ascii)
            .unwrap()
            .join("\n")
    }

    #[test]
    fn render_two_slices() {
        let src = "pie\n\"A\" : 50\n\"B\" : 50";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_three_slices() {
        let src = "pie\n\"Red\" : 30\n\"Green\" : 20\n\"Blue\" : 50";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_with_title() {
        let src = "pie title Distribution\n\"A\" : 60\n\"B\" : 40";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_with_show_data() {
        let src = "pie showData title Sales\n\"Q1\" : 123\n\"Q2\" : 47\n\"Q3\" : 103";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_uneven_proportions() {
        let src = "pie\n\"Big\" : 95\n\"Small\" : 5";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_long_label_truncates_or_wraps() {
        let src = "pie\n\"This is a really really long label that overflows\" : 70\n\"Short\" : 30";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_single_slice_full_bar() {
        let src = "pie\n\"Only\" : 1";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_ascii_charset() {
        let src = "pie title ASCII Mode\n\"Alpha\" : 60\n\"Beta\" : 40";
        insta::assert_snapshot!(render_ascii(src));
    }

    #[test]
    fn render_each_slice_on_own_line() {
        let src = "pie\n\"A\" : 25\n\"B\" : 25\n\"C\" : 25\n\"D\" : 25";
        let out = render(src);
        let count_with_pct = out.lines().filter(|l| l.contains('%')).count();
        assert_eq!(
            count_with_pct, 4,
            "expected 4 percentage lines, out=\n{out}"
        );
        assert!(out.contains('A'));
    }

    #[test]
    fn render_bars_proportional_to_value() {
        let src = "pie\n\"Big\" : 80\n\"Small\" : 20";
        let out = render(src);
        let lines: Vec<&str> = out.lines().filter(|l| l.contains('%')).collect();
        assert_eq!(lines.len(), 2);
        let big_filled = lines[0].chars().filter(|&c| c == '█').count();
        let small_filled = lines[1].chars().filter(|&c| c == '█').count();
        assert!(
            big_filled > small_filled,
            "big bar ({big_filled}) should be wider than small ({small_filled})\nout=\n{out}"
        );
    }

    #[test]
    fn render_percentages_sum_to_100() {
        let src = "pie\n\"A\" : 1\n\"B\" : 1\n\"C\" : 1";
        let out = render(src);
        let mut sum = 0.0;
        for line in out.lines() {
            if let Some(p_idx) = line.find('%') {
                let prefix = &line[..p_idx];
                let num_start = prefix
                    .rfind(|c: char| c.is_whitespace())
                    .map(|i| i + 1)
                    .unwrap_or(0);
                let num_str = &prefix[num_start..];
                if let Ok(v) = num_str.parse::<f64>() {
                    sum += v;
                }
            }
        }
        assert!(
            (sum - 100.0).abs() < 0.5,
            "percentages sum {sum} should be ~100, out=\n{out}"
        );
    }

    #[test]
    fn render_title_appears_first() {
        let src = "pie title My Title\n\"A\" : 1\n\"B\" : 1";
        let out = render(src);
        let first_nonblank = out.lines().find(|l| !l.trim().is_empty()).unwrap();
        assert!(
            first_nonblank.contains("My Title"),
            "title should be first nonblank line, out=\n{out}"
        );
    }

    #[test]
    fn render_show_data_displays_raw_values() {
        let src = "pie showData\n\"A\" : 123\n\"B\" : 47";
        let out = render(src);
        assert!(
            out.contains("123"),
            "expected raw 123 in output, got\n{out}"
        );
        assert!(out.contains("47"), "expected raw 47 in output, got\n{out}");
    }

    // ---- Color verification tests ----

    fn distinct_colors(rows: &[Vec<StyledRun>]) -> std::collections::HashSet<Color> {
        let mut set = std::collections::HashSet::new();
        for row in rows {
            for run in row {
                if let Some(c) = run.color {
                    set.insert(c);
                }
            }
        }
        set
    }

    fn find_color_at(row: &[StyledRun], target: char) -> Option<Color> {
        for run in row {
            if run.text.contains(target) {
                return run.color;
            }
        }
        None
    }

    #[test]
    fn render_styled_attaches_colors() {
        let diag =
            crate::mermaid::ascii::pie::parser::parse("pie title Hello\n\"A\" : 30\n\"B\" : 70")
                .unwrap();
        let rows =
            super::render_styled(&diag, 60, crate::mermaid::ascii::CharsetKind::Unicode).unwrap();
        let colors = distinct_colors(&rows);
        assert!(
            colors.len() >= 2,
            "expected at least 2 distinct colors, got {colors:?}"
        );
    }

    #[test]
    fn render_styled_specific_element_colored() {
        let diag =
            crate::mermaid::ascii::pie::parser::parse("pie\n\"A\" : 50\n\"B\" : 50").unwrap();
        let rows =
            super::render_styled(&diag, 60, crate::mermaid::ascii::CharsetKind::Unicode).unwrap();
        // Slice 0 → Cyan filled bar
        let first_filled = find_color_at(&rows[0], '█');
        assert_eq!(first_filled, Some(Color::Cyan));
        // Slice 1 → Magenta filled bar
        let second_filled = find_color_at(&rows[1], '█');
        assert_eq!(second_filled, Some(Color::Magenta));
        // Empty cells (░) painted DarkGray
        let first_empty = find_color_at(&rows[0], '░');
        assert_eq!(first_empty, Some(Color::DarkGray));
    }

    #[test]
    fn render_styled_no_alignment_drift() {
        let src = "pie showData title Sales\n\"Q1\" : 123\n\"Q2\" : 47\n\"Q3\" : 103";
        let diag = crate::mermaid::ascii::pie::parser::parse(src).unwrap();
        let plain = crate::mermaid::ascii::pie::render(
            src,
            60,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap();
        let styled =
            super::render_styled(&diag, 60, crate::mermaid::ascii::CharsetKind::Unicode).unwrap();
        assert_eq!(plain.len(), styled.len());
        for (i, (line, row)) in plain.iter().zip(styled.iter()).enumerate() {
            let flat: String = row.iter().map(|r| r.text.as_str()).collect();
            let flat = flat.trim_end().to_string();
            assert_eq!(&flat, line, "row {i} mismatch");
        }
    }
}
