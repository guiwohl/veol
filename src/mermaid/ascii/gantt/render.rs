use super::ast::{GanttChart, Task, TaskStatus};
use crate::mermaid::ascii::canvas::Canvas;
use crate::mermaid::ascii::{AsciiRenderError, CharsetKind, StyledRow};
use ratatui::style::Color;
use unicode_width::UnicodeWidthStr;

const MIN_BAR_WIDTH: usize = 12;
const MAX_LABEL_COL: usize = 24;
const LABEL_GAP: usize = 2;
const MIN_TOTAL_WIDTH: usize = 40;

pub fn render(
    diag: &GanttChart,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_lines())
}

pub fn render_styled(
    diag: &GanttChart,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_styled_lines())
}

fn render_to_canvas(
    diag: &GanttChart,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    if diag.sections.is_empty() || diag.sections.iter().all(|s| s.tasks.is_empty()) {
        return Err(AsciiRenderError::Empty);
    }

    let glyphs = Glyphs::for_charset(charset);

    let max_end: i64 = diag
        .sections
        .iter()
        .flat_map(|s| s.tasks.iter())
        .map(|t| t.start_day + t.duration_days.max(1))
        .max()
        .unwrap_or(1);
    let max_end = max_end.max(1) as usize;

    let widest_label = diag
        .sections
        .iter()
        .flat_map(|s| s.tasks.iter())
        .map(|t| label_for(t).chars().count())
        .max()
        .unwrap_or(0)
        .min(MAX_LABEL_COL);

    let effective_width = (max_width as usize).max(MIN_TOTAL_WIDTH);
    let overhead = widest_label + LABEL_GAP + 2 + 2;
    let bar_width = if overhead + MIN_BAR_WIDTH > effective_width {
        MIN_BAR_WIDTH
    } else {
        effective_width - overhead
    };
    let bar_width = bar_width.max(MIN_BAR_WIDTH);

    let axis_indent = 2 + widest_label + LABEL_GAP + 1;
    let total_width = (axis_indent + bar_width + 1).max(1);

    // Height
    let mut height = 0usize;
    if diag.title.is_some() {
        height += 2;
    }
    height += 2; // axis label + axis tick row
    for section in &diag.sections {
        if !section.name.is_empty() {
            height += 1;
        }
        height += section.tasks.len();
    }
    let title_width = diag
        .title
        .as_ref()
        .map(|t| UnicodeWidthStr::width(t.as_str()))
        .unwrap_or(0);
    let total_width = total_width.max(title_width).max(1);
    let height = height.max(1);

    let mut canvas = Canvas::new(total_width, height, charset);

    let mut y = 0usize;
    if let Some(title) = &diag.title {
        canvas.put_str_colored(0, y, title, Color::White);
        y += 2;
    }

    paint_axis_labels(&mut canvas, y, max_end, bar_width, axis_indent);
    y += 1;
    paint_axis_ticks(&mut canvas, y, bar_width, axis_indent, &glyphs);
    y += 1;

    for section in &diag.sections {
        if !section.name.is_empty() {
            canvas.put_str_colored(0, y, &section.name, Color::Blue);
            y += 1;
        }
        for task in &section.tasks {
            paint_task(
                &mut canvas,
                y,
                task,
                widest_label,
                bar_width,
                max_end,
                &glyphs,
            );
            y += 1;
        }
    }

    Ok(canvas)
}

struct Glyphs {
    fill_default: char,
    fill_active: char,
    fill_done: char,
    fill_crit: char,
    empty: char,
    milestone: char,
    bar_open: char,
    bar_close: char,
    axis_left: char,
    axis_right: char,
    axis_tick: char,
    axis_line: char,
}

impl Glyphs {
    fn for_charset(c: CharsetKind) -> Self {
        match c {
            CharsetKind::Unicode => Glyphs {
                fill_default: '█',
                fill_active: '▓',
                fill_done: '▒',
                fill_crit: '█',
                empty: '░',
                milestone: '◆',
                bar_open: '[',
                bar_close: ']',
                axis_left: '├',
                axis_right: '┤',
                axis_tick: '┼',
                axis_line: '─',
            },
            CharsetKind::Ascii => Glyphs {
                fill_default: '#',
                fill_active: '+',
                fill_done: '.',
                fill_crit: '*',
                empty: ' ',
                milestone: 'o',
                bar_open: '[',
                bar_close: ']',
                axis_left: '|',
                axis_right: '|',
                axis_tick: '+',
                axis_line: '-',
            },
        }
    }
}

