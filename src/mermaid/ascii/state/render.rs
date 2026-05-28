use super::ast::{State, StateDiagram, StateDir, StateKind};
use crate::mermaid::ascii::canvas::Canvas;
use crate::mermaid::ascii::charset::get_charset;
use crate::mermaid::ascii::{AsciiRenderError, CharsetKind, StyledRow};
use ratatui::style::Color;
use std::collections::{HashMap, HashSet, VecDeque};
use unicode_width::UnicodeWidthStr;

pub fn render(
    diag: &StateDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    let canvas = render_to_canvas(diag, max_width, charset)?;
    Ok(canvas.into_lines())
}

pub fn render_styled(
    diag: &StateDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    let canvas = render_to_canvas(diag, max_width, charset)?;
    Ok(canvas.into_styled_lines())
}

fn render_to_canvas(
    diag: &StateDiagram,
    _max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    if diag.states.is_empty() && diag.transitions.is_empty() {
        return Err(AsciiRenderError::Empty);
    }
    let region = render_region(diag, None, charset);
    Ok(region.canvas)
}

struct Rendered {
    canvas: Canvas,
    attach: HashMap<String, Attach>,
}

#[derive(Clone, Copy)]
struct Attach {
    top: (usize, usize),
    bot: (usize, usize),
    left: (usize, usize),
    right: (usize, usize),
}

const H_GAP: usize = 4;
const V_GAP: usize = 3;

fn render_region(diag: &StateDiagram, parent: Option<&str>, charset: CharsetKind) -> Rendered {
    let ids: Vec<String> = diag
        .states
        .values()
        .filter(|s| s.parent.as_deref() == parent)
        .map(|s| s.id.clone())
        .collect();

    let layers = layer_states(diag, &ids);
    match diag.direction {
        StateDir::TopDown => render_layers_td(diag, &layers, charset),
        StateDir::LeftRight => render_layers_lr(diag, &layers, charset),
    }
}

fn layer_states(diag: &StateDiagram, ids: &[String]) -> Vec<Vec<String>> {
    if ids.is_empty() {
        return Vec::new();
    }
    let id_set: HashSet<String> = ids.iter().cloned().collect();

    let mut indeg: HashMap<String, usize> = HashMap::new();
    for id in ids {
        indeg.insert(id.clone(), 0);
    }
    for t in &diag.transitions {
        if id_set.contains(&t.from) && id_set.contains(&t.to) && t.from != t.to {
            *indeg.entry(t.to.clone()).or_insert(0) += 1;
        }
    }

    let mut starts: Vec<String> = ids
        .iter()
        .filter(|id| {
            let st = &diag.states[*id];
            st.kind == StateKind::Start || indeg.get(*id).copied().unwrap_or(0) == 0
        })
        .cloned()
        .collect();
    if starts.is_empty() {
        starts = ids.iter().take(1).cloned().collect();
    }

    let mut layer_of: HashMap<String, usize> = HashMap::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    for s in &starts {
        layer_of.insert(s.clone(), 0);
        queue.push_back(s.clone());
    }
    while let Some(cur) = queue.pop_front() {
        let cur_layer = layer_of[&cur];
        for t in &diag.transitions {
            if t.from == cur && id_set.contains(&t.to) && t.from != t.to {
                let nl = cur_layer + 1;
                let existing = layer_of.get(&t.to).copied();
                if existing.is_none_or(|e| nl > e) {
                    layer_of.insert(t.to.clone(), nl);
                    queue.push_back(t.to.clone());
                }
            }
        }
    }
    for id in ids {
        layer_of.entry(id.clone()).or_insert(0);
    }

    let max_layer = layer_of.values().copied().max().unwrap_or(0);
    let mut layers: Vec<Vec<String>> = vec![Vec::new(); max_layer + 1];
    for id in ids {
        let l = layer_of[id];
        layers[l].push(id.clone());
    }
    layers
}

