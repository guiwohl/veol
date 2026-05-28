use super::ast::{
    ArrowKind, Block, BlockKind, DiagramItem, Message, Note, NotePosition, SequenceDiagram,
};
use crate::mermaid::ascii::canvas::Canvas;
use crate::mermaid::ascii::charset::{get_charset, Charset, CharsetKind};
use crate::mermaid::ascii::error::AsciiRenderError;
use crate::mermaid::ascii::StyledRow;
use ratatui::style::Color;
use unicode_width::UnicodeWidthStr;

const PARTICIPANT_SPACING: usize = 5;
const SELF_MESSAGE_WIDTH: usize = 4;
const BOX_PADDING: usize = 2;
const MIN_BOX_WIDTH: usize = 3;
const BOX_BORDER: usize = 2;
const LABEL_LEFT_MARGIN: usize = 2;

struct Layout {
    widths: Vec<usize>,
    centers: Vec<usize>,
    total_width: usize,
}

fn calculate_layout(diag: &SequenceDiagram) -> Layout {
    let mut widths = Vec::with_capacity(diag.participants.len());
    for p in &diag.participants {
        let w = (UnicodeWidthStr::width(p.label.as_str()) + BOX_PADDING).max(MIN_BOX_WIDTH);
        widths.push(w);
    }
    let mut centers = Vec::with_capacity(diag.participants.len());
    let mut cur_x: usize = 0;
    for (i, _p) in diag.participants.iter().enumerate() {
        let box_w = widths[i] + BOX_BORDER;
        if i == 0 {
            centers.push(box_w / 2);
            cur_x = box_w;
        } else {
            cur_x += PARTICIPANT_SPACING;
            centers.push(cur_x + box_w / 2);
            cur_x += box_w;
        }
    }
    let total_width = if let Some(&last) = centers.last() {
        let last_w = *widths.last().unwrap();
        last + (last_w + BOX_BORDER) / 2
    } else {
        0
    };
    Layout {
        widths,
        centers,
        total_width,
    }
}

struct RenderState<'a> {
    layout: &'a Layout,
    cs: &'static dyn Charset,
    charset_kind: CharsetKind,
    auto_num: bool,
    auto_counter: usize,
    activations: Vec<u32>,
}

pub fn render(
    diag: &SequenceDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    let canvas = render_to_canvas(diag, max_width, charset)?;
    let lines: Vec<String> = canvas
        .into_lines()
        .into_iter()
        .map(|l| trim_to_max(l, max_width as usize))
        .collect();
    Ok(strip_trailing_blank(lines))
}

pub fn render_styled(
    diag: &SequenceDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    let canvas = render_to_canvas(diag, max_width, charset)?;
    let rows: Vec<StyledRow> = canvas
        .into_styled_lines()
        .into_iter()
        .map(|r| trim_styled_to_max(r, max_width as usize))
        .collect();
    Ok(strip_trailing_blank_styled(rows))
}

fn render_to_canvas(
    diag: &SequenceDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    if diag.participants.is_empty() && diag.items.is_empty() {
        return Err(AsciiRenderError::Empty);
    }
    if diag.participants.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    let layout = calculate_layout(diag);
    let cs = get_charset(charset);

    let canvas_w = (layout.total_width + 4).max(max_width as usize);
    let est_h = estimate_height(&diag.items, 0);
    let total_h = 3 + est_h + 2;
    let mut canvas = Canvas::new(canvas_w, total_h, charset);

    draw_header(&mut canvas, &layout, diag, cs);

    let mut state = RenderState {
        layout: &layout,
        cs,
        charset_kind: charset,
        auto_num: diag.autonumber,
        auto_counter: 0,
        activations: vec![0; diag.participants.len()],
    };

    let mut row: usize = 3;
    for item in &diag.items {
        row = render_item(&mut canvas, &mut state, item, row);
    }

    pre_lifelines(&mut canvas, &state, row);

    Ok(canvas)
}

