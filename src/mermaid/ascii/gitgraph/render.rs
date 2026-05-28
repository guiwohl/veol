use super::ast::{CommitKind, GitGraph, GitOp, Orientation};
use crate::mermaid::ascii::canvas::Canvas;
use crate::mermaid::ascii::{AsciiRenderError, CharsetKind, StyledRow};
use ratatui::style::Color;
use unicode_width::UnicodeWidthStr;

const COL_STEP: usize = 3;
const ROW_STEP: usize = 3;

pub fn render(
    diag: &GitGraph,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| trim_blank_lines(c.into_lines()))
}

pub fn render_styled(
    diag: &GitGraph,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset)
        .map(|c| trim_blank_styled_lines(c.into_styled_lines()))
}

fn render_to_canvas(
    diag: &GitGraph,
    _max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    if diag.ops.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    match diag.orientation {
        Orientation::LeftRight => render_lr(diag, charset),
        Orientation::TopDown => render_td(diag, charset),
    }
}

#[derive(Clone)]
struct Lane {
    name: String,
    last_step: Option<usize>,
}

struct Plan {
    lanes: Vec<Lane>,
    events: Vec<Event>,
    total_steps: usize,
}

#[derive(Clone)]
enum Event {
    Commit {
        lane: usize,
        step: usize,
        kind: CommitKind,
        tag: Option<String>,
    },
    Merge {
        from_lane: usize,
        from_step: usize,
        to_lane: usize,
        to_step: usize,
        tag: Option<String>,
    },
    CherryPick {
        lane: usize,
        step: usize,
        id: String,
    },
}

fn build_plan(diag: &GitGraph) -> Plan {
    let mut lanes: Vec<Lane> = vec![Lane {
        name: "main".to_string(),
        last_step: None,
    }];
    let mut active: usize = 0;
    let mut step: usize = 0;
    let mut events: Vec<Event> = Vec::new();

    let find_lane =
        |lanes: &[Lane], name: &str| -> Option<usize> { lanes.iter().position(|l| l.name == name) };

    for op in &diag.ops {
        match op {
            GitOp::Commit { kind, tag, .. } => {
                step += 1;
                events.push(Event::Commit {
                    lane: active,
                    step,
                    kind: *kind,
                    tag: tag.clone(),
                });
                lanes[active].last_step = Some(step);
            }
            GitOp::Branch { name, from } => {
                if find_lane(&lanes, name).is_none() {
                    let parent_idx = match from {
                        Some(f) => find_lane(&lanes, f).unwrap_or(active),
                        None => active,
                    };
                    let last = lanes[parent_idx].last_step;
                    lanes.push(Lane {
                        name: name.clone(),
                        last_step: last,
                    });
                }
                if let Some(i) = find_lane(&lanes, name) {
                    active = i;
                }
            }
            GitOp::Checkout { branch } => {
                if let Some(i) = find_lane(&lanes, branch) {
                    active = i;
                }
            }
            GitOp::Merge { branch, tag, .. } => {
                let from_lane = find_lane(&lanes, branch).unwrap_or(active);
                let from_step = lanes[from_lane].last_step.unwrap_or(0);
                step += 1;
                events.push(Event::Merge {
                    from_lane,
                    from_step,
                    to_lane: active,
                    to_step: step,
                    tag: tag.clone(),
                });
                lanes[active].last_step = Some(step);
            }
            GitOp::CherryPick { id } => {
                step += 1;
                events.push(Event::CherryPick {
                    lane: active,
                    step,
                    id: id.clone(),
                });
                lanes[active].last_step = Some(step);
            }
        }
    }

    Plan {
        lanes,
        events,
        total_steps: step,
    }
}

fn commit_glyph(kind: CommitKind, charset: CharsetKind) -> char {
    match (kind, charset) {
        (CommitKind::Highlight, CharsetKind::Unicode) => '◉',
        (CommitKind::Highlight, CharsetKind::Ascii) => 'O',
        (CommitKind::Reverse, CharsetKind::Unicode) => '◐',
        (CommitKind::Reverse, CharsetKind::Ascii) => 'o',
        (CommitKind::Normal, CharsetKind::Unicode) => '●',
        (CommitKind::Normal, CharsetKind::Ascii) => '*',
    }
}

