use super::ast::{Timeline, TimelineSection};
use crate::mermaid::ascii::canvas::Canvas;
use crate::mermaid::ascii::charset::{get_charset, Charset};
use crate::mermaid::ascii::{AsciiRenderError, CharsetKind, StyledRow};
use ratatui::style::Color;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const PERIOD_INDENT: usize = 2;
const PERIOD_TO_MARKER_GAP: usize = 1;
const MARKER_TO_EVENT_GAP: usize = 2;
const EVENT_RIGHT_MARGIN: usize = 2;
const MIN_BOX_INNER: usize = 16;
const SECTION_NAME_DASH_PREFIX: usize = 1;

pub fn render(
    diag: &Timeline,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_lines())
}

pub fn render_styled(
    diag: &Timeline,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_styled_lines())
}

fn render_to_canvas(
    diag: &Timeline,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    if diag.sections.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    let cs = get_charset(charset);
    let marker = match charset {
        CharsetKind::Unicode => '●',
        CharsetKind::Ascii => '*',
    };

    let widest_period = diag
        .sections
        .iter()
        .flat_map(|s| s.entries.iter())
        .map(|e| UnicodeWidthStr::width(e.period.as_str()))
        .max()
        .unwrap_or(0);

    let widest_event = diag
        .sections
        .iter()
        .flat_map(|s| s.entries.iter())
        .flat_map(|e| e.events.iter())
        .map(|s| UnicodeWidthStr::width(s.as_str()))
        .max()
        .unwrap_or(0);

    let widest_section_name = diag
        .sections
        .iter()
        .map(|s| UnicodeWidthStr::width(s.name.as_str()))
        .max()
        .unwrap_or(0);

    let row_content_width = PERIOD_INDENT
        + widest_period
        + PERIOD_TO_MARKER_GAP
        + 1
        + MARKER_TO_EVENT_GAP
        + widest_event
        + EVENT_RIGHT_MARGIN;
    let section_name_needs = SECTION_NAME_DASH_PREFIX + 1 + widest_section_name + 1 + 2;
    let inner_width = row_content_width.max(section_name_needs).max(MIN_BOX_INNER);

    let max_inner = (max_width as usize).saturating_sub(2).max(MIN_BOX_INNER);
    let inner_width = inner_width.min(max_inner);

    let event_col_x =
        PERIOD_INDENT + widest_period + PERIOD_TO_MARKER_GAP + 1 + MARKER_TO_EVENT_GAP;
    let event_max_width = inner_width
        .saturating_sub(event_col_x + EVENT_RIGHT_MARGIN)
        .max(1);

    // Compute total height
    let mut height = 0usize;
    if diag.title.is_some() {
        height += 2;
    }
    for section in &diag.sections {
        let body_rows = section_body_row_count(section, event_max_width);
        height += 1 + 1 + body_rows + 1 + 1; // top + pad + body + pad + bottom
    }
    let title_width = diag
        .title
        .as_ref()
        .map(|t| UnicodeWidthStr::width(t.as_str()))
        .unwrap_or(0);
    let total_width = (inner_width + 2).max(title_width).max(1);
    let height = height.max(1);

    let mut canvas = Canvas::new(total_width, height, charset);

    let mut y = 0usize;
    if let Some(title) = &diag.title {
        canvas.put_str_colored(0, y, title, Color::White);
        y += 2;
    }

    for section in &diag.sections {
        y = paint_section(
            &mut canvas,
            y,
            section,
            inner_width,
            event_col_x,
            event_max_width,
            widest_period,
            marker,
            cs,
        );
    }

    Ok(canvas)
}

fn section_body_row_count(section: &TimelineSection, event_max_width: usize) -> usize {
    let entry_count = section.entries.len();
    let mut total = 0usize;
    for (i, entry) in section.entries.iter().enumerate() {
        for ev in &entry.events {
            total += wrap_text(ev, event_max_width).len();
        }
        if i + 1 < entry_count {
            total += 1;
        }
    }
    total
}