fn trim_styled_to_max(row: StyledRow, max: usize) -> StyledRow {
    if max == 0 {
        return row;
    }
    let total_w: usize = row
        .iter()
        .map(|r| UnicodeWidthStr::width(r.text.as_str()))
        .sum();
    if total_w <= max {
        return row;
    }
    let mut out: StyledRow = Vec::with_capacity(row.len());
    let mut w: usize = 0;
    for run in row {
        let mut acc = String::new();
        let mut done = false;
        for c in run.text.chars() {
            let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);
            if w + cw > max {
                done = true;
                break;
            }
            acc.push(c);
            w += cw;
        }
        if !acc.is_empty() {
            out.push(crate::mermaid::ascii::canvas::StyledRun {
                text: acc,
                color: run.color,
            });
        }
        if done {
            break;
        }
    }
    out
}

fn strip_trailing_blank_styled(mut rows: Vec<StyledRow>) -> Vec<StyledRow> {
    while let Some(last) = rows.last() {
        if last.is_empty() {
            rows.pop();
        } else {
            break;
        }
    }
    rows
}

fn trim_to_max(s: String, max: usize) -> String {
    if max == 0 || UnicodeWidthStr::width(s.as_str()) <= max {
        s
    } else {
        let mut acc = String::new();
        let mut w = 0;
        for c in s.chars() {
            let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);
            if w + cw > max {
                break;
            }
            acc.push(c);
            w += cw;
        }
        acc
    }
}

fn strip_trailing_blank(mut lines: Vec<String>) -> Vec<String> {
    while let Some(last) = lines.last() {
        if last.is_empty() {
            lines.pop();
        } else {
            break;
        }
    }
    lines
}

fn estimate_height(items: &[DiagramItem], depth: usize) -> usize {
    let mut h = depth;
    for it in items {
        h += match it {
            DiagramItem::Message(m) => {
                if m.from == m.to {
                    5
                } else {
                    3
                }
            }
            DiagramItem::Note(_) => 4,
            DiagramItem::Activate(_) | DiagramItem::Deactivate(_) => 1,
            DiagramItem::Block(b) => {
                let mut inner = 2;
                for (i, br) in b.branches.iter().enumerate() {
                    if i > 0 {
                        inner += 1;
                    }
                    inner += estimate_height(&br.items, depth + 1) + 1;
                }
                inner + 2
            }
        };
    }
    h + 4
}

fn draw_header(canvas: &mut Canvas, layout: &Layout, diag: &SequenceDiagram, cs: &dyn Charset) {
    for (i, p) in diag.participants.iter().enumerate() {
        let w = layout.widths[i];
        let center = layout.centers[i];
        let left = center - (w + BOX_BORDER) / 2;
        let right = left + w + BOX_BORDER - 1;

        canvas.put_char(left, 0, cs.top_left());
        for x in (left + 1)..right {
            canvas.put_char(x, 0, cs.horiz());
        }
        canvas.put_char(right, 0, cs.top_right());

        canvas.put_char(left, 1, cs.vert());
        let label_w = UnicodeWidthStr::width(p.label.as_str());
        let pad = (w.saturating_sub(label_w)) / 2;
        canvas.put_str(left + 1 + pad, 1, &p.label);
        canvas.put_char(right, 1, cs.vert());

        canvas.put_char(left, 2, cs.bot_left());
        for x in (left + 1)..right {
            canvas.put_char(x, 2, cs.horiz());
        }
        canvas.put_char(right, 2, cs.bot_right());
        canvas.put_char(center, 2, cs.tee_down());

        canvas.paint_box(left, 0, right, 2, Color::Cyan);
    }
}

fn render_item(
    canvas: &mut Canvas,
    state: &mut RenderState,
    item: &DiagramItem,
    row: usize,
) -> usize {
    match item {
        DiagramItem::Message(m) => {
            if m.from == m.to {
                render_self_message(canvas, state, m, row)
            } else {
                render_message(canvas, state, m, row)
            }
        }
        DiagramItem::Note(n) => render_note(canvas, state, n, row),
        DiagramItem::Activate(i) => {
            if *i < state.activations.len() {
                state.activations[*i] = state.activations[*i].saturating_add(1);
            }
            row
        }
        DiagramItem::Deactivate(i) => {
            if *i < state.activations.len() {
                state.activations[*i] = state.activations[*i].saturating_sub(1);
            }
            row
        }
        DiagramItem::Block(b) => render_block(canvas, state, b, row),
    }
}