fn render_node_box(state: &State, charset: CharsetKind, diag: &StateDiagram) -> Canvas {
    match state.kind {
        StateKind::Start => render_start(charset),
        StateKind::End => render_end(charset),
        StateKind::Choice => render_choice(state, charset),
        StateKind::Fork | StateKind::Join => render_bar(state, charset),
        StateKind::Composite => render_composite(state, charset, diag),
        StateKind::Normal => render_rounded(state, charset),
    }
}

fn render_start(charset: CharsetKind) -> Canvas {
    let mut c = Canvas::new(3, 1, charset);
    let ch = if matches!(charset, CharsetKind::Unicode) {
        '●'
    } else {
        '*'
    };
    c.put_char(1, 0, ch);
    c.set_color(1, 0, Color::Green);
    c
}

fn render_end(charset: CharsetKind) -> Canvas {
    if matches!(charset, CharsetKind::Unicode) {
        let mut c = Canvas::new(3, 1, charset);
        c.put_char(1, 0, '◉');
        c.set_color(1, 0, Color::Red);
        c
    } else {
        let mut c = Canvas::new(3, 1, charset);
        c.put_str(0, 0, "(*)");
        c.paint_text(0, 0, "(*)", Color::Red);
        c
    }
}

fn render_rounded(state: &State, charset: CharsetKind) -> Canvas {
    let cs = get_charset(charset);
    let lines = compose_lines(state);
    let inner_w = lines
        .iter()
        .map(|l| UnicodeWidthStr::width(l.as_str()))
        .max()
        .unwrap_or(state.label.len())
        .max(1);
    let w = inner_w + 4;
    let h = lines.len() + 2;
    let mut c = Canvas::new(w, h, charset);
    c.draw_box(0, 0, w - 1, h - 1);
    c.put_char(0, 0, cs.round_top_left());
    c.put_char(w - 1, 0, cs.round_top_right());
    c.put_char(0, h - 1, cs.round_bot_left());
    c.put_char(w - 1, h - 1, cs.round_bot_right());
    c.paint_box(0, 0, w - 1, h - 1, Color::Cyan);
    for (i, line) in lines.iter().enumerate() {
        let lw = UnicodeWidthStr::width(line.as_str());
        let x = 1 + (inner_w + 2 - lw) / 2;
        c.put_str(x, 1 + i, line);
    }
    c
}

fn compose_lines(state: &State) -> Vec<String> {
    let label = if state.label.is_empty() {
        state.id.clone()
    } else {
        state.label.clone()
    };
    let mut lines = vec![label];
    if let Some(desc) = &state.description {
        if !desc.is_empty() {
            lines.push(desc.clone());
        }
    }
    if let Some(note) = &state.note {
        if !note.is_empty() {
            lines.push(format!("[{}]", note));
        }
    }
    lines
}

fn render_choice(state: &State, charset: CharsetKind) -> Canvas {
    let label = if state.label.is_empty() {
        state.id.clone()
    } else {
        state.label.clone()
    };
    let lw = UnicodeWidthStr::width(label.as_str()).max(1);
    let w = lw + 4;
    let h = 3;
    let mut c = Canvas::new(w, h, charset);
    let cs = get_charset(charset);
    let mid = w / 2;
    c.put_char(mid - 1, 0, cs.diag_up());
    c.put_char(mid, 0, cs.diag_down());
    c.put_char(0, 1, cs.diag_up());
    c.put_str(2, 1, &label);
    c.put_char(w - 1, 1, cs.diag_down());
    c.put_char(mid - 1, 2, cs.diag_down());
    c.put_char(mid, 2, cs.diag_up());
    c.set_color(mid - 1, 0, Color::Magenta);
    c.set_color(mid, 0, Color::Magenta);
    c.set_color(0, 1, Color::Magenta);
    c.set_color(w - 1, 1, Color::Magenta);
    c.set_color(mid - 1, 2, Color::Magenta);
    c.set_color(mid, 2, Color::Magenta);
    c
}

