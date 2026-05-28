use super::ast::{Block, BlockDiagram, BlockShape};
use crate::mermaid::ascii::canvas::Canvas;
use crate::mermaid::ascii::charset::get_charset;
use crate::mermaid::ascii::{AsciiRenderError, CharsetKind, StyledRow};
use ratatui::style::Color;
use unicode_width::UnicodeWidthStr;

const PAD_X: usize = 2;
const COL_GAP: usize = 2;
const ROW_GAP: usize = 1;
const EDGE_GAP: usize = 1;

pub fn render(
    d: &BlockDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    render_to_canvas(d, max_width, charset).map(|c| c.into_lines())
}

pub fn render_styled(
    d: &BlockDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    render_to_canvas(d, max_width, charset).map(|c| c.into_styled_lines())
}

fn render_to_canvas(
    d: &BlockDiagram,
    _max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    if d.blocks.is_empty() && d.edges.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    let columns = if d.columns > 0 {
        d.columns as usize
    } else if !d.blocks.is_empty() {
        d.blocks.len()
    } else {
        1
    };

    let rows: Vec<Vec<&Block>> = chunk_rows(&d.blocks, columns);
    let row_count = rows.len();

    let mut col_widths: Vec<usize> = vec![0; columns];
    let mut row_heights: Vec<usize> = vec![0; row_count];

    for (r, row) in rows.iter().enumerate() {
        for (c, b) in row.iter().enumerate() {
            let (bw, bh) = block_box_size(b);
            if c < col_widths.len() && bw > col_widths[c] {
                col_widths[c] = bw;
            }
            if r < row_heights.len() && bh > row_heights[r] {
                row_heights[r] = bh;
            }
        }
    }

    if col_widths.iter().all(|&w| w == 0) {
        for w in col_widths.iter_mut() {
            *w = 3;
        }
    }
    if row_heights.iter().all(|&h| h == 0) {
        for h in row_heights.iter_mut() {
            *h = 3;
        }
    }

    let total_width: usize = col_widths.iter().sum::<usize>() + COL_GAP * columns.saturating_sub(1);
    let total_height: usize =
        row_heights.iter().sum::<usize>() + ROW_GAP * row_count.saturating_sub(1);

    let edge_height: usize = if d.edges.is_empty() {
        0
    } else {
        EDGE_GAP + d.edges.len()
    };

    let canvas_w = total_width.max(1);
    let canvas_h = (total_height + edge_height).max(1);
    let mut canvas = Canvas::new(canvas_w, canvas_h, charset);

    let mut col_x: Vec<usize> = Vec::with_capacity(columns);
    let mut acc = 0usize;
    for (i, w) in col_widths.iter().enumerate() {
        col_x.push(acc);
        acc += w;
        if i + 1 < columns {
            acc += COL_GAP;
        }
    }
    let mut row_y: Vec<usize> = Vec::with_capacity(row_count);
    let mut acc_y = 0usize;
    for (i, h) in row_heights.iter().enumerate() {
        row_y.push(acc_y);
        acc_y += h;
        if i + 1 < row_count {
            acc_y += ROW_GAP;
        }
    }

    for (r, row) in rows.iter().enumerate() {
        for (c, b) in row.iter().enumerate() {
            let cell_x = col_x[c];
            let cell_y = row_y[r];
            let cell_w = col_widths[c];
            let cell_h = row_heights[r];
            let (bw, bh) = block_box_size(b);
            let off_x = cell_x + (cell_w.saturating_sub(bw)) / 2;
            let off_y = cell_y + (cell_h.saturating_sub(bh)) / 2;
            draw_block(&mut canvas, charset, b, off_x, off_y, bw, bh);
        }
    }

    if !d.edges.is_empty() {
        let cs = get_charset(charset);
        let mut ey = total_height + EDGE_GAP;
        for edge in &d.edges {
            // Edge text consists of:  from " " hline arrow ( "|" label "|" )? " " to
            // We paint: the arrow chars yellow, the label green.
            let arrow_glyphs = format!("{}{}", cs.horiz(), cs.arrow_right());
            let mut x = 0usize;
            canvas.put_str(x, ey, &edge.from);
            x += UnicodeWidthStr::width(edge.from.as_str()) + 1; // space
                                                                 // arrow glyphs (─►)
            canvas.put_str_colored(x, ey, &arrow_glyphs, Color::Yellow);
            x += UnicodeWidthStr::width(arrow_glyphs.as_str());
            if !edge.label.is_empty() {
                let lbl_segment = format!("|{}|", edge.label);
                canvas.put_str(x, ey, "|");
                x += 1;
                canvas.put_str_colored(x, ey, &edge.label, Color::Green);
                x += UnicodeWidthStr::width(edge.label.as_str());
                canvas.put_str(x, ey, "|");
                x += 1;
                let _ = lbl_segment;
            }
            canvas.put_str(x, ey, " ");
            x += 1;
            canvas.put_str(x, ey, &edge.to);
            ey += 1;
        }
    }

    Ok(canvas)
}