fn label_for(task: &Task) -> String {
    match task.status {
        TaskStatus::Active => format!("{} (active)", task.label),
        TaskStatus::Done => format!("{} (done)", task.label),
        TaskStatus::Crit => format!("{} (crit)", task.label),
        TaskStatus::Milestone => task.label.clone(),
        TaskStatus::Default => task.label.clone(),
    }
}

fn paint_task(
    canvas: &mut Canvas,
    y: usize,
    task: &Task,
    label_col: usize,
    bar_width: usize,
    max_end: usize,
    g: &Glyphs,
) {
    let label_text = label_for(task);
    let label = pad_or_truncate(&label_text, label_col);

    // Indent of 2 spaces, then label, then LABEL_GAP, then "[bar]"
    canvas.put_str(2, y, &label);

    let bracket_open_x = 2 + label_col + LABEL_GAP;
    canvas.put_char(bracket_open_x, y, g.bar_open);
    let bar_x = bracket_open_x + 1;

    // Fill bar with empty
    for i in 0..bar_width {
        canvas.put_char(bar_x + i, y, g.empty);
        canvas.set_color(bar_x + i, y, Color::DarkGray);
    }

    let start_cell = day_to_cell(task.start_day.max(0) as usize, max_end, bar_width);
    if task.status == TaskStatus::Milestone {
        let idx = start_cell.min(bar_width.saturating_sub(1));
        canvas.put_char(bar_x + idx, y, g.milestone);
        canvas.set_color(bar_x + idx, y, Color::Magenta);
    } else {
        let end_day = (task.start_day + task.duration_days.max(1)) as usize;
        let end_cell = day_to_cell(end_day, max_end, bar_width);
        let end_cell = end_cell.max(start_cell + 1).min(bar_width);
        let (fill_char, fill_color) = match task.status {
            TaskStatus::Active => (g.fill_active, Color::Yellow),
            TaskStatus::Done => (g.fill_done, Color::Green),
            TaskStatus::Crit => (g.fill_crit, Color::Red),
            _ => (g.fill_default, Color::Cyan),
        };
        for i in start_cell..end_cell {
            canvas.put_char(bar_x + i, y, fill_char);
            canvas.set_color(bar_x + i, y, fill_color);
        }
    }

    canvas.put_char(bar_x + bar_width, y, g.bar_close);
}

fn day_to_cell(day: usize, max_end: usize, bar_width: usize) -> usize {
    if max_end == 0 {
        return 0;
    }
    let cell = (day * bar_width) / max_end;
    cell.min(bar_width)
}

fn paint_axis_labels(
    canvas: &mut Canvas,
    y: usize,
    max_end: usize,
    bar_width: usize,
    indent: usize,
) {
    let step = pick_tick_step(max_end);
    let mut d = 0;
    while d <= max_end {
        let cell = day_to_cell(d, max_end, bar_width);
        let label = if d == 0 {
            "Day 0".to_string()
        } else {
            d.to_string()
        };
        let start = indent + cell;
        canvas.put_str_colored(start, y, &label, Color::Green);
        d += step;
    }
}

fn paint_axis_ticks(canvas: &mut Canvas, y: usize, bar_width: usize, indent: usize, g: &Glyphs) {
    // Axis line
    for x in indent..=indent + bar_width {
        canvas.put_char(x, y, g.axis_line);
        canvas.set_color(x, y, Color::DarkGray);
    }
    canvas.put_char(indent, y, g.axis_left);
    canvas.set_color(indent, y, Color::DarkGray);
    canvas.put_char(indent + bar_width, y, g.axis_right);
    canvas.set_color(indent + bar_width, y, Color::DarkGray);

    let step_cells = pick_tick_cells(bar_width);
    let mut cell = step_cells;
    while cell < bar_width {
        canvas.put_char(indent + cell, y, g.axis_tick);
        canvas.set_color(indent + cell, y, Color::DarkGray);
        cell += step_cells;
    }
}

