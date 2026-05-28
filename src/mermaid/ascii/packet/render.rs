use super::ast::{PacketDiagram, PacketField};
use crate::mermaid::ascii::canvas::Canvas;
use crate::mermaid::ascii::{AsciiRenderError, CharsetKind, StyledRow};
use ratatui::style::Color;
use unicode_width::UnicodeWidthStr;

const CELL_WIDTH: usize = 2;

pub fn render(
    diag: &PacketDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_lines())
}

pub fn render_styled(
    diag: &PacketDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_styled_lines())
}

fn render_to_canvas(
    diag: &PacketDiagram,
    _max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    if diag.fields.is_empty() {
        return Err(AsciiRenderError::Empty);
    }
    let bpr = if diag.bits_per_row == 0 {
        32
    } else {
        diag.bits_per_row as usize
    };

    let max_bit = diag.fields.iter().map(|f| f.end_bit).max().unwrap_or(0) as usize;
    let last_row = max_bit / bpr;
    let total_rows = last_row + 1;

    let row_width = bpr * CELL_WIDTH + 1;
    let label_width = compute_label_col_width(diag, bpr);

    let cs = pick_chars(charset);

    // Pre-compute all lines first so we know canvas dimensions.
    struct LineSpec {
        text: String,
        kind: LineKind,
    }
    enum LineKind {
        Title,
        Blank,
        BitsAxis,
        BoxTop,
        FieldRow { row_idx: usize, label_text: String },
        BoxBottom,
        RowSeparator,
    }

    let mut lines: Vec<LineSpec> = Vec::new();

    if let Some(t) = &diag.title {
        lines.push(LineSpec {
            text: t.clone(),
            kind: LineKind::Title,
        });
        lines.push(LineSpec {
            text: String::new(),
            kind: LineKind::Blank,
        });
    }
    lines.push(LineSpec {
        text: render_bit_axis(bpr, label_width),
        kind: LineKind::BitsAxis,
    });

    for row_idx in 0..total_rows {
        let row_start_bit = row_idx * bpr;
        let row_end_bit = row_start_bit + bpr - 1;

        let (top, mid, bot) = render_row_lines(diag, row_idx, bpr, row_width, &cs);
        let label = format_row_label(row_start_bit, row_end_bit, label_width);
        let pad = " ".repeat(label_width);

        if row_idx == 0 {
            lines.push(LineSpec {
                text: format!("{pad}{top}"),
                kind: LineKind::BoxTop,
            });
        }
        lines.push(LineSpec {
            text: format!("{label}{mid}"),
            kind: LineKind::FieldRow {
                row_idx,
                label_text: label.clone(),
            },
        });
        if row_idx + 1 == total_rows {
            lines.push(LineSpec {
                text: format!("{pad}{bot}"),
                kind: LineKind::BoxBottom,
            });
        } else {
            let sep = render_row_separator(diag, row_idx, bpr, row_width, &cs);
            lines.push(LineSpec {
                text: format!("{pad}{sep}"),
                kind: LineKind::RowSeparator,
            });
        }
    }

    let canvas_w = lines
        .iter()
        .map(|l| UnicodeWidthStr::width(l.text.as_str()))
        .max()
        .unwrap_or(1)
        .max(1);
    let canvas_h = lines.len().max(1);
    let mut canvas = Canvas::new(canvas_w, canvas_h, charset);

    for (y, line) in lines.iter().enumerate() {
        canvas.put_str(0, y, &line.text);
        match &line.kind {
            LineKind::Title => {
                canvas.paint_text(0, y, &line.text, Color::White);
            }
            LineKind::Blank => {}
            LineKind::BitsAxis => {
                canvas.paint_text(0, y, &line.text, Color::DarkGray);
            }
            LineKind::BoxTop | LineKind::BoxBottom | LineKind::RowSeparator => {
                // Border: starts at label_width offset.
                for x in label_width..(label_width + row_width) {
                    canvas.set_color(x, y, Color::Cyan);
                }
            }
            LineKind::FieldRow {
                row_idx,
                label_text,
            } => {
                let lbl_trim = label_text.trim_end_matches(' ');
                if !lbl_trim.is_empty() {
                    canvas.paint_text(0, y, lbl_trim, Color::Green);
                }
                for &b in &boundaries_in_row(diag, *row_idx, bpr) {
                    let col = b * CELL_WIDTH;
                    let pos = col.min(row_width - 1);
                    canvas.set_color(label_width + pos, y, Color::Cyan);
                }
            }
        }
    }

    Ok(canvas)
}