fn render_bar(state: &State, charset: CharsetKind) -> Canvas {
    let label = if state.label.is_empty() {
        state.id.clone()
    } else {
        state.label.clone()
    };
    let lw = UnicodeWidthStr::width(label.as_str()).max(3);
    let w = lw + 4;
    let mut c = Canvas::new(w, 1, charset);
    let bar = if matches!(charset, CharsetKind::Unicode) {
        '▆'
    } else {
        '='
    };
    for x in 0..w {
        c.put_char(x, 0, bar);
    }
    c.paint_hline(0, w.saturating_sub(1), 0, Color::Magenta);
    c
}

fn render_composite(state: &State, charset: CharsetKind, diag: &StateDiagram) -> Canvas {
    let title = if state.label.is_empty() {
        state.id.clone()
    } else {
        state.label.clone()
    };
    let tw = UnicodeWidthStr::width(title.as_str());
    let cs = get_charset(charset);

    let body_canvas = if state.concurrent_regions.is_empty() {
        render_region(diag, Some(&state.id), charset).canvas
    } else {
        build_concurrent_body(state, diag, charset)
    };
    let body_w = body_canvas.width().max(tw + 2);
    let w = body_w + 4;
    let h = body_canvas.height() + 3;
    let mut c = Canvas::new(w, h, charset);
    c.draw_box(0, 0, w - 1, h - 1);
    c.put_char(0, 0, cs.round_top_left());
    c.put_char(w - 1, 0, cs.round_top_right());
    c.put_char(0, h - 1, cs.round_bot_left());
    c.put_char(w - 1, h - 1, cs.round_bot_right());
    c.paint_box(0, 0, w - 1, h - 1, Color::Blue);
    c.put_str(2, 0, &title);
    let sep_y = 1;
    for x in 1..w - 1 {
        c.put_char(x, sep_y, cs.horiz());
    }
    c.put_char(0, sep_y, cs.tee_right());
    c.put_char(w - 1, sep_y, cs.tee_left());
    c.paint_hline(0, w - 1, sep_y, Color::Blue);
    c.merge(&body_canvas, 2, 2);
    c
}

fn build_concurrent_body(state: &State, diag: &StateDiagram, charset: CharsetKind) -> Canvas {
    let regions: Vec<Vec<String>> = {
        let mut all = state.concurrent_regions.clone();
        let in_regions: HashSet<String> = all.iter().flatten().cloned().collect();
        let leftover: Vec<String> = state
            .children
            .iter()
            .filter(|c| !in_regions.contains(*c))
            .cloned()
            .collect();
        if !leftover.is_empty() {
            all.push(leftover);
        }
        all
    };

    let region_canvases: Vec<Canvas> = regions
        .iter()
        .map(|r| render_region_subset(diag, r, charset))
        .collect();
    let max_w = region_canvases
        .iter()
        .map(|c| c.width())
        .max()
        .unwrap_or(1)
        .max(1);
    let total_h: usize = region_canvases.iter().map(|c| c.height()).sum::<usize>()
        + region_canvases.len().saturating_sub(1);
    let mut out = Canvas::new(max_w, total_h.max(1), charset);
    let div = if matches!(charset, CharsetKind::Unicode) {
        '┄'
    } else {
        '-'
    };
    let mut y = 0;
    for (i, rc) in region_canvases.iter().enumerate() {
        if i > 0 {
            for x in 0..max_w {
                out.put_char(x, y, div);
            }
            y += 1;
        }
        out.merge(rc, 0, y);
        y += rc.height();
    }
    out
}

fn render_region_subset(diag: &StateDiagram, ids: &[String], charset: CharsetKind) -> Canvas {
    let layers = layer_states(diag, ids);
    match diag.direction {
        StateDir::TopDown => render_layers_td(diag, &layers, charset).canvas,
        StateDir::LeftRight => render_layers_lr(diag, &layers, charset).canvas,
    }
}

