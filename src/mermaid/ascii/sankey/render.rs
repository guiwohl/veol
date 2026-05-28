use super::ast::SankeyDiagram;
use crate::mermaid::ascii::canvas::Canvas;
use crate::mermaid::ascii::{AsciiRenderError, CharsetKind, StyledRow};
use ratatui::style::Color;
use std::collections::HashMap;
use unicode_width::UnicodeWidthStr;

const MAX_BAR: usize = 12;
const MIN_BAR: usize = 1;
const MAX_LABEL_COL: usize = 22;
const GAP: usize = 2;

const SOURCE_CYCLE: [Color; 6] = [
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    Color::Green,
    Color::Blue,
    Color::Red,
];

pub fn render(
    diag: &SankeyDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_lines())
}

pub fn render_styled(
    diag: &SankeyDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_styled_lines())
}

fn render_to_canvas(
    diag: &SankeyDiagram,
    _max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    if diag.flows.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    let (horiz, arrow, corner) = match charset {
        CharsetKind::Unicode => ('─', '►', '└'),
        CharsetKind::Ascii => ('-', '>', '+'),
    };

    let max_val = diag.flows.iter().map(|f| f.value).fold(0.0_f64, f64::max);
    let total: f64 = diag.flows.iter().map(|f| f.value).sum();

    let src_col = diag
        .flows
        .iter()
        .map(|f| UnicodeWidthStr::width(f.source.as_str()))
        .max()
        .unwrap_or(0)
        .min(MAX_LABEL_COL);

    let tgt_col = diag
        .flows
        .iter()
        .map(|f| UnicodeWidthStr::width(f.target.as_str()))
        .max()
        .unwrap_or(0)
        .min(MAX_LABEL_COL);

    let widest_value = diag
        .flows
        .iter()
        .map(|f| format_value(f.value).len())
        .max()
        .unwrap_or(0);

    // line layout: src(src_col) GAP bar(MAX_BAR) arrow tgt(tgt_col) GAP ( val(widest_value) )
    let value_field_w = widest_value + 2; // "(...)"
    let line_w = src_col + GAP + MAX_BAR + 1 + GAP + tgt_col + GAP + value_field_w;

    let total_row_text = format!(
        "{corner}{horiz} total: {tot}",
        corner = corner,
        horiz = horiz,
        tot = format_value(total),
    );
    let total_indent = src_col + GAP;
    let total_row_w = total_indent + UnicodeWidthStr::width(total_row_text.as_str());

    let canvas_w = line_w.max(total_row_w).max("Sankey".len()).max(1);
    let canvas_h = 2 + diag.flows.len() + 1;

    let mut canvas = Canvas::new(canvas_w, canvas_h, charset);
    canvas.put_str(0, 0, "Sankey");

    let mut source_color_for: HashMap<String, Color> = HashMap::new();
    let mut next_idx = 0usize;
    for f in &diag.flows {
        if !source_color_for.contains_key(&f.source) {
            let c = SOURCE_CYCLE[next_idx % SOURCE_CYCLE.len()];
            source_color_for.insert(f.source.clone(), c);
            next_idx += 1;
        }
    }

    for (i, flow) in diag.flows.iter().enumerate() {
        let y = 2 + i;
        let bar_n = if max_val > 0.0 {
            let n = ((flow.value / max_val) * MAX_BAR as f64).round() as usize;
            n.clamp(MIN_BAR, MAX_BAR)
        } else {
            MIN_BAR
        };

        let src = pad_or_truncate(&flow.source, src_col);
        let tgt = pad_or_truncate(&flow.target, tgt_col);
        let bar_segment: String = std::iter::repeat_n(horiz, bar_n).collect();
        let bar_padding: String = std::iter::repeat_n(' ', MAX_BAR - bar_n).collect();
        let value = format_value(flow.value);
        let value_padded = format!("{value:>w$}", w = widest_value);

        let mut x = 0usize;
        canvas.put_str_colored(x, y, &src, Color::Cyan);
        x += src_col + GAP;

        let bar_color = *source_color_for.get(&flow.source).unwrap();
        canvas.put_str_colored(x, y, &bar_segment, bar_color);
        x += bar_n;
        canvas.put_str(x, y, &bar_padding);
        x += MAX_BAR - bar_n;
        canvas.put_str_colored(x, y, &arrow.to_string(), Color::Yellow);
        x += 1;
        x += GAP;
        canvas.put_str_colored(x, y, &tgt, Color::Magenta);
        x += tgt_col + GAP;
        canvas.put_str(x, y, "(");
        x += 1;
        canvas.put_str_colored(x, y, &value_padded, Color::Green);
        x += widest_value;
        canvas.put_str(x, y, ")");
        // x advances done.
    }

    let total_y = 2 + diag.flows.len();
    canvas.put_str_colored(total_indent, total_y, &total_row_text, Color::DarkGray);

    Ok(canvas)
}

fn format_value(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v as i64)
    } else {
        format!("{v:.1}")
    }
}