struct ChSet {
    horiz: char,
    vert: char,
    tl: char,
    tr: char,
    bl: char,
    br: char,
    tee_down: char,
    tee_up: char,
    tee_right: char,
    tee_left: char,
    cross: char,
}

fn pick_chars(kind: CharsetKind) -> ChSet {
    match kind {
        CharsetKind::Unicode => ChSet {
            horiz: '─',
            vert: '│',
            tl: '┌',
            tr: '┐',
            bl: '└',
            br: '┘',
            tee_down: '┬',
            tee_up: '┴',
            tee_right: '├',
            tee_left: '┤',
            cross: '┼',
        },
        CharsetKind::Ascii => ChSet {
            horiz: '-',
            vert: '|',
            tl: '+',
            tr: '+',
            bl: '+',
            br: '+',
            tee_down: '+',
            tee_up: '+',
            tee_right: '+',
            tee_left: '+',
            cross: '+',
        },
    }
}

fn compute_label_col_width(diag: &PacketDiagram, bpr: usize) -> usize {
    let max_bit = diag.fields.iter().map(|f| f.end_bit).max().unwrap_or(0) as usize;
    let total_rows = max_bit / bpr + 1;
    let last_row_end = (total_rows - 1) * bpr + bpr - 1;
    let widest = format!("{}-{} ", (total_rows - 1) * bpr, last_row_end).len();
    widest.max(2)
}

fn render_bit_axis(bpr: usize, label_width: usize) -> String {
    let mut row = vec![' '; label_width + bpr * CELL_WIDTH + 1];
    let pad = label_width;
    let label = "bits ";
    let axis_origin = pad.saturating_sub(label.len());
    write_at(&mut row, axis_origin, label);
    let marks = [0usize, 8, 16, 24, bpr.saturating_sub(1)];
    for &m in marks.iter() {
        if m >= bpr {
            continue;
        }
        let col = pad + 1 + m * CELL_WIDTH;
        let s = m.to_string();
        let start = col.saturating_sub(s.len() / 2).max(pad);
        write_at(&mut row, start, &s);
    }
    let s: String = row.into_iter().collect();
    s.trim_end().to_string()
}

fn write_at(row: &mut [char], x: usize, s: &str) {
    let chars: Vec<char> = s.chars().collect();
    for (i, c) in chars.into_iter().enumerate() {
        if x + i < row.len() {
            row[x + i] = c;
        }
    }
}

fn boundaries_in_row(diag: &PacketDiagram, row_idx: usize, bpr: usize) -> Vec<usize> {
    let row_start = row_idx * bpr;
    let row_end = row_start + bpr - 1;
    let mut set: Vec<usize> = Vec::new();
    set.push(0);
    set.push(bpr);
    for f in &diag.fields {
        let s = f.start_bit as usize;
        let e = f.end_bit as usize;
        if e < row_start || s > row_end {
            continue;
        }
        let local_s = s.saturating_sub(row_start);
        let local_e = (e.min(row_end)) - row_start;
        if s >= row_start {
            set.push(local_s);
        }
        if e <= row_end {
            set.push(local_e + 1);
        }
    }
    set.sort_unstable();
    set.dedup();
    set
}

fn label_segments_in_row(
    diag: &PacketDiagram,
    row_idx: usize,
    bpr: usize,
) -> Vec<(usize, usize, &PacketField)> {
    let row_start = row_idx * bpr;
    let row_end = row_start + bpr - 1;
    let mut segs: Vec<(usize, usize, &PacketField)> = Vec::new();
    for f in &diag.fields {
        let s = f.start_bit as usize;
        let e = f.end_bit as usize;
        if e < row_start || s > row_end {
            continue;
        }
        let local_s = s.max(row_start) - row_start;
        let local_e = e.min(row_end) - row_start;
        segs.push((local_s, local_e, f));
    }
    segs
}