fn render_layers_td(diag: &StateDiagram, layers: &[Vec<String>], charset: CharsetKind) -> Rendered {
    let mut layer_canvases: Vec<Vec<(String, Canvas)>> = Vec::new();
    for layer in layers {
        let mut row = Vec::new();
        for id in layer {
            let st = &diag.states[id];
            let canvas = render_node_box(st, charset, diag);
            row.push((id.clone(), canvas));
        }
        layer_canvases.push(row);
    }

    let mut layer_widths: Vec<usize> = Vec::new();
    let mut layer_heights: Vec<usize> = Vec::new();
    for row in &layer_canvases {
        let total_w: usize =
            row.iter().map(|(_, c)| c.width()).sum::<usize>() + H_GAP * row.len().saturating_sub(1);
        let max_h: usize = row.iter().map(|(_, c)| c.height()).max().unwrap_or(0);
        layer_widths.push(total_w);
        layer_heights.push(max_h);
    }
    let max_label_w: usize = diag
        .transitions
        .iter()
        .map(|t| UnicodeWidthStr::width(t.label.as_str()))
        .max()
        .unwrap_or(0);
    let min_w = max_label_w + 4;
    let total_w = layer_widths
        .iter()
        .copied()
        .max()
        .unwrap_or(1)
        .max(min_w)
        .max(1);
    let total_h: usize =
        layer_heights.iter().sum::<usize>() + V_GAP * layers.len().saturating_sub(1).max(1);
    let total_h = total_h.max(1);

    let mut canvas = Canvas::new(total_w, total_h, charset);
    let mut attach: HashMap<String, Attach> = HashMap::new();

    let mut y_cursor: usize = 0;
    for (i, row) in layer_canvases.iter().enumerate() {
        let row_w = layer_widths[i];
        let row_h = layer_heights[i];
        let mut x_cursor = (total_w.saturating_sub(row_w)) / 2;
        for (id, cv) in row {
            canvas.merge(cv, x_cursor, y_cursor);
            let w = cv.width();
            let h = cv.height();
            let top = (x_cursor + w / 2, y_cursor);
            let bot = (x_cursor + w / 2, y_cursor + h.saturating_sub(1));
            let left = (x_cursor, y_cursor + h / 2);
            let right = (x_cursor + w.saturating_sub(1), y_cursor + h / 2);
            attach.insert(
                id.clone(),
                Attach {
                    top,
                    bot,
                    left,
                    right,
                },
            );
            x_cursor += w + H_GAP;
        }
        y_cursor += row_h + V_GAP;
    }

    draw_transitions_td(&mut canvas, diag, &attach, charset);
    Rendered { canvas, attach }
}

fn render_layers_lr(diag: &StateDiagram, layers: &[Vec<String>], charset: CharsetKind) -> Rendered {
    let mut layer_canvases: Vec<Vec<(String, Canvas)>> = Vec::new();
    for layer in layers {
        let mut col = Vec::new();
        for id in layer {
            let st = &diag.states[id];
            let canvas = render_node_box(st, charset, diag);
            col.push((id.clone(), canvas));
        }
        layer_canvases.push(col);
    }

    let mut layer_widths: Vec<usize> = Vec::new();
    let mut layer_heights: Vec<usize> = Vec::new();
    for col in &layer_canvases {
        let total_h: usize = col.iter().map(|(_, c)| c.height()).sum::<usize>()
            + V_GAP * col.len().saturating_sub(1);
        let max_w: usize = col.iter().map(|(_, c)| c.width()).max().unwrap_or(0);
        layer_widths.push(max_w);
        layer_heights.push(total_h);
    }
    let total_h = layer_heights.iter().copied().max().unwrap_or(1).max(1);
    let total_w: usize =
        layer_widths.iter().sum::<usize>() + H_GAP * layers.len().saturating_sub(1).max(1);
    let total_w = total_w.max(1);

    let mut canvas = Canvas::new(total_w, total_h, charset);
    let mut attach: HashMap<String, Attach> = HashMap::new();

    let mut x_cursor: usize = 0;
    for (i, col) in layer_canvases.iter().enumerate() {
        let col_w = layer_widths[i];
        let col_h = layer_heights[i];
        let mut y_cursor = (total_h.saturating_sub(col_h)) / 2;
        for (id, cv) in col {
            canvas.merge(cv, x_cursor, y_cursor);
            let w = cv.width();
            let h = cv.height();
            let top = (x_cursor + w / 2, y_cursor);
            let bot = (x_cursor + w / 2, y_cursor + h.saturating_sub(1));
            let left = (x_cursor, y_cursor + h / 2);
            let right = (x_cursor + w.saturating_sub(1), y_cursor + h / 2);
            attach.insert(
                id.clone(),
                Attach {
                    top,
                    bot,
                    left,
                    right,
                },
            );
            y_cursor += h + V_GAP;
        }
        x_cursor += col_w + H_GAP;
    }

    draw_transitions_lr(&mut canvas, diag, &attach, charset);
    Rendered { canvas, attach }
}