#[allow(clippy::too_many_arguments)]
fn paint_section(
    canvas: &mut Canvas,
    start_y: usize,
    section: &TimelineSection,
    inner_width: usize,
    event_col_x: usize,
    event_max_width: usize,
    widest_period: usize,
    marker: char,
    cs: &dyn Charset,
) -> usize {
    let mut y = start_y;
    // Top border
    paint_top_border(canvas, y, &section.name, inner_width, cs);
    y += 1;
    // Padding row
    paint_padding_row(canvas, y, inner_width, cs);
    y += 1;

    let entry_count = section.entries.len();
    for (i, entry) in section.entries.iter().enumerate() {
        let lines_for_events: Vec<Vec<String>> = entry
            .events
            .iter()
            .map(|e| wrap_text(e, event_max_width))
            .collect();

        let mut first = true;
        for ev_lines in lines_for_events.iter() {
            for ev_line in ev_lines.iter() {
                paint_period_row(
                    canvas,
                    y,
                    if first {
                        Some(entry.period.as_str())
                    } else {
                        None
                    },
                    widest_period,
                    first,
                    marker,
                    event_col_x,
                    ev_line,
                    inner_width,
                    cs,
                );
                y += 1;
                first = false;
            }
        }

        if i + 1 < entry_count {
            paint_connector_row(canvas, y, widest_period, inner_width, cs);
            y += 1;
        }
    }

    paint_padding_row(canvas, y, inner_width, cs);
    y += 1;
    paint_bottom_border(canvas, y, inner_width, cs);
    y += 1;
    y
}

fn paint_top_border(
    canvas: &mut Canvas,
    y: usize,
    name: &str,
    inner_width: usize,
    cs: &dyn Charset,
) {
    canvas.put_char(0, y, cs.top_left());
    canvas.set_color(0, y, Color::Blue);
    let name_trim = name.trim();
    let mut x = 1usize;
    if name_trim.is_empty() {
        for _ in 0..inner_width {
            canvas.put_char(x, y, cs.horiz());
            canvas.set_color(x, y, Color::Blue);
            x += 1;
        }
    } else {
        // first dash
        canvas.put_char(x, y, cs.horiz());
        canvas.set_color(x, y, Color::Blue);
        x += 1;
        canvas.put_char(x, y, ' ');
        x += 1;
        let label_w = UnicodeWidthStr::width(name_trim);
        let used = 1 + 1 + label_w + 1; // dash + space + label + space
        if used >= inner_width {
            let avail = inner_width.saturating_sub(3);
            let truncated = truncate_to_width(name_trim, avail);
            canvas.put_str_colored(x, y, &truncated, Color::Blue);
            let tw = UnicodeWidthStr::width(truncated.as_str());
            x += tw;
            canvas.put_char(x, y, ' ');
            x += 1;
            let cur = 1 + 1 + tw + 1;
            for _ in cur..inner_width {
                canvas.put_char(x, y, cs.horiz());
                canvas.set_color(x, y, Color::Blue);
                x += 1;
            }
        } else {
            canvas.put_str_colored(x, y, name_trim, Color::Blue);
            x += label_w;
            canvas.put_char(x, y, ' ');
            x += 1;
            for _ in used..inner_width {
                canvas.put_char(x, y, cs.horiz());
                canvas.set_color(x, y, Color::Blue);
                x += 1;
            }
        }
    }
    canvas.put_char(inner_width + 1, y, cs.top_right());
    canvas.set_color(inner_width + 1, y, Color::Blue);
}

fn paint_bottom_border(canvas: &mut Canvas, y: usize, inner_width: usize, cs: &dyn Charset) {
    canvas.put_char(0, y, cs.bot_left());
    canvas.set_color(0, y, Color::Blue);
    for i in 0..inner_width {
        canvas.put_char(1 + i, y, cs.horiz());
        canvas.set_color(1 + i, y, Color::Blue);
    }
    canvas.put_char(inner_width + 1, y, cs.bot_right());
    canvas.set_color(inner_width + 1, y, Color::Blue);
}

fn paint_padding_row(canvas: &mut Canvas, y: usize, inner_width: usize, cs: &dyn Charset) {
    canvas.put_char(0, y, cs.vert());
    canvas.set_color(0, y, Color::Blue);
    canvas.put_char(inner_width + 1, y, cs.vert());
    canvas.set_color(inner_width + 1, y, Color::Blue);
}