fn chunk_rows<'a>(blocks: &'a [Block], columns: usize) -> Vec<Vec<&'a Block>> {
    if columns == 0 {
        return vec![blocks.iter().collect()];
    }
    let mut rows: Vec<Vec<&'a Block>> = Vec::new();
    let mut cur: Vec<&'a Block> = Vec::new();
    for b in blocks {
        cur.push(b);
        if cur.len() == columns {
            rows.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        rows.push(cur);
    }
    rows
}

fn block_box_size(b: &Block) -> (usize, usize) {
    let label = if b.label.is_empty() { &b.id } else { &b.label };
    let label_w = UnicodeWidthStr::width(label.as_str()).max(1);
    let inner_w = label_w + PAD_X * 2;
    let w = inner_w + 2;
    let extra = match b.shape {
        BlockShape::Circle => 2,
        BlockShape::Stadium => 2,
        _ => 0,
    };
    (w + extra, 3)
}

fn draw_block(
    canvas: &mut Canvas,
    charset: CharsetKind,
    b: &Block,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
) {
    let cs = get_charset(charset);
    let x1 = x;
    let y1 = y;
    let x2 = x + w.saturating_sub(1);
    let y2 = y + h.saturating_sub(1);

    canvas.draw_box(x1, y1, x2, y2);
    canvas.paint_box(x1, y1, x2, y2, Color::Cyan);

    match b.shape {
        BlockShape::Round | BlockShape::Stadium | BlockShape::Circle => {
            canvas.put_char(x1, y1, cs.round_top_left());
            canvas.put_char(x2, y1, cs.round_top_right());
            canvas.put_char(x1, y2, cs.round_bot_left());
            canvas.put_char(x2, y2, cs.round_bot_right());
        }
        BlockShape::Rhombus => {
            canvas.put_char(x1, y1, cs.diag_up());
            canvas.put_char(x2, y1, cs.diag_down());
            canvas.put_char(x1, y2, cs.diag_down());
            canvas.put_char(x2, y2, cs.diag_up());
        }
        BlockShape::Square => {}
    }

    if matches!(b.shape, BlockShape::Stadium) {
        canvas.put_char(x1, y1 + 1, cs.vert());
        canvas.put_char(x2, y1 + 1, cs.vert());
        canvas.set_color(x1, y1 + 1, Color::Cyan);
        canvas.set_color(x2, y1 + 1, Color::Cyan);
    }
    if matches!(b.shape, BlockShape::Circle) {
        let inner_left = x1 + 1;
        let inner_right = x2.saturating_sub(1);
        canvas.put_char(inner_left, y1 + 1, '(');
        canvas.put_char(inner_right, y1 + 1, ')');
        canvas.set_color(inner_left, y1 + 1, Color::Cyan);
        canvas.set_color(inner_right, y1 + 1, Color::Cyan);
    }

    let label = if b.label.is_empty() { &b.id } else { &b.label };
    let label_w = UnicodeWidthStr::width(label.as_str());
    let inner_w = w.saturating_sub(2);
    let lx = x1 + 1 + inner_w.saturating_sub(label_w) / 2;
    let ly = y1 + h / 2;
    canvas.put_str(lx, ly, label);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mermaid::ascii::block::parser::parse;

    fn render_src(src: &str) -> String {
        let d = parse(src).unwrap();
        render(&d, 200, CharsetKind::Unicode).unwrap().join("\n")
    }

    fn render_ascii(src: &str) -> String {
        let d = parse(src).unwrap();
        render(&d, 200, CharsetKind::Ascii).unwrap().join("\n")
    }

    #[test]
    fn render_block_minimal() {
        insta::assert_snapshot!(render_src("block-beta\na"));
    }

    #[test]
    fn render_two_columns() {
        insta::assert_snapshot!(render_src("block-beta\ncolumns 2\na b c d"));
    }

    #[test]
    fn render_three_columns_specified() {
        insta::assert_snapshot!(render_src(
            "block-beta\ncolumns 3\na[\"Hello\"] b(\"Round\") c((\"Circle\"))"
        ));
    }

    #[test]
    fn render_with_round_block() {
        insta::assert_snapshot!(render_src("block-beta\nb(\"Round\")"));
    }

    #[test]
    fn render_with_diamond() {
        insta::assert_snapshot!(render_src("block-beta\nd{\"Decide\"}"));
    }

    #[test]
    fn render_with_circle() {
        insta::assert_snapshot!(render_src("block-beta\nc((\"Center\"))"));
    }

    #[test]
    fn render_with_edge() {
        insta::assert_snapshot!(render_src("block-beta\na b\na --> b"));
    }

    #[test]
    fn render_ascii_charset() {
        insta::assert_snapshot!(render_ascii(
            "block-beta\ncolumns 2\na[\"Hello\"] b(\"Round\")"
        ));
    }

    // Non-snapshot unit tests.

    #[test]
    fn render_blocks_arranged_in_rows() {
        let d = parse("block-beta\ncolumns 2\na b c d").unwrap();
        let lines = render(&d, 200, CharsetKind::Unicode).unwrap();
        assert!(lines.len() >= 7, "expected >=7 lines, got {}", lines.len());
    }

    #[test]
    fn render_each_shape_distinct() {
        let sq = render_src("block-beta\na");
        let rd = render_src("block-beta\nb(\"x\")");
        let di = render_src("block-beta\nc{\"x\"}");
        let ci = render_src("block-beta\nd((\"x\"))");
        let st = render_src("block-beta\ne([\"x\"])");
        let mut all = vec![&sq, &rd, &di, &ci, &st];
        all.sort();
        all.dedup();
        assert_eq!(all.len(), 5, "shapes should all render distinctly");
    }

    #[test]
    fn render_columns_respected() {
        let d = parse("block-beta\ncolumns 3\na b c d e f").unwrap();
        let lines = render(&d, 200, CharsetKind::Unicode).unwrap();
        let joined = lines.join("\n");
        assert!(
            lines.len() >= 7,
            "expected at least 7 lines for 2 rows of blocks"
        );
        assert!(joined.contains('a'));
        assert!(joined.contains('f'));
    }

    #[test]
    fn render_edge_label_visible() {
        let lines = render(
            &parse("block-beta\na b\na -->|hello| b").unwrap(),
            200,
            CharsetKind::Unicode,
        )
        .unwrap();
        let joined = lines.join("\n");
        assert!(
            joined.contains("hello"),
            "edge label should appear in output"
        );
    }

    fn render_styled_str(src: &str) -> Vec<StyledRow> {
        let d = parse(src).unwrap();
        render_styled(&d, 200, CharsetKind::Unicode).unwrap()
    }

    #[test]
    fn render_styled_attaches_colors() {
        let src = "block-beta\na";
        let rows = render_styled_str(src);
        let any_colored = rows
            .iter()
            .flat_map(|r| r.iter())
            .any(|r| r.color.is_some());
        assert!(any_colored, "expected at least one colored run");
    }

    #[test]
    fn render_styled_specific_element_colored() {
        let src = "block-beta\na b\na -->|lbl| b";
        let rows = render_styled_str(src);
        let mut has_cyan_border = false;
        let mut has_yellow_arrow = false;
        let mut has_green_label = false;
        for row in &rows {
            for run in row {
                if run.color == Some(Color::Cyan)
                    && run
                        .text
                        .chars()
                        .any(|c| matches!(c, '┌' | '┐' | '└' | '┘' | '─' | '│' | '+' | '-' | '|'))
                {
                    has_cyan_border = true;
                }
                if run.color == Some(Color::Yellow)
                    && (run.text.contains('►') || run.text.contains('>'))
                {
                    has_yellow_arrow = true;
                }
                if run.color == Some(Color::Green) && run.text.contains("lbl") {
                    has_green_label = true;
                }
            }
        }
        assert!(has_cyan_border, "block borders should be cyan");
        assert!(has_yellow_arrow, "edge arrow should be yellow");
        assert!(has_green_label, "edge label should be green");
    }

    #[test]
    fn render_styled_no_alignment_drift() {
        let src = "block-beta\ncolumns 2\na b c d\na -->|hi| b";
        let d = parse(src).unwrap();
        let plain = render(&d, 200, CharsetKind::Unicode).unwrap();
        let styled = render_styled(&d, 200, CharsetKind::Unicode).unwrap();
        assert_eq!(plain.len(), styled.len());
        for (i, row) in styled.iter().enumerate() {
            let flat: String = row.iter().map(|r| r.text.as_str()).collect();
            let flat_trimmed = flat.trim_end_matches(' ').to_string();
            assert_eq!(flat_trimmed, plain[i], "row {i} differs");
        }
    }
}