fn draw_transitions_td(
    canvas: &mut Canvas,
    diag: &StateDiagram,
    attach: &HashMap<String, Attach>,
    charset: CharsetKind,
) {
    let cs = get_charset(charset);
    for t in &diag.transitions {
        let (Some(fa), Some(ta)) = (attach.get(&t.from), attach.get(&t.to)) else {
            continue;
        };
        let (sx, sy) = fa.bot;
        let (ex, ey) = ta.top;
        if ey <= sy {
            continue;
        }
        let mid_y = (sy + ey) / 2;
        for y in (sy + 1)..mid_y {
            place_vert(canvas, sx, y, cs.vert());
        }
        canvas.paint_vline(sx, sy + 1, mid_y.saturating_sub(1), Color::Yellow);
        if sx != ex {
            place_horiz_range(canvas, sx, ex, mid_y, cs.horiz());
            if sx < ex {
                canvas.put_char(sx, mid_y, cs.round_bot_left());
                canvas.put_char(ex, mid_y, cs.round_top_right());
            } else {
                canvas.put_char(sx, mid_y, cs.round_bot_right());
                canvas.put_char(ex, mid_y, cs.round_top_left());
            }
            canvas.paint_hline(sx, ex, mid_y, Color::Yellow);
        }
        for y in mid_y + 1..ey {
            place_vert(canvas, ex, y, cs.vert());
        }
        if mid_y + 1 < ey {
            canvas.paint_vline(ex, mid_y + 1, ey.saturating_sub(1), Color::Yellow);
        }
        if ey > 0 {
            canvas.put_char(ex, ey.saturating_sub(1), cs.arrow_down());
            canvas.set_color(ex, ey.saturating_sub(1), Color::Yellow);
        }
        if !t.label.is_empty() {
            let lw = UnicodeWidthStr::width(t.label.as_str());
            let canvas_w = canvas.width();
            let raw_lx = if sx == ex {
                (sx + 2).min(canvas_w.saturating_sub(lw))
            } else {
                ((sx + ex) / 2).saturating_sub(lw / 2)
            };
            let lx = raw_lx.min(canvas_w.saturating_sub(lw));
            canvas.put_str(lx, mid_y, &t.label);
            canvas.paint_text(lx, mid_y, &t.label, Color::Green);
        }
    }
}