fn pad_or_truncate(s: &str, width: usize) -> String {
    let w = UnicodeWidthStr::width(s);
    if w <= width {
        let mut out = s.to_string();
        out.push_str(&" ".repeat(width - w));
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
    use super::*;
    use crate::mermaid::ascii::sankey::parser::parse;

    fn render_str(src: &str) -> String {
        let d = parse(src).unwrap();
        render(&d, 80, CharsetKind::Unicode).unwrap().join("\n")
    }

    fn render_ascii_str(src: &str) -> String {
        let d = parse(src).unwrap();
        render(&d, 80, CharsetKind::Ascii).unwrap().join("\n")
    }

    #[test]
    fn render_sankey_minimal() {
        let src = "sankey-beta\nA,B,10\n";
        insta::assert_snapshot!(render_str(src));
    }

    #[test]
    fn render_three_flows() {
        let src = "sankey-beta\nA,B,10\nA,C,20\nB,D,5\n";
        insta::assert_snapshot!(render_str(src));
    }

    #[test]
    fn render_proportional_widths() {
        let src =
            "sankey-beta\nAgricultural 'waste',Bio-conversion,124.729\nBio-conversion,Liquid,0.597\nBio-conversion,Losses,26.862\nBio-conversion,Solid,280.322\nBio-conversion,Gas,81.144\n";
        insta::assert_snapshot!(render_str(src));
    }

    #[test]
    fn render_source_node_listed() {
        let src = "sankey-beta\nAlpha,Beta,5\n";
        let out = render_str(src);
        assert!(out.contains("Alpha"), "missing source\n{out}");
    }

    #[test]
    fn render_target_node_listed() {
        let src = "sankey-beta\nAlpha,Beta,5\n";
        let out = render_str(src);
        assert!(out.contains("Beta"), "missing target\n{out}");
    }

    #[test]
    fn render_total_value_shown() {
        let src = "sankey-beta\nA,B,10\nA,C,20\n";
        let out = render_str(src);
        assert!(out.contains("total:"), "missing total label\n{out}");
        assert!(out.contains("30"), "expected total 30\n{out}");
    }

    #[test]
    fn render_ascii_charset() {
        let src = "sankey-beta\nA,B,10\nA,C,5\n";
        insta::assert_snapshot!(render_ascii_str(src));
    }

    #[test]
    fn render_flows_proportional_to_value() {
        let src = "sankey-beta\nA,Big,100\nA,Small,10\n";
        let out = render_str(src);
        let lines: Vec<&str> = out.lines().collect();
        let big = lines.iter().find(|l| l.contains("Big")).unwrap();
        let small = lines.iter().find(|l| l.contains("Small")).unwrap();
        let big_bars = big.chars().filter(|&c| c == '─').count();
        let small_bars = small.chars().filter(|&c| c == '─').count();
        assert!(
            big_bars > small_bars,
            "big bar ({big_bars}) should exceed small ({small_bars})\n{out}"
        );
    }

    #[test]
    fn render_each_flow_labeled() {
        let src = "sankey-beta\nA,X,1\nB,Y,2\nC,Z,3\n";
        let out = render_str(src);
        let flow_lines = out
            .lines()
            .filter(|l| l.contains('►') && !l.contains("total"))
            .count();
        assert_eq!(flow_lines, 3, "expected 3 flow lines, got:\n{out}");
    }

    #[test]
    fn render_values_visible() {
        let src = "sankey-beta\nA,B,124.7\nA,C,0.5\n";
        let out = render_str(src);
        assert!(out.contains("124.7"), "missing 124.7\n{out}");
        assert!(out.contains("0.5"), "missing 0.5\n{out}");
    }

    fn render_styled_str(src: &str) -> Vec<StyledRow> {
        let d = parse(src).unwrap();
        render_styled(&d, 80, CharsetKind::Unicode).unwrap()
    }

    #[test]
    fn render_styled_attaches_colors() {
        let src = "sankey-beta\nA,B,10\n";
        let rows = render_styled_str(src);
        let any_colored = rows
            .iter()
            .flat_map(|r| r.iter())
            .any(|r| r.color.is_some());
        assert!(any_colored, "expected at least one colored run");
    }

    #[test]
    fn render_styled_specific_element_colored() {
        let src = "sankey-beta\nA,B,10\nA,C,20\n";
        let rows = render_styled_str(src);
        let mut has_cyan_a = false;
        let mut has_magenta_target = false;
        let mut has_yellow_arrow = false;
        let mut has_green_value = false;
        let mut has_darkgray_total = false;
        for row in &rows {
            for run in row {
                if run.color == Some(Color::Cyan) && run.text.starts_with('A') {
                    has_cyan_a = true;
                }
                if run.color == Some(Color::Magenta)
                    && (run.text.starts_with('B') || run.text.starts_with('C'))
                {
                    has_magenta_target = true;
                }
                if run.color == Some(Color::Yellow) && run.text.contains('►') {
                    has_yellow_arrow = true;
                }
                if run.color == Some(Color::Green)
                    && run
                        .text
                        .trim()
                        .chars()
                        .all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == ' ')
                    && !run.text.trim().is_empty()
                {
                    has_green_value = true;
                }
                if run.color == Some(Color::DarkGray) && run.text.contains("total:") {
                    has_darkgray_total = true;
                }
            }
        }
        assert!(has_cyan_a, "source name should be cyan");
        assert!(has_magenta_target, "target name should be magenta");
        assert!(has_yellow_arrow, "arrow should be yellow");
        assert!(has_green_value, "value should be green");
        assert!(has_darkgray_total, "total line should be dark gray");
    }

    #[test]
    fn render_styled_no_alignment_drift() {
        let src = "sankey-beta\nA,B,10\nA,C,20\nB,D,5\n";
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