fn pre_lifelines(canvas: &mut Canvas, state: &RenderState, row: usize) {
    let cs = state.cs;
    canvas.ensure_size(state.layout.total_width + 2, row + 1);
    for (i, &c) in state.layout.centers.iter().enumerate() {
        let active = state.activations[i] > 0;
        let ch = if active {
            active_vert(state.charset_kind, cs)
        } else {
            cs.vert()
        };
        if canvas.get(c, row) == ' ' {
            canvas.put_char(c, row, ch);
            canvas.set_color(c, row, if active { Color::Magenta } else { Color::Cyan });
        }
    }
}

fn active_vert(kind: CharsetKind, cs: &dyn Charset) -> char {
    match kind {
        CharsetKind::Unicode => '║',
        CharsetKind::Ascii => cs.vert(),
    }
}

fn message_label(m: &Message, state: &mut RenderState) -> String {
    let mut label = m.label.clone();
    if state.auto_num {
        state.auto_counter += 1;
        if label.is_empty() {
            label = format!("{}.", state.auto_counter);
        } else {
            label = format!("{}. {}", state.auto_counter, label);
        }
    }
    label
}

fn arrow_body_char(arrow: ArrowKind, kind: CharsetKind, cs: &dyn Charset) -> char {
    match (arrow, kind) {
        (
            ArrowKind::Dotted | ArrowKind::AsyncDotted | ArrowKind::DottedCross,
            CharsetKind::Unicode,
        ) => '┈',
        (
            ArrowKind::Dotted | ArrowKind::AsyncDotted | ArrowKind::DottedCross,
            CharsetKind::Ascii,
        ) => '.',
        _ => cs.horiz(),
    }
}

fn arrow_head_char(
    arrow: ArrowKind,
    going_right: bool,
    cs: &dyn Charset,
    kind: CharsetKind,
) -> char {
    match arrow {
        ArrowKind::SolidCross | ArrowKind::DottedCross => match kind {
            CharsetKind::Unicode => '✗',
            CharsetKind::Ascii => 'x',
        },
        ArrowKind::AsyncSolid | ArrowKind::AsyncDotted => match kind {
            CharsetKind::Unicode => {
                if going_right {
                    '►'
                } else {
                    '◄'
                }
            }
            CharsetKind::Ascii => {
                if going_right {
                    '>'
                } else {
                    '<'
                }
            }
        },
        _ => {
            if going_right {
                cs.arrow_right()
            } else {
                cs.arrow_left()
            }
        }
    }
}

fn render_message(canvas: &mut Canvas, state: &mut RenderState, m: &Message, row: usize) -> usize {
    let from = state.layout.centers[m.from];
    let to = state.layout.centers[m.to];
    let label = message_label(m, state);
    let cs = state.cs;

    pre_lifelines(canvas, state, row);

    if !label.is_empty() {
        let start = from.min(to) + LABEL_LEFT_MARGIN;
        canvas.ensure_size(start + UnicodeWidthStr::width(label.as_str()) + 4, row + 1);
        canvas.put_str(start, row, &label);
        canvas.paint_text(start, row, &label, Color::Green);
    }

    let arrow_row = row + 1;
    pre_lifelines(canvas, state, arrow_row);

    let body = arrow_body_char(m.arrow, state.charset_kind, cs);
    let going_right = from < to;
    let head = arrow_head_char(m.arrow, going_right, cs, state.charset_kind);

    if going_right {
        canvas.put_char(from, arrow_row, cs.tee_right());
        canvas.set_color(from, arrow_row, Color::Yellow);
        for x in (from + 1)..to {
            canvas.put_char(x, arrow_row, body);
            canvas.set_color(x, arrow_row, Color::Yellow);
        }
        if to > 0 {
            canvas.put_char(to - 1, arrow_row, head);
            canvas.set_color(to - 1, arrow_row, Color::Magenta);
        }
        canvas.put_char(to, arrow_row, cs.vert());
        canvas.set_color(to, arrow_row, Color::Cyan);
    } else {
        canvas.put_char(to, arrow_row, cs.vert());
        canvas.set_color(to, arrow_row, Color::Cyan);
        canvas.put_char(to + 1, arrow_row, head);
        canvas.set_color(to + 1, arrow_row, Color::Magenta);
        for x in (to + 2)..from {
            canvas.put_char(x, arrow_row, body);
            canvas.set_color(x, arrow_row, Color::Yellow);
        }
        canvas.put_char(from, arrow_row, cs.tee_left());
        canvas.set_color(from, arrow_row, Color::Yellow);
    }

    arrow_row + 1
}