fn draw_transitions_lr(
    canvas: &mut Canvas,
    diag: &StateDiagram,
    attach: &HashMap<String, Attach>,
    charset: CharsetKind,
) {
    let cs = get_charset(charset);
    for t in &diag.transitions {
        let (Some(fa), Some(ta)) = (attach.get(&t.from), attach.get(&t.to)) else {
            continue;
        };
        let (sx, sy) = fa.right;
        let (ex, ey) = ta.left;
        if ex <= sx {
            continue;
        }
        let mid_x = (sx + ex) / 2;
        if sy == ey {
            for x in (sx + 1)..ex {
                place_horiz(canvas, x, sy, cs.horiz());
            }
            if sx + 1 < ex {
                canvas.paint_hline(sx + 1, ex.saturating_sub(1), sy, Color::Yellow);
            }
        } else {
            for x in (sx + 1)..=mid_x {
                place_horiz(canvas, x, sy, cs.horiz());
            }
            if sx < mid_x {
                canvas.paint_hline(sx + 1, mid_x, sy, Color::Yellow);
            }
            place_vert_range(canvas, mid_x, sy, ey, cs.vert());
            canvas.paint_vline(mid_x, sy, ey, Color::Yellow);
            if sy < ey {
                canvas.put_char(mid_x, sy, cs.round_top_right());
                canvas.put_char(mid_x, ey, cs.round_bot_left());
            } else {
                canvas.put_char(mid_x, sy, cs.round_bot_right());
                canvas.put_char(mid_x, ey, cs.round_top_left());
            }
            for x in mid_x + 1..ex {
                place_horiz(canvas, x, ey, cs.horiz());
            }
            if mid_x + 1 < ex {
                canvas.paint_hline(mid_x + 1, ex.saturating_sub(1), ey, Color::Yellow);
            }
        }
        if ex > 0 {
            canvas.put_char(ex.saturating_sub(1), ey, cs.arrow_right());
            canvas.set_color(ex.saturating_sub(1), ey, Color::Yellow);
        }
        if !t.label.is_empty() {
            let lw = UnicodeWidthStr::width(t.label.as_str());
            let lx = mid_x.saturating_sub(lw / 2);
            let ly = if sy == ey {
                sy.saturating_sub(1)
            } else {
                (sy + ey) / 2
            };
            canvas.put_str(lx, ly, &t.label);
            canvas.paint_text(lx, ly, &t.label, Color::Green);
        }
    }
}

fn place_horiz(canvas: &mut Canvas, x: usize, y: usize, c: char) {
    let cur = canvas.get(x, y);
    if cur == ' ' {
        canvas.put_char(x, y, c);
    }
}

fn place_vert(canvas: &mut Canvas, x: usize, y: usize, c: char) {
    let cur = canvas.get(x, y);
    if cur == ' ' {
        canvas.put_char(x, y, c);
    }
}

fn place_horiz_range(canvas: &mut Canvas, x1: usize, x2: usize, y: usize, c: char) {
    let (lo, hi) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
    for x in lo..=hi {
        place_horiz(canvas, x, y, c);
    }
}

fn place_vert_range(canvas: &mut Canvas, x: usize, y1: usize, y2: usize, c: char) {
    let (lo, hi) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
    for y in lo..=hi {
        place_vert(canvas, x, y, c);
    }
}