fn pick_tick_step(max_end: usize) -> usize {
    if max_end <= 10 {
        1
    } else if max_end <= 30 {
        5
    } else if max_end <= 90 {
        10
    } else {
        (max_end / 8).max(1)
    }
}

fn pick_tick_cells(bar_width: usize) -> usize {
    (bar_width / 5).max(1)
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
    use crate::mermaid::ascii::StyledRun;
    use ratatui::style::Color;

    fn render(src: &str) -> String {
        crate::mermaid::ascii::gantt::render(src, 100, crate::mermaid::ascii::CharsetKind::Unicode)
            .unwrap()
            .join("\n")
    }

    fn render_ascii(src: &str) -> String {
        crate::mermaid::ascii::gantt::render(src, 100, crate::mermaid::ascii::CharsetKind::Ascii)
            .unwrap()
            .join("\n")
    }

    #[test]
    fn render_gantt_minimal() {
        let src = "gantt\nsection A\nTask 1 :t1, 2024-01-01, 5d";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_with_title() {
        let src = "gantt\ntitle Project Plan\nsection A\nTask 1 :t1, 2024-01-01, 5d";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_section_with_tasks() {
        let src = "gantt\nsection Build\nCompile :c1, 2024-01-01, 3d\nTest :t1, 2024-01-04, 2d";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_task_active() {
        let src = "gantt\nsection A\nT1 :a1, 2024-01-01, 4d\nT2 :active, a2, 2024-01-05, 3d";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_task_done() {
        let src = "gantt\nsection A\nT1 :done, a1, 2024-01-01, 4d\nT2 :a2, 2024-01-05, 3d";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_task_crit() {
        let src = "gantt\nsection A\nT1 :a1, 2024-01-01, 3d\nT2 :crit, a2, 2024-01-04, 4d";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_milestone() {
        let src = "gantt\nsection A\nT1 :a1, 2024-01-01, 5d\nLaunch :milestone, m1, 2024-01-06, 0d";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_dependent_task_after() {
        let src = "gantt\nsection A\nFoo :f1, 2024-01-01, 3d\nBar :b1, after f1, 4d";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_two_sections() {
        let src = "gantt\nsection Alpha\nA1 :a1, 2024-01-01, 3d\nsection Beta\nB1 :b1, 2024-01-05, 4d\nB2 :b2, 2024-01-10, 2d";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_long_task_label_truncates() {
        let src = "gantt\nsection A\nThis is a really really long task label that overflows :t1, 2024-01-01, 5d";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_ascii_charset() {
        let src = "gantt\ntitle ASCII\nsection A\nT1 :t1, 2024-01-01, 3d\nT2 :active, t2, 2024-01-05, 2d\nM :milestone, m1, 2024-01-08, 0d";
        insta::assert_snapshot!(render_ascii(src));
    }

    #[test]
    fn render_axis_row_present() {
        let src = "gantt\nsection A\nT :t1, 2024-01-01, 5d";
        let out = render(src);
        assert!(
            out.contains('├') && out.contains('┤'),
            "expected axis line endpoints, got:\n{out}"
        );
    }

    #[test]
    fn render_each_task_on_own_row() {
        let src =
            "gantt\nsection A\nA :a1, 2024-01-01, 2d\nB :b1, 2024-01-03, 2d\nC :c1, 2024-01-05, 2d";
        let out = render(src);
        let bar_rows = out
            .lines()
            .filter(|l| l.contains('[') && l.contains(']'))
            .count();
        assert_eq!(bar_rows, 3, "expected 3 bar rows, got:\n{out}");
    }

    #[test]
    fn render_bar_length_proportional_to_duration() {
        let src = "gantt\nsection A\nShort :s1, 2024-01-01, 1d\nLong :l1, 2024-01-02, 10d";
        let out = render(src);
        let lines: Vec<&str> = out.lines().filter(|l| l.contains('[')).collect();
        assert_eq!(lines.len(), 2);
        let short_fill = lines[0].chars().filter(|&c| c == '█').count();
        let long_fill = lines[1].chars().filter(|&c| c == '█').count();
        assert!(
            long_fill > short_fill,
            "long ({long_fill}) should be wider than short ({short_fill}):\n{out}"
        );
    }

    #[test]
    fn render_milestone_has_special_marker() {
        let src = "gantt\nsection A\nT :t1, 2024-01-01, 4d\nM :milestone, m1, 2024-01-05, 0d";
        let out = render(src);
        assert!(
            out.contains('◆'),
            "expected milestone marker ◆, got:\n{out}"
        );
    }

    #[test]
    fn render_active_task_distinct_from_default() {
        let src = "gantt\nsection A\nDef :d1, 2024-01-01, 3d\nAct :active, a1, 2024-01-05, 3d";
        let out = render(src);
        let def_line = out.lines().find(|l| l.contains("Def")).unwrap();
        let act_line = out.lines().find(|l| l.contains("Act")).unwrap();
        let def_default = def_line.chars().filter(|&c| c == '█').count();
        let act_active = act_line.chars().filter(|&c| c == '▓').count();
        assert!(def_default > 0, "default task should use █:\n{out}");
        assert!(act_active > 0, "active task should use ▓:\n{out}");
        assert_eq!(
            act_line.chars().filter(|&c| c == '█').count(),
            0,
            "active line should NOT contain default █:\n{out}"
        );
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
        let src =
            "gantt\ntitle Plan\nsection A\nT1 :t1, 2024-01-01, 3d\nT2 :active, t2, 2024-01-05, 2d";
        let diag = crate::mermaid::ascii::gantt::parser::parse(src).unwrap();
        let rows =
            super::render_styled(&diag, 100, crate::mermaid::ascii::CharsetKind::Unicode).unwrap();
        let colors = distinct_colors(&rows);
        assert!(
            colors.len() >= 4,
            "expected at least 4 distinct colors, got {colors:?}"
        );
    }

    #[test]
    fn render_styled_specific_element_colored() {
        let src = "gantt\nsection A\nT1 :t1, 2024-01-01, 3d\nT2 :active, t2, 2024-01-05, 2d\nT3 :done, t3, 2024-01-08, 2d\nT4 :crit, t4, 2024-01-11, 2d\nM :milestone, m1, 2024-01-14, 0d";
        let diag = crate::mermaid::ascii::gantt::parser::parse(src).unwrap();
        let rows =
            super::render_styled(&diag, 100, crate::mermaid::ascii::CharsetKind::Unicode).unwrap();
        // Section name A → Blue (row 2 after axis label + axis tick)
        let section_color = find_color_at(&rows[2], 'A');
        assert_eq!(section_color, Some(Color::Blue));
        // T1 default task bar → Cyan
        let t1_fill = find_color_at(&rows[3], '█');
        assert_eq!(t1_fill, Some(Color::Cyan));
        // T2 active → Yellow
        let t2_fill = find_color_at(&rows[4], '▓');
        assert_eq!(t2_fill, Some(Color::Yellow));
        // T3 done → Green
        let t3_fill = find_color_at(&rows[5], '▒');
        assert_eq!(t3_fill, Some(Color::Green));
        // T4 crit → Red
        let t4_fill = find_color_at(&rows[6], '█');
        assert_eq!(t4_fill, Some(Color::Red));
        // M milestone → Magenta
        let m_fill = find_color_at(&rows[7], '◆');
        assert_eq!(m_fill, Some(Color::Magenta));
    }

    #[test]
    fn render_styled_no_alignment_drift() {
        let src = "gantt\ntitle Project Plan\nsection Alpha\nA1 :a1, 2024-01-01, 3d\nsection Beta\nB1 :active, b1, 2024-01-05, 4d\nB2 :done, b2, 2024-01-10, 2d\nM :milestone, m1, 2024-01-13, 0d";
        let diag = crate::mermaid::ascii::gantt::parser::parse(src).unwrap();
        let plain = crate::mermaid::ascii::gantt::render(
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