#[allow(clippy::too_many_arguments)]
fn paint_period_row(
    canvas: &mut Canvas,
    y: usize,
    period: Option<&str>,
    widest_period: usize,
    show_marker: bool,
    marker: char,
    event_col_x: usize,
    event_text: &str,
    inner_width: usize,
    cs: &dyn Charset,
) {
    canvas.put_char(0, y, cs.vert());
    canvas.set_color(0, y, Color::Blue);
    canvas.put_char(inner_width + 1, y, cs.vert());
    canvas.set_color(inner_width + 1, y, Color::Blue);

    let period_start = PERIOD_INDENT;
    let period_field_end = period_start + widest_period;
    if let Some(p) = period {
        let pw = UnicodeWidthStr::width(p);
        let start = period_start + widest_period.saturating_sub(pw);
        canvas.put_str_colored(1 + start, y, p, Color::Yellow);
    }
    let marker_col = period_field_end + PERIOD_TO_MARKER_GAP;
    if show_marker {
        if marker_col < inner_width {
            canvas.put_char(1 + marker_col, y, marker);
            canvas.set_color(1 + marker_col, y, Color::Magenta);
        }
        let dash_col = marker_col + 1;
        if dash_col < inner_width {
            canvas.put_char(1 + dash_col, y, cs.horiz());
        }
    }
    // event text — clamp to inner area
    place_text_in_inner(canvas, y, event_col_x, event_text, inner_width);
}

fn paint_connector_row(
    canvas: &mut Canvas,
    y: usize,
    widest_period: usize,
    inner_width: usize,
    cs: &dyn Charset,
) {
    canvas.put_char(0, y, cs.vert());
    canvas.set_color(0, y, Color::Blue);
    canvas.put_char(inner_width + 1, y, cs.vert());
    canvas.set_color(inner_width + 1, y, Color::Blue);
    let marker_col = PERIOD_INDENT + widest_period + PERIOD_TO_MARKER_GAP;
    if marker_col < inner_width {
        canvas.put_char(1 + marker_col, y, cs.vert());
        canvas.set_color(1 + marker_col, y, Color::DarkGray);
    }
}

fn place_text_in_inner(
    canvas: &mut Canvas,
    y: usize,
    start_in_inner: usize,
    s: &str,
    inner_width: usize,
) {
    let mut col = start_in_inner;
    for c in s.chars() {
        let w = UnicodeWidthChar::width(c).unwrap_or(1).max(1);
        if col >= inner_width {
            break;
        }
        canvas.put_char(1 + col, y, c);
        col += w;
    }
}

fn truncate_to_width(s: &str, max: usize) -> String {
    if UnicodeWidthStr::width(s) <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let mut acc = String::new();
    let mut used = 0;
    for c in s.chars() {
        let cw = UnicodeWidthChar::width(c).unwrap_or(1);
        if used + cw + 1 > max {
            break;
        }
        acc.push(c);
        used += cw;
    }
    acc.push('…');
    acc
}