fn render_row_lines(
    diag: &PacketDiagram,
    row_idx: usize,
    bpr: usize,
    row_width: usize,
    cs: &ChSet,
) -> (String, String, String) {
    let bounds = boundaries_in_row(diag, row_idx, bpr);
    let segs = label_segments_in_row(diag, row_idx, bpr);

    let mut top = vec![cs.horiz; row_width];
    let mut mid = vec![' '; row_width];
    let mut bot = vec![cs.horiz; row_width];

    for &b in &bounds {
        let col = b * CELL_WIDTH;
        let pos = col.min(row_width - 1);
        top[pos] = if pos == 0 {
            cs.tl
        } else if pos == row_width - 1 {
            cs.tr
        } else {
            cs.tee_down
        };
        bot[pos] = if pos == 0 {
            cs.bl
        } else if pos == row_width - 1 {
            cs.br
        } else {
            cs.tee_up
        };
        mid[pos] = cs.vert;
    }

    for (ls, le, f) in segs {
        let col_l = ls * CELL_WIDTH;
        let col_r = (le + 1) * CELL_WIDTH;
        let inner_left = col_l + 1;
        let inner_right = col_r.saturating_sub(1);
        if inner_right <= inner_left {
            continue;
        }
        let inner_w = inner_right - inner_left;
        let label = truncate_to_width(&f.label, inner_w);
        let label_w = UnicodeWidthStr::width(label.as_str());
        let pad = inner_w.saturating_sub(label_w);
        let left_pad = pad / 2;
        let start_col = inner_left + left_pad;
        let chars: Vec<char> = label.chars().collect();
        let mut cx = start_col;
        for c in chars {
            let w = unicode_width::UnicodeWidthChar::width(c)
                .unwrap_or(1)
                .max(1);
            if cx >= row_width {
                break;
            }
            mid[cx] = c;
            cx += w;
        }
    }

    (
        top.into_iter().collect(),
        mid.into_iter().collect(),
        bot.into_iter().collect(),
    )
}

fn render_row_separator(
    diag: &PacketDiagram,
    row_idx: usize,
    bpr: usize,
    row_width: usize,
    cs: &ChSet,
) -> String {
    let bounds_top = boundaries_in_row(diag, row_idx, bpr);
    let bounds_bot = boundaries_in_row(diag, row_idx + 1, bpr);

    let mut line = vec![cs.horiz; row_width];

    let mut top_set: Vec<usize> = bounds_top.iter().map(|b| b * CELL_WIDTH).collect();
    let mut bot_set: Vec<usize> = bounds_bot.iter().map(|b| b * CELL_WIDTH).collect();
    top_set.sort_unstable();
    bot_set.sort_unstable();
    let mut all: Vec<usize> = top_set.iter().chain(bot_set.iter()).copied().collect();
    all.sort_unstable();
    all.dedup();

    for pos in all {
        if pos >= row_width {
            continue;
        }
        let in_top = top_set.binary_search(&pos).is_ok();
        let in_bot = bot_set.binary_search(&pos).is_ok();
        let is_left = pos == 0;
        let is_right = pos == row_width - 1;

        line[pos] = match (in_top, in_bot, is_left, is_right) {
            (true, true, true, _) => cs.tee_right,
            (true, true, _, true) => cs.tee_left,
            (true, true, _, _) => cs.cross,
            (true, false, true, _) => cs.bl,
            (true, false, _, true) => cs.br,
            (true, false, _, _) => cs.tee_up,
            (false, true, true, _) => cs.tl,
            (false, true, _, true) => cs.tr,
            (false, true, _, _) => cs.tee_down,
            _ => cs.horiz,
        };
    }

    line.into_iter().collect()
}

fn format_row_label(start: usize, end: usize, width: usize) -> String {
    let s = format!("{start}-{end} ");
    if s.len() >= width {
        s
    } else {
        let mut out = s;
        while out.len() < width {
            out.push(' ');
        }
        out
    }
}