fn render_self_message(
    canvas: &mut Canvas,
    state: &mut RenderState,
    m: &Message,
    row: usize,
) -> usize {
    let cs = state.cs;
    let center = state.layout.centers[m.from];
    let width = SELF_MESSAGE_WIDTH;
    let label = message_label(m, state);

    pre_lifelines(canvas, state, row);

    if !label.is_empty() {
        let start = center + LABEL_LEFT_MARGIN;
        canvas.ensure_size(start + UnicodeWidthStr::width(label.as_str()) + 4, row + 1);
        canvas.put_str(start, row, &label);
        canvas.paint_text(start, row, &label, Color::Green);
    }

    let l1 = row + 1;
    let l2 = row + 2;
    let l3 = row + 3;

    let needed = center + width + 1;
    canvas.ensure_size(needed, l3 + 1);

    pre_lifelines(canvas, state, l1);
    canvas.put_char(center, l1, cs.tee_right());
    canvas.set_color(center, l1, Color::Yellow);
    for x in 1..(width - 1) {
        canvas.put_char(center + x, l1, cs.horiz());
        canvas.set_color(center + x, l1, Color::Yellow);
    }
    canvas.put_char(center + width - 1, l1, cs.top_right());
    canvas.set_color(center + width - 1, l1, Color::Yellow);

    pre_lifelines(canvas, state, l2);
    canvas.put_char(center + width - 1, l2, cs.vert());
    canvas.set_color(center + width - 1, l2, Color::Yellow);

    pre_lifelines(canvas, state, l3);
    canvas.put_char(center, l3, cs.vert());
    canvas.set_color(center, l3, Color::Cyan);
    let head = arrow_head_char(m.arrow, false, cs, state.charset_kind);
    canvas.put_char(center + 1, l3, head);
    canvas.set_color(center + 1, l3, Color::Magenta);
    for x in 2..(width - 1) {
        canvas.put_char(center + x, l3, cs.horiz());
        canvas.set_color(center + x, l3, Color::Yellow);
    }
    canvas.put_char(center + width - 1, l3, cs.bot_right());
    canvas.set_color(center + width - 1, l3, Color::Yellow);

    l3 + 1
}

fn render_note(canvas: &mut Canvas, state: &mut RenderState, n: &Note, row: usize) -> usize {
    let cs = state.cs;
    let (x1, x2) = note_bounds(n, state);

    let top = row;
    let mid = row + 1;
    let bot = row + 2;

    canvas.ensure_size(x2 + 2, bot + 1);

    canvas.put_char(x1, top, cs.top_left());
    for x in (x1 + 1)..x2 {
        canvas.put_char(x, top, cs.horiz());
    }
    canvas.put_char(x2, top, cs.top_right());

    canvas.put_char(x1, mid, cs.vert());
    let inner_w = x2.saturating_sub(x1 + 1);
    let label_w = UnicodeWidthStr::width(n.text.as_str());
    let pad = inner_w.saturating_sub(label_w) / 2;
    for x in (x1 + 1)..x2 {
        canvas.put_char(x, mid, ' ');
    }
    canvas.put_str(x1 + 1 + pad, mid, &n.text);
    canvas.put_char(x2, mid, cs.vert());

    canvas.put_char(x1, bot, cs.bot_left());
    for x in (x1 + 1)..x2 {
        canvas.put_char(x, bot, cs.horiz());
    }
    canvas.put_char(x2, bot, cs.bot_right());

    canvas.paint_box(x1, top, x2, bot, Color::DarkGray);
    canvas.paint_text(x1 + 1 + pad, mid, &n.text, Color::DarkGray);

    bot + 1
}