fn commit_color(kind: CommitKind) -> Color {
    match kind {
        CommitKind::Normal => Color::Green,
        CommitKind::Highlight => Color::Yellow,
        CommitKind::Reverse => Color::Red,
    }
}

fn put_label_colored(canvas: &mut Canvas, y: usize, x: usize, label: &str, color: Color) {
    let mut cx = x;
    for c in label.chars() {
        if cx >= canvas.width() {
            break;
        }
        canvas.put_char(cx, y, c);
        canvas.set_color(cx, y, color);
        cx += unicode_width::UnicodeWidthChar::width(c)
            .unwrap_or(1)
            .max(1);
    }
}

fn render_lr(diag: &GitGraph, charset: CharsetKind) -> Result<Canvas, AsciiRenderError> {
    let plan = build_plan(diag);
    if plan.total_steps == 0 {
        return Err(AsciiRenderError::Empty);
    }

    let label_w = plan
        .lanes
        .iter()
        .map(|l| UnicodeWidthStr::width(l.name.as_str()) + 2)
        .max()
        .unwrap_or(0);

    let horiz = match charset {
        CharsetKind::Unicode => '─',
        CharsetKind::Ascii => '-',
    };
    let diag_up = match charset {
        CharsetKind::Unicode => '╱',
        CharsetKind::Ascii => '/',
    };
    let diag_down = match charset {
        CharsetKind::Unicode => '╲',
        CharsetKind::Ascii => '\\',
    };

    let width = label_w + plan.total_steps * COL_STEP + 4;
    let lane_count = plan.lanes.len();
    let height = lane_count * ROW_STEP;
    let mut canvas = Canvas::new(width, height, charset);

    let step_x = |s: usize| label_w + s * COL_STEP;
    let lane_y = |l: usize| l * ROW_STEP + 1;

    for (li, lane) in plan.lanes.iter().enumerate() {
        let y = lane_y(li);
        let label = format!("{}: ", lane.name);
        put_label_colored(&mut canvas, y, 0, &label, Color::Cyan);
    }

    let mut lane_first: Vec<Option<usize>> = vec![None; lane_count];
    let mut lane_last: Vec<Option<usize>> = vec![None; lane_count];

    for ev in &plan.events {
        let (lane, step) = match ev {
            Event::Commit { lane, step, .. } => (*lane, *step),
            Event::Merge {
                to_lane, to_step, ..
            } => (*to_lane, *to_step),
            Event::CherryPick { lane, step, .. } => (*lane, *step),
        };
        if lane_first[lane].is_none() {
            lane_first[lane] = Some(step);
        }
        lane_last[lane] = Some(step);
    }

    for li in 0..lane_count {
        if let (Some(a), Some(b)) = (lane_first[li], lane_last[li]) {
            let y = lane_y(li);
            let x1 = step_x(a);
            let x2 = step_x(b);
            for x in x1..=x2 {
                canvas.put_char(x, y, horiz);
            }
            canvas.paint_hline(x1, x2, y, Color::DarkGray);
        }
    }

    for ev in &plan.events {
        match ev {
            Event::Commit {
                lane,
                step,
                kind,
                tag,
            } => {
                let y = lane_y(*lane);
                let x = step_x(*step);
                if x < width {
                    canvas.put_char(x, y, commit_glyph(*kind, charset));
                    canvas.set_color(x, y, commit_color(*kind));
                }
                if let Some(t) = tag {
                    if y > 0 {
                        let label = format!("({t})");
                        let tx = x.saturating_sub(1);
                        put_label_colored(&mut canvas, y - 1, tx, &label, Color::Magenta);
                    }
                }
            }
            Event::Merge {
                from_lane,
                from_step,
                to_lane,
                to_step,
                tag,
            } => {
                let y_to = lane_y(*to_lane);
                let x_to = step_x(*to_step);
                if x_to < width {
                    canvas.put_char(x_to, y_to, commit_glyph(CommitKind::Normal, charset));
                    canvas.set_color(x_to, y_to, commit_color(CommitKind::Normal));
                }
                let y_from = lane_y(*from_lane);
                let x_from = step_x(*from_step);
                draw_diag_lr(&mut canvas, x_from, y_from, x_to, y_to, diag_up, diag_down);
                if let Some(t) = tag {
                    if y_to > 0 {
                        let label = format!("({t})");
                        let tx = x_to.saturating_sub(1);
                        put_label_colored(&mut canvas, y_to - 1, tx, &label, Color::Magenta);
                    }
                }
            }
            Event::CherryPick { lane, step, id } => {
                let y = lane_y(*lane);
                let x = step_x(*step);
                if x < width {
                    canvas.put_char(x, y, commit_glyph(CommitKind::Normal, charset));
                    canvas.set_color(x, y, commit_color(CommitKind::Normal));
                }
                if y > 0 {
                    let label = format!("[{id}]");
                    let tx = x.saturating_sub(1);
                    put_label_colored(&mut canvas, y - 1, tx, &label, Color::Blue);
                }
            }
        }
    }

    Ok(canvas)
}