#[allow(dead_code)]
impl Rendered {
    fn attach(&self) -> &HashMap<String, Attach> {
        &self.attach
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(src: &str) -> String {
        crate::mermaid::ascii::state::render(src, 200, CharsetKind::Unicode)
            .unwrap()
            .join("\n")
    }

    fn render_ascii(src: &str) -> String {
        crate::mermaid::ascii::state::render(src, 200, CharsetKind::Ascii)
            .unwrap()
            .join("\n")
    }

    // --- Snapshot tests -------------------------------------------------

    #[test]
    fn render_minimal_state_diagram() {
        insta::assert_snapshot!(render("stateDiagram-v2\nA"));
    }

    #[test]
    fn render_start_to_state() {
        insta::assert_snapshot!(render("stateDiagram-v2\n[*] --> A"));
    }

    #[test]
    fn render_state_to_end() {
        insta::assert_snapshot!(render("stateDiagram-v2\nA --> [*]"));
    }

    #[test]
    fn render_state_to_state_with_label() {
        insta::assert_snapshot!(render("stateDiagram-v2\nA --> B : go"));
    }

    #[test]
    fn render_three_states_chain() {
        insta::assert_snapshot!(render("stateDiagram-v2\nA --> B\nB --> C"));
    }

    #[test]
    fn render_state_with_description() {
        insta::assert_snapshot!(render(
            "stateDiagram-v2\nIdle\nIdle : waiting for input\nIdle --> Running"
        ));
    }

    #[test]
    fn render_composite_with_substates() {
        insta::assert_snapshot!(render(
            "stateDiagram-v2\nstate Outer {\n  [*] --> Inner\n  Inner --> [*]\n}"
        ));
    }

    #[test]
    fn render_choice_diamond() {
        insta::assert_snapshot!(render(
            "stateDiagram-v2\nstate C <<choice>>\nA --> C\nC --> B\nC --> D"
        ));
    }

    #[test]
    fn render_fork_join() {
        insta::assert_snapshot!(render(
            "stateDiagram-v2\nstate F <<fork>>\nstate J <<join>>\nA --> F\nF --> B\nF --> C\nB --> J\nC --> J"
        ));
    }

    #[test]
    fn render_direction_lr() {
        insta::assert_snapshot!(render("stateDiagram-v2\ndirection LR\nA --> B"));
    }

    #[test]
    fn render_direction_td() {
        insta::assert_snapshot!(render("stateDiagram-v2\ndirection TD\nA --> B"));
    }

    #[test]
    fn render_concurrent_regions() {
        insta::assert_snapshot!(render("stateDiagram-v2\nstate Active {\n  A\n  --\n  B\n}"));
    }

    #[test]
    fn render_ascii_charset() {
        insta::assert_snapshot!(render_ascii("stateDiagram-v2\nA --> B"));
    }

    #[test]
    fn render_state_with_note() {
        insta::assert_snapshot!(render("stateDiagram-v2\nA\nnote right of A : hi"));
    }

    // --- Unit tests -----------------------------------------------------

    #[test]
    fn render_start_marker_is_filled_circle() {
        let out = render("stateDiagram-v2\n[*] --> A");
        assert!(out.contains('●'), "expected ● in:\n{}", out);
    }

    #[test]
    fn render_end_marker_is_target() {
        let out = render("stateDiagram-v2\nA --> [*]");
        assert!(out.contains('◉'), "expected ◉ in:\n{}", out);
    }

    #[test]
    fn render_states_are_rounded_boxes() {
        let out = render("stateDiagram-v2\nA");
        assert!(
            out.contains('╭') || out.contains('╮'),
            "expected rounded corner in:\n{}",
            out
        );
    }

    #[test]
    fn render_choice_is_diamond() {
        let out = render("stateDiagram-v2\nstate C <<choice>>\nA --> C");
        assert!(
            out.contains('╱') || out.contains('╲'),
            "expected diamond in:\n{}",
            out
        );
    }

    #[test]
    fn render_transition_label_visible() {
        let out = render("stateDiagram-v2\nA --> B : my_label");
        assert!(out.contains("my_label"), "expected label in:\n{}", out);
    }

    #[test]
    fn render_composite_has_outer_frame() {
        let out = render("stateDiagram-v2\nstate Outer {\n  A\n}");
        let has_rounded = out.contains('╭') && out.contains('╯');
        assert!(has_rounded, "expected composite frame in:\n{}", out);
    }

    #[test]
    fn render_styled_attaches_color_to_borders() {
        let rows = crate::mermaid::ascii::state::render_styled(
            "stateDiagram-v2\nA --> B",
            100,
            CharsetKind::Unicode,
        )
        .unwrap();
        assert!(rows
            .iter()
            .flatten()
            .any(|r| r.color == Some(ratatui::style::Color::Cyan)));
    }

    #[test]
    fn render_styled_has_multiple_colors() {
        let rows = crate::mermaid::ascii::state::render_styled(
            "stateDiagram-v2\n[*] --> A\nA --> B : go\nB --> [*]",
            100,
            CharsetKind::Unicode,
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
        let rows = crate::mermaid::ascii::state::render_styled(
            "stateDiagram-v2\n[*] --> A",
            100,
            CharsetKind::Unicode,
        )
        .unwrap();
        let start_color = rows
            .iter()
            .flatten()
            .find(|r| r.text.contains('●'))
            .map(|r| r.color);
        assert_eq!(
            start_color,
            Some(Some(ratatui::style::Color::Green)),
            "start marker ● should be green"
        );
    }
}