fn note_bounds(n: &Note, state: &RenderState) -> (usize, usize) {
    let centers = &state.layout.centers;
    let label_w = UnicodeWidthStr::width(n.text.as_str()).max(4);
    let box_w = label_w + 4;
    if n.participants.is_empty() {
        return (0, box_w);
    }
    match n.position {
        NotePosition::LeftOf => {
            let c = centers[n.participants[0]];
            let right = c.saturating_sub(1);
            if right >= box_w {
                (right - box_w, right)
            } else {
                (0, box_w)
            }
        }
        NotePosition::RightOf => {
            let c = centers[n.participants[0]];
            let left = c + 1;
            (left, left + box_w)
        }
        NotePosition::Over => {
            if n.participants.len() == 1 {
                let c = centers[n.participants[0]];
                let half = box_w / 2;
                let left = c.saturating_sub(half);
                let right = left + box_w;
                (left, right)
            } else {
                let mut min_c = usize::MAX;
                let mut max_c = 0;
                for &i in &n.participants {
                    min_c = min_c.min(centers[i]);
                    max_c = max_c.max(centers[i]);
                }
                let left0 = min_c.saturating_sub(2);
                let right0 = max_c + 2;
                let needed_w = box_w;
                let cur_w = right0.saturating_sub(left0);
                if cur_w < needed_w {
                    let extra = (needed_w - cur_w).div_ceil(2);
                    (left0.saturating_sub(extra), right0 + extra)
                } else {
                    (left0, right0)
                }
            }
        }
    }
}

fn render_block(canvas: &mut Canvas, state: &mut RenderState, b: &Block, row: usize) -> usize {
    let cs = state.cs;
    if state.layout.centers.is_empty() {
        return row;
    }
    let lo_center = *state.layout.centers.first().unwrap();
    let hi_center = *state.layout.centers.last().unwrap();
    let left = lo_center.saturating_sub(2);

    let top_row = row;
    let mut cur = row + 1;

    let kind_str = block_kind_label(b.kind);
    let header_label = if b.label.is_empty() {
        format!(" {kind_str} ")
    } else {
        format!(" {kind_str}: {} ", b.label)
    };

    let mut max_divider_w = 0usize;
    for (bi, branch) in b.branches.iter().enumerate() {
        if bi > 0 {
            let dl = if branch.label.is_empty() {
                " else ".to_string()
            } else {
                format!(" else: {} ", branch.label)
            };
            max_divider_w = max_divider_w.max(UnicodeWidthStr::width(dl.as_str()));
        }
    }
    let header_w0 = UnicodeWidthStr::width(header_label.as_str());
    let label_needs = header_w0.max(max_divider_w) + 4;
    let right = (hi_center + 2).max(left + label_needs);

    canvas.ensure_size(right + 2, cur + 1);
    canvas.put_char(left, top_row, cs.top_left());
    for x in (left + 1)..right {
        canvas.put_char(x, top_row, cs.horiz());
    }
    canvas.put_char(right, top_row, cs.top_right());
    canvas.paint_hline(left, right, top_row, Color::Blue);
    let label_start = left + 2;
    let header_w = UnicodeWidthStr::width(header_label.as_str());
    if label_start + header_w < right {
        canvas.put_str(label_start, top_row, &header_label);
        canvas.paint_text(label_start, top_row, &header_label, Color::Blue);
    }
    for &cx in &state.layout.centers {
        if cx > left && cx < right && cx >= label_start + header_w {
            canvas.put_char(cx, top_row, cs.tee_down());
            canvas.set_color(cx, top_row, Color::Blue);
        }
    }

    for (bi, branch) in b.branches.iter().enumerate() {
        if bi > 0 {
            let divider_label = if branch.label.is_empty() {
                " else ".to_string()
            } else {
                format!(" else: {} ", branch.label)
            };
            canvas.ensure_size(right + 2, cur + 1);
            canvas.put_char(left, cur, cs.tee_right());
            for x in (left + 1)..right {
                canvas.put_char(x, cur, cs.horiz());
            }
            canvas.put_char(right, cur, cs.tee_left());
            canvas.paint_hline(left, right, cur, Color::Blue);
            let dw = UnicodeWidthStr::width(divider_label.as_str());
            if left + 2 + dw < right {
                canvas.put_str(left + 2, cur, &divider_label);
                canvas.paint_text(left + 2, cur, &divider_label, Color::Blue);
            }
            for &cx in &state.layout.centers {
                if cx > left && cx < right && cx >= left + 2 + dw {
                    canvas.put_char(cx, cur, cs.cross());
                    canvas.set_color(cx, cur, Color::Blue);
                }
            }
            cur += 1;
        }
        for item in &branch.items {
            cur = render_item(canvas, state, item, cur);
        }
    }

    canvas.ensure_size(right + 2, cur + 1);
    canvas.put_char(left, cur, cs.bot_left());
    for x in (left + 1)..right {
        canvas.put_char(x, cur, cs.horiz());
    }
    canvas.put_char(right, cur, cs.bot_right());
    canvas.paint_hline(left, right, cur, Color::Blue);
    for &cx in &state.layout.centers {
        if cx > left && cx < right {
            canvas.put_char(cx, cur, cs.tee_up());
            canvas.set_color(cx, cur, Color::Blue);
        }
    }
    canvas.paint_vline(left, top_row, cur, Color::Blue);
    canvas.paint_vline(right, top_row, cur, Color::Blue);

    cur + 1
}