fn wrap_text(s: &str, max: usize) -> Vec<String> {
    if max == 0 {
        return vec![s.to_string()];
    }
    if UnicodeWidthStr::width(s) <= max {
        return vec![s.to_string()];
    }
    let words: Vec<&str> = s.split_whitespace().collect();
    if words.is_empty() {
        return vec![s.to_string()];
    }
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut cur_w = 0usize;
    for w in words {
        let ww = UnicodeWidthStr::width(w);
        if cur.is_empty() {
            if ww <= max {
                cur.push_str(w);
                cur_w = ww;
            } else {
                out.push(hard_break(w, max));
            }
            continue;
        }
        if cur_w + 1 + ww <= max {
            cur.push(' ');
            cur.push_str(w);
            cur_w += 1 + ww;
        } else {
            out.push(std::mem::take(&mut cur));
            cur_w = 0;
            if ww <= max {
                cur.push_str(w);
                cur_w = ww;
            } else {
                out.push(hard_break(w, max));
            }
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn hard_break(s: &str, max: usize) -> String {
    let mut acc = String::new();
    let mut used = 0;
    for c in s.chars() {
        let cw = UnicodeWidthChar::width(c).unwrap_or(1);
        if used + cw > max {
            break;
        }
        acc.push(c);
        used += cw;
    }
    acc
}

#[cfg(test)]
mod tests {
    use crate::mermaid::ascii::StyledRun;
    use ratatui::style::Color;

    fn render(s: &str) -> String {
        crate::mermaid::ascii::timeline::render(s, 100, crate::mermaid::ascii::CharsetKind::Unicode)
            .unwrap()
            .join("\n")
    }

    fn render_ascii(s: &str) -> String {
        crate::mermaid::ascii::timeline::render(s, 100, crate::mermaid::ascii::CharsetKind::Ascii)
            .unwrap()
            .join("\n")
    }

    #[test]
    fn render_timeline_minimal() {
        let src = "timeline\n    section S\n      2001 : E1";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_with_title() {
        let src = "timeline\n    title History of Tech\n    section 2000s\n      2001 : Wikipedia";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_one_section() {
        let src =
            "timeline\n    section 2000s\n      2001 : Wikipedia launched\n      2004 : Facebook";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_two_sections() {
        let src = "timeline\n    title History of Tech\n    section 2000s\n      2001 : Wikipedia launched\n      2004 : Facebook\n    section 2010s\n      2010 : Instagram\n      2012 : Pinterest";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_period_with_multiple_events() {
        let src = "timeline\n    section 2000s\n      2004 : Facebook\n           : Gmail";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_long_event_wraps() {
        let src = "timeline\n    section S\n      2001 : This is a really long event description that goes on";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_ascii_charset() {
        let src = "timeline\n    title T\n    section S\n      2001 : E1\n      2004 : E2";
        insta::assert_snapshot!(render_ascii(src));
    }

    #[test]
    fn render_periods_are_columns_or_rows() {
        let src = "timeline\n    section S\n      2001 : E1\n      2004 : E2";
        let out = render(src);
        assert!(out.contains("2001"), "period 2001 missing, got:\n{out}");
        assert!(out.contains("2004"), "period 2004 missing, got:\n{out}");
        let lines: Vec<&str> = out.lines().collect();
        let p2001 = lines.iter().position(|l| l.contains("2001")).unwrap();
        let p2004 = lines.iter().position(|l| l.contains("2004")).unwrap();
        assert!(
            p2001 < p2004,
            "expected 2001 before 2004 (vertical), got:\n{out}"
        );
    }

    #[test]
    fn render_section_label_visible() {
        let src = "timeline\n    section MySection\n      2001 : E1";
        let out = render(src);
        assert!(
            out.contains("MySection"),
            "section label missing, got:\n{out}"
        );
    }

    #[test]
    fn render_each_event_present() {
        let src =
            "timeline\n    section S\n      2001 : Alpha\n      2004 : Beta\n           : Gamma";
        let out = render(src);
        for ev in ["Alpha", "Beta", "Gamma"] {
            assert!(out.contains(ev), "event {ev} missing, got:\n{out}");
        }
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
        let src = "timeline\n    title T\n    section S\n      2001 : Alpha\n      2004 : Beta";
        let diag = crate::mermaid::ascii::timeline::parser::parse(src).unwrap();
        let rows =
            super::render_styled(&diag, 100, crate::mermaid::ascii::CharsetKind::Unicode).unwrap();
        let colors = distinct_colors(&rows);
        assert!(
            colors.len() >= 2,
            "expected at least 2 distinct colors, got {colors:?}"
        );
    }

    #[test]
    fn render_styled_specific_element_colored() {
        let src = "timeline\n    section S\n      2001 : Alpha\n      2004 : Beta";
        let diag = crate::mermaid::ascii::timeline::parser::parse(src).unwrap();
        let rows =
            super::render_styled(&diag, 100, crate::mermaid::ascii::CharsetKind::Unicode).unwrap();
        // First row = top border with section name S → Blue
        let border_color = find_color_at(&rows[0], '┌');
        assert_eq!(border_color, Some(Color::Blue));
        // Period text 2001 → Yellow
        let mut found_period = None;
        for row in &rows {
            for run in row {
                if run.text.contains("2001") {
                    found_period = run.color;
                }
            }
        }
        assert_eq!(found_period, Some(Color::Yellow));
        // Marker ● → Magenta
        let mut found_marker = None;
        for row in &rows {
            for run in row {
                if run.text.contains('●') {
                    found_marker = run.color;
                }
            }
        }
        assert_eq!(found_marker, Some(Color::Magenta));
    }

    #[test]
    fn render_styled_no_alignment_drift() {
        let src = "timeline\n    title History of Tech\n    section 2000s\n      2001 : Wikipedia launched\n      2004 : Facebook\n    section 2010s\n      2010 : Instagram\n      2012 : Pinterest";
        let diag = crate::mermaid::ascii::timeline::parser::parse(src).unwrap();
        let plain = crate::mermaid::ascii::timeline::render(
            src,
            100,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap();
        let styled =
            super::render_styled(&diag, 100, crate::mermaid::ascii::CharsetKind::Unicode).unwrap();
        assert_eq!(plain.len(), styled.len());
        for (i, (line, row)) in plain.iter().zip(styled.iter()).enumerate() {
            let flat: String = row.iter().map(|r| r.text.as_str()).collect();
            let flat = flat.trim_end().to_string();
            assert_eq!(&flat, line, "row {i} mismatch");
        }
    }
}