fn trim_blank_lines(mut lines: Vec<String>) -> Vec<String> {
    while lines.first().is_some_and(|l| l.is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
}

fn trim_blank_styled_lines(mut rows: Vec<StyledRow>) -> Vec<StyledRow> {
    while rows.first().is_some_and(row_is_blank) {
        rows.remove(0);
    }
    while rows.last().is_some_and(row_is_blank) {
        rows.pop();
    }
    rows
}

fn row_is_blank(row: &StyledRow) -> bool {
    row.iter()
        .all(|run| run.color.is_none() && run.text.trim().is_empty())
}

fn draw_diag_lr(
    canvas: &mut Canvas,
    x_from: usize,
    y_from: usize,
    x_to: usize,
    y_to: usize,
    diag_up: char,
    diag_down: char,
) {
    if x_to <= x_from {
        return;
    }
    let dx = x_to as isize - x_from as isize;
    let dy = y_to as isize - y_from as isize;
    let glyph = if dy < 0 { diag_up } else { diag_down };
    let steps = dx.unsigned_abs();
    for k in 1..steps {
        let x = x_from + k;
        let y_off = (dy * k as isize) / dx;
        let y = (y_from as isize + y_off).max(0) as usize;
        if y < canvas.height() && x < canvas.width() && canvas.get(x, y) == ' ' {
            canvas.put_char(x, y, glyph);
            canvas.set_color(x, y, Color::Yellow);
        }
    }
}

fn render_td(diag: &GitGraph, charset: CharsetKind) -> Result<Canvas, AsciiRenderError> {
    let plan = build_plan(diag);
    if plan.total_steps == 0 {
        return Err(AsciiRenderError::Empty);
    }

    let vert = match charset {
        CharsetKind::Unicode => '│',
        CharsetKind::Ascii => '|',
    };
    let diag_up = match charset {
        CharsetKind::Unicode => '╱',
        CharsetKind::Ascii => '/',
    };
    let diag_down = match charset {
        CharsetKind::Unicode => '╲',
        CharsetKind::Ascii => '\\',
    };

    let lane_count = plan.lanes.len();
    let label_row_h = 1;
    let lane_label_w = plan
        .lanes
        .iter()
        .map(|l| UnicodeWidthStr::width(l.name.as_str()))
        .max()
        .unwrap_or(0);
    let lane_w = lane_label_w.max(6) + 2;
    let width = lane_count * lane_w + 4;
    let height = label_row_h + plan.total_steps * ROW_STEP + 2;
    let mut canvas = Canvas::new(width, height, charset);

    let lane_x = |l: usize| l * lane_w + lane_w / 2;
    let step_y = |s: usize| label_row_h + (s - 1) * ROW_STEP + 1;

    for (li, lane) in plan.lanes.iter().enumerate() {
        let x = lane_x(li);
        let name = &lane.name;
        let nw = UnicodeWidthStr::width(name.as_str());
        let start_x = x.saturating_sub(nw / 2);
        put_label_colored(&mut canvas, 0, start_x, name, Color::Cyan);
    }

    let mut lane_first: Vec<Option<usize>> = vec![None; lane_count];
    let mut lane_last: Vec<Option<usize>> = vec![None; lane_count];

    for ev in &plan.events {
        let (lane, step) = match ev {
            Event::Commit { lane, step, .. } => (*lane, *step),
            Event::Merge {
                to_lane, to_step, ..
            } => (*to_lane, *to_step),
            Event::CherryPick { lane, step, .. } => (*lane, *step),
        };
        if lane_first[lane].is_none() {
            lane_first[lane] = Some(step);
        }
        lane_last[lane] = Some(step);
    }

    for li in 0..lane_count {
        if let (Some(a), Some(b)) = (lane_first[li], lane_last[li]) {
            let x = lane_x(li);
            let y1 = step_y(a);
            let y2 = step_y(b);
            for y in y1..=y2 {
                if x < canvas.width() {
                    canvas.put_char(x, y, vert);
                }
            }
            canvas.paint_vline(x, y1, y2, Color::DarkGray);
        }
    }

    for ev in &plan.events {
        match ev {
            Event::Commit {
                lane,
                step,
                kind,
                tag,
            } => {
                let y = step_y(*step);
                let x = lane_x(*lane);
                if y < height && x < width {
                    canvas.put_char(x, y, commit_glyph(*kind, charset));
                    canvas.set_color(x, y, commit_color(*kind));
                }
                if let Some(t) = tag {
                    let label = format!("({t})");
                    put_label_colored(&mut canvas, y, x + 2, &label, Color::Magenta);
                }
            }
            Event::Merge {
                from_lane,
                from_step,
                to_lane,
                to_step,
                tag,
            } => {
                let y_to = step_y(*to_step);
                let x_to = lane_x(*to_lane);
                if y_to < height && x_to < width {
                    canvas.put_char(x_to, y_to, commit_glyph(CommitKind::Normal, charset));
                    canvas.set_color(x_to, y_to, commit_color(CommitKind::Normal));
                }
                let y_from = step_y(*from_step);
                let x_from = lane_x(*from_lane);
                draw_diag_td(&mut canvas, x_from, y_from, x_to, y_to, diag_up, diag_down);
                if let Some(t) = tag {
                    let label = format!("({t})");
                    put_label_colored(&mut canvas, y_to, x_to + 2, &label, Color::Magenta);
                }
            }
            Event::CherryPick { lane, step, id } => {
                let y = step_y(*step);
                let x = lane_x(*lane);
                if y < height && x < width {
                    canvas.put_char(x, y, commit_glyph(CommitKind::Normal, charset));
                    canvas.set_color(x, y, commit_color(CommitKind::Normal));
                }
                let label = format!("[{id}]");
                put_label_colored(&mut canvas, y, x + 2, &label, Color::Blue);
            }
        }
    }

    Ok(canvas)
}

fn draw_diag_td(
    canvas: &mut Canvas,
    x_from: usize,
    y_from: usize,
    x_to: usize,
    y_to: usize,
    diag_up: char,
    diag_down: char,
) {
    if y_to <= y_from {
        return;
    }
    let dx = x_to as isize - x_from as isize;
    let dy = y_to as isize - y_from as isize;
    let glyph = if dx < 0 { diag_up } else { diag_down };
    let steps = dy.unsigned_abs();
    for k in 1..steps {
        let y = y_from + k;
        let x_off = (dx * k as isize) / dy;
        let x = (x_from as isize + x_off).max(0) as usize;
        if y < canvas.height() && x < canvas.width() && canvas.get(x, y) == ' ' {
            canvas.put_char(x, y, glyph);
            canvas.set_color(x, y, Color::Yellow);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mermaid::ascii::StyledRun;
    use ratatui::style::Color;

    fn render(s: &str) -> String {
        crate::mermaid::ascii::gitgraph::render(s, 100, crate::mermaid::ascii::CharsetKind::Unicode)
            .unwrap()
            .join("\n")
    }

    fn render_ascii(s: &str) -> String {
        crate::mermaid::ascii::gitgraph::render(s, 100, crate::mermaid::ascii::CharsetKind::Ascii)
            .unwrap()
            .join("\n")
    }

    fn render_styled(s: &str) -> Vec<Vec<StyledRun>> {
        crate::mermaid::ascii::gitgraph::render_styled(
            s,
            100,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap()
    }

    #[test]
    fn render_linear_commits_main_only() {
        let src = "gitGraph\n    commit\n    commit\n    commit";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_with_id_labels() {
        let src = "gitGraph\n    commit id: \"a\"\n    commit id: \"b\"";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_with_tag() {
        let src = "gitGraph\n    commit\n    commit tag: \"v1\"";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_branch_and_commits() {
        let src = "gitGraph\n    commit\n    branch feature\n    commit\n    commit";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_branch_checkout_merge() {
        let src = "gitGraph\n    commit\n    branch feature\n    checkout feature\n    commit\n    commit\n    checkout main\n    merge feature";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_highlight_commit() {
        let src = "gitGraph\n    commit\n    commit type: HIGHLIGHT";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_reverse_commit() {
        let src = "gitGraph\n    commit\n    commit type: REVERSE";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_cherrypick() {
        let src = "gitGraph\n    commit\n    cherry-pick id: \"abc\"";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_two_branches() {
        let src = "gitGraph\n    commit\n    branch feature\n    checkout feature\n    commit\n    checkout main\n    branch hotfix\n    checkout hotfix\n    commit";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_orientation_td() {
        let src = "gitGraph TD:\n    commit\n    branch feature\n    checkout feature\n    commit\n    checkout main\n    merge feature";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_ascii_charset() {
        let src = "gitGraph\n    commit\n    branch feature\n    checkout feature\n    commit\n    checkout main\n    merge feature";
        insta::assert_snapshot!(render_ascii(src));
    }

    #[test]
    fn render_commit_dot_appears() {
        let src = "gitGraph\n    commit";
        let out = render(src);
        assert!(
            out.contains('●'),
            "expected commit dot in output, got\n{out}"
        );
    }

    #[test]
    fn render_branch_label_visible() {
        let src = "gitGraph\n    commit\n    branch feature\n    checkout feature\n    commit";
        let out = render(src);
        assert!(out.contains("feature"), "expected branch label, got\n{out}");
        assert!(out.contains("main"), "expected main label, got\n{out}");
    }

    #[test]
    fn render_merge_arrow_present() {
        let src = "gitGraph\n    commit\n    branch feature\n    checkout feature\n    commit\n    checkout main\n    merge feature";
        let out = render(src);
        let has_diag = out.contains('╱') || out.contains('╲');
        assert!(has_diag, "expected diagonal merge line, got\n{out}");
    }

    #[test]
    fn render_highlight_commit_distinct_glyph() {
        let src = "gitGraph\n    commit type: HIGHLIGHT";
        let out = render(src);
        assert!(out.contains('◉'), "expected highlight glyph ◉, got\n{out}");
    }

    #[test]
    fn render_styled_attaches_colors() {
        let src = "gitGraph\n    commit\n    branch feature\n    checkout feature\n    commit\n    checkout main\n    merge feature";
        let rows = render_styled(src);
        let any_colored = rows
            .iter()
            .flat_map(|r| r.iter())
            .any(|run| run.color.is_some());
        assert!(any_colored, "expected colored runs");
    }

    #[test]
    fn render_styled_specific_element_colored() {
        let src = "gitGraph\n    commit\n    commit type: HIGHLIGHT";
        let rows = render_styled(src);
        // Normal commit ● should be green.
        let green_text: String = rows
            .iter()
            .flat_map(|r| r.iter())
            .filter(|r| r.color == Some(Color::Green))
            .map(|r| r.text.as_str())
            .collect();
        assert!(
            green_text.contains('●'),
            "expected green normal commit, got rows={rows:?}"
        );
        // Highlight commit ◉ should be yellow.
        let yellow_text: String = rows
            .iter()
            .flat_map(|r| r.iter())
            .filter(|r| r.color == Some(Color::Yellow))
            .map(|r| r.text.as_str())
            .collect();
        assert!(
            yellow_text.contains('◉'),
            "expected yellow highlight commit, got rows={rows:?}"
        );
        // Lane label "main:" should be cyan.
        let cyan_text: String = rows
            .iter()
            .flat_map(|r| r.iter())
            .filter(|r| r.color == Some(Color::Cyan))
            .map(|r| r.text.as_str())
            .collect();
        assert!(
            cyan_text.contains("main"),
            "expected cyan main lane label, got rows={rows:?}"
        );
    }

    #[test]
    fn render_styled_no_alignment_drift() {
        let src = "gitGraph\n    commit\n    branch feature\n    checkout feature\n    commit\n    checkout main\n    merge feature";
        let plain = crate::mermaid::ascii::gitgraph::render(
            src,
            100,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap();
        let styled = render_styled(src);
        assert_eq!(plain.len(), styled.len());
        for (i, row) in styled.iter().enumerate() {
            let flat: String = row.iter().map(|r| r.text.as_str()).collect();
            let flat_trimmed = flat.trim_end().to_string();
            assert_eq!(flat_trimmed, plain[i], "row {i} drift");
        }
    }
}