fn truncate_to_width(s: &str, max_w: usize) -> String {
    let w = UnicodeWidthStr::width(s);
    if w <= max_w {
        return s.to_string();
    }
    if max_w == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut used = 0usize;
    for c in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(c)
            .unwrap_or(1)
            .max(1);
        if used + cw + 1 > max_w {
            break;
        }
        out.push(c);
        used += cw;
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mermaid::ascii::packet::parser::parse;

    fn render_str(src: &str) -> String {
        let d = parse(src).unwrap();
        render(&d, 80, CharsetKind::Unicode).unwrap().join("\n")
    }

    fn render_ascii_str(src: &str) -> String {
        let d = parse(src).unwrap();
        render(&d, 80, CharsetKind::Ascii).unwrap().join("\n")
    }

    #[test]
    fn render_packet_minimal() {
        let src = "packet-beta\n0-31: \"Header\"\n";
        insta::assert_snapshot!(render_str(src));
    }

    #[test]
    fn render_with_title() {
        let src =
            "packet-beta\ntitle TCP Packet\n0-15: \"Source Port\"\n16-31: \"Destination Port\"\n";
        insta::assert_snapshot!(render_str(src));
    }

    #[test]
    fn render_single_field() {
        let src = "packet-beta\n0-7: \"Type\"\n";
        insta::assert_snapshot!(render_str(src));
    }

    #[test]
    fn render_two_fields_same_row() {
        let src = "packet-beta\n0-15: \"Source Port\"\n16-31: \"Destination Port\"\n";
        insta::assert_snapshot!(render_str(src));
    }

    #[test]
    fn render_field_spanning_rows() {
        let src = "packet-beta\n0-15: \"A\"\n16-31: \"B\"\n32-63: \"Big Field\"\n";
        insta::assert_snapshot!(render_str(src));
    }

    #[test]
    fn render_with_label_visible() {
        let src = "packet-beta\n0-31: \"My Header\"\n";
        let out = render_str(src);
        assert!(out.contains("My Header"), "label not visible:\n{out}");
    }

    #[test]
    fn render_ascii_charset() {
        let src = "packet-beta\n0-15: \"A\"\n16-31: \"B\"\n";
        insta::assert_snapshot!(render_ascii_str(src));
    }

    #[test]
    fn render_has_bit_numbers() {
        let src = "packet-beta\n0-31: \"X\"\n";
        let out = render_str(src);
        assert!(out.contains("bits"), "missing 'bits' axis label:\n{out}");
        assert!(out.contains(" 0 ") || out.starts_with("0") || out.contains("  0 "));
        assert!(out.contains("31"), "missing bit 31 marker:\n{out}");
    }

    #[test]
    fn render_fields_aligned_to_bits() {
        let src = "packet-beta\n0-15: \"A\"\n16-31: \"B\"\n";
        let d = parse(src).unwrap();
        let out = render(&d, 80, CharsetKind::Unicode).unwrap();
        let mid = out
            .iter()
            .find(|l| l.contains('A') && l.contains('B'))
            .unwrap();
        let a_idx = mid.find('A').unwrap();
        let b_idx = mid.find('B').unwrap();
        assert!(a_idx < b_idx, "A should appear before B in row:\n{mid}");
    }

    #[test]
    fn render_labels_visible() {
        let src = "packet-beta\n0-15: \"Source Port\"\n16-31: \"Destination Port\"\n32-63: \"Sequence Number\"\n";
        let out = render_str(src);
        assert!(out.contains("Source Port"), "missing Source Port:\n{out}");
        assert!(
            out.contains("Destination Port") || out.contains("Destinati"),
            "missing dest port (possibly truncated):\n{out}"
        );
        assert!(
            out.contains("Sequence Number"),
            "missing Sequence Number:\n{out}"
        );
    }

    fn render_styled_str(src: &str) -> Vec<StyledRow> {
        let d = parse(src).unwrap();
        render_styled(&d, 80, CharsetKind::Unicode).unwrap()
    }

    #[test]
    fn render_styled_attaches_colors() {
        let src = "packet-beta\n0-31: \"Header\"\n";
        let rows = render_styled_str(src);
        let any_colored = rows
            .iter()
            .flat_map(|r| r.iter())
            .any(|r| r.color.is_some());
        assert!(any_colored, "expected at least one colored run");
    }

    #[test]
    fn render_styled_specific_element_colored() {
        let src = "packet-beta\ntitle TCP\n0-15: \"A\"\n16-31: \"B\"\n";
        let rows = render_styled_str(src);
        let mut has_white_title = false;
        let mut has_darkgray_bits = false;
        let mut has_cyan_border = false;
        let mut has_green_range = false;
        for row in &rows {
            for run in row {
                if run.color == Some(Color::White) && run.text.contains("TCP") {
                    has_white_title = true;
                }
                if run.color == Some(Color::DarkGray) && run.text.contains("bits") {
                    has_darkgray_bits = true;
                }
                if run.color == Some(Color::Cyan)
                    && run.text.chars().any(|c| {
                        matches!(
                            c,
                            '┌' | '┐'
                                | '└'
                                | '┘'
                                | '─'
                                | '│'
                                | '┬'
                                | '┴'
                                | '├'
                                | '┤'
                                | '┼'
                                | '+'
                                | '-'
                                | '|'
                        )
                    })
                {
                    has_cyan_border = true;
                }
                if run.color == Some(Color::Green)
                    && run.text.contains('-')
                    && run.text.chars().any(|c| c.is_ascii_digit())
                {
                    has_green_range = true;
                }
            }
        }
        assert!(has_white_title, "title should be white");
        assert!(has_darkgray_bits, "bits axis should be dark gray");
        assert!(has_cyan_border, "field borders should be cyan");
        assert!(has_green_range, "bit range label should be green");
    }

    #[test]
    fn render_styled_no_alignment_drift() {
        let src = "packet-beta\ntitle TCP\n0-15: \"A\"\n16-31: \"B\"\n32-63: \"Big\"\n";
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