fn block_kind_label(k: BlockKind) -> &'static str {
    match k {
        BlockKind::Loop => "loop",
        BlockKind::Alt => "alt",
        BlockKind::Opt => "opt",
        BlockKind::Par => "par",
        BlockKind::Critical => "critical",
        BlockKind::Break => "break",
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mermaid::ascii::CharsetKind;

    fn render(src: &str) -> String {
        crate::mermaid::ascii::sequence::render(src, 200, CharsetKind::Unicode)
            .unwrap()
            .join("\n")
    }

    fn render_ascii(src: &str) -> String {
        crate::mermaid::ascii::sequence::render(src, 200, CharsetKind::Ascii)
            .unwrap()
            .join("\n")
    }

    // ---------- Snapshot tests ----------

    #[test]
    fn render_two_participants_solid() {
        let out = render("sequenceDiagram\nA->>B: hello\n");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_two_participants_dotted() {
        let out = render("sequenceDiagram\nA-->>B: reply\n");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_three_participants_chain() {
        let out = render("sequenceDiagram\nA->>B: ask\nB->>C: forward\nC-->>A: answer\n");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_self_message() {
        let out = render("sequenceDiagram\nA->>A: think\n");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_self_message_with_label() {
        let out = render("sequenceDiagram\nA->>A: recurse deeply\n");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_autonumber() {
        let out = render("sequenceDiagram\nautonumber\nA->>B: first\nB->>A: second\n");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_with_note_left() {
        let out = render(
            "sequenceDiagram\nparticipant A\nparticipant B\nA->>B: hi\nNote left of A: aside\n",
        );
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_with_note_right() {
        let out = render(
            "sequenceDiagram\nparticipant A\nparticipant B\nA->>B: hi\nNote right of B: comment\n",
        );
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_with_note_over_two() {
        let out = render("sequenceDiagram\nA->>B: hi\nNote over A,B: shared context\n");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_with_activate_deactivate() {
        let out =
            render("sequenceDiagram\nA->>B: request\nactivate B\nB-->>A: response\ndeactivate B\n");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_with_loop_block() {
        let out = render("sequenceDiagram\nA->>B: start\nloop every tick\nB->>A: ping\nend\n");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_with_alt_else_block() {
        let out =
            render("sequenceDiagram\nalt success\nA->>B: ok\nelse failure\nA->>B: err\nend\n");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_with_opt_block() {
        let out = render("sequenceDiagram\nopt maybe\nA->>B: do\nend\n");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_with_par_block() {
        let out = render("sequenceDiagram\npar work\nA->>B: x\nand more\nB->>C: y\nend\n");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_with_nested_blocks() {
        let out = render("sequenceDiagram\nloop outer\nA->>B: a\nopt inner\nA->>B: b\nend\nend\n");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_ascii_two_participants() {
        let out = render_ascii("sequenceDiagram\nA->>B: hi\n");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_async_arrow_x() {
        let out = render("sequenceDiagram\nA-xB: lost\n");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_participant_with_label_via_as() {
        let out =
            render("sequenceDiagram\nparticipant A as Alice\nparticipant B as Bob\nA->>B: hi\n");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn render_long_label_wraps_or_widens() {
        let out = render(
            "sequenceDiagram\nA->>B: this is a fairly long message label that takes space\n",
        );
        insta::assert_snapshot!(out);
    }

    // ---------- Unit tests ----------

    #[test]
    fn render_empty_diagram_returns_error() {
        let err =
            crate::mermaid::ascii::sequence::render("sequenceDiagram\n", 200, CharsetKind::Unicode)
                .unwrap_err();
        assert!(matches!(err, AsciiRenderError::Empty));
    }

    #[test]
    fn render_no_messages_just_participants() {
        let out = crate::mermaid::ascii::sequence::render(
            "sequenceDiagram\nparticipant A\nparticipant B\n",
            200,
            CharsetKind::Unicode,
        )
        .unwrap();
        let joined = out.join("\n");
        assert!(joined.contains('A'));
        assert!(joined.contains('B'));
    }

    #[test]
    fn render_participants_evenly_spaced() {
        let out = crate::mermaid::ascii::sequence::render(
            "sequenceDiagram\nparticipant A\nparticipant B\nparticipant C\n",
            200,
            CharsetKind::Unicode,
        )
        .unwrap();
        let header = &out[1];
        let pos_a = header.find('A').unwrap();
        let pos_b = header.find('B').unwrap();
        let pos_c = header.find('C').unwrap();
        let d1 = pos_b - pos_a;
        let d2 = pos_c - pos_b;
        assert_eq!(d1, d2);
    }

    #[test]
    fn render_lifeline_chars_present() {
        let out = render("sequenceDiagram\nA->>B: hi\n");
        assert!(out.contains('│'));
    }

    #[test]
    fn render_max_width_does_not_exceed() {
        let lines = crate::mermaid::ascii::sequence::render(
            "sequenceDiagram\nA->>B: hi\n",
            40,
            CharsetKind::Unicode,
        )
        .unwrap();
        for line in &lines {
            assert!(
                UnicodeWidthStr::width(line.as_str()) <= 40,
                "line over limit: {line:?}"
            );
        }
    }

    #[test]
    fn render_message_label_appears_in_output() {
        let out = render("sequenceDiagram\nA->>B: greetings\n");
        assert!(out.contains("greetings"), "output:\n{out}");
    }

    #[test]
    fn render_self_message_has_three_lines() {
        let out = render("sequenceDiagram\nA->>A: loop\n");
        let lines: Vec<&str> = out.lines().collect();
        let arrow_lines: Vec<&&str> = lines
            .iter()
            .filter(|l| l.contains('┐') || l.contains('┘'))
            .collect();
        assert!(
            arrow_lines.len() >= 2,
            "expected self-message frame chars, got {out}"
        );
    }

    #[test]
    fn render_arrowhead_at_target_side() {
        let out = render("sequenceDiagram\nA->>B: hi\n");
        assert!(out.contains('►'), "expected right arrowhead, got {out}");
        let out2 = render("sequenceDiagram\nA->>B: hi\nB->>A: bye\n");
        assert!(out2.contains('◄'), "expected left arrowhead, got {out2}");
    }

    #[test]
    fn render_styled_attaches_color_to_participant_borders() {
        let rows = crate::mermaid::ascii::sequence::render_styled(
            "sequenceDiagram\nA->>B: hello\n",
            200,
            CharsetKind::Unicode,
        )
        .unwrap();
        assert!(
            rows.iter().flatten().any(|r| r.color == Some(Color::Cyan)),
            "expected at least one Cyan border run, got {rows:?}"
        );
    }

    #[test]
    fn render_styled_arrowhead_is_magenta() {
        let rows = crate::mermaid::ascii::sequence::render_styled(
            "sequenceDiagram\nA->>B: hello\n",
            200,
            CharsetKind::Unicode,
        )
        .unwrap();
        assert!(
            rows.iter()
                .flatten()
                .any(|r| r.color == Some(Color::Magenta)),
            "expected a Magenta arrowhead run, got {rows:?}"
        );
    }

    #[test]
    fn render_styled_label_is_green() {
        let rows = crate::mermaid::ascii::sequence::render_styled(
            "sequenceDiagram\nA->>B: greetings\n",
            200,
            CharsetKind::Unicode,
        )
        .unwrap();
        let green_text: String = rows
            .iter()
            .flatten()
            .filter(|r| r.color == Some(Color::Green))
            .map(|r| r.text.as_str())
            .collect();
        assert!(
            green_text.contains("greetings"),
            "expected 'greetings' painted green, got {rows:?}"
        );
    }
}
