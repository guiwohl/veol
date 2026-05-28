use super::ast::{GanttChart, Section, Task, TaskStatus};
use crate::mermaid::ascii::error::AsciiRenderError;

pub fn parse(source: &str) -> Result<GanttChart, AsciiRenderError> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    let cleaned = strip_frontmatter(trimmed);
    let raw_lines: Vec<&str> = cleaned.lines().collect();

    let mut header_idx: Option<usize> = None;
    for (i, line) in raw_lines.iter().enumerate() {
        let t = strip_comment(line).trim();
        if t.is_empty() {
            continue;
        }
        if t == "gantt" || t.starts_with("gantt ") || t.starts_with("gantt\t") {
            header_idx = Some(i);
            break;
        } else {
            return Err(AsciiRenderError::Parse {
                line: i + 1,
                msg: format!("expected 'gantt' header, got: {t:?}"),
            });
        }
    }
    let header_idx = header_idx.ok_or(AsciiRenderError::Empty)?;

    let mut chart = GanttChart::default();
    let mut current_section: Option<Section> = None;
    let mut absolute_anchor: Option<NaiveDate> = None;

    for (offset, raw) in raw_lines.iter().enumerate().skip(header_idx + 1) {
        let stripped = strip_comment(raw);
        let line = stripped.trim();
        if line.is_empty() {
            continue;
        }
        let lineno = offset + 1;

        if let Some(rest) = strip_directive(line, "title") {
            chart.title = Some(rest.to_string());
            continue;
        }
        if let Some(rest) = strip_directive(line, "dateFormat") {
            chart.date_format = Some(rest.to_string());
            continue;
        }
        if let Some(rest) = strip_directive(line, "axisFormat") {
            chart.axis_format = Some(rest.to_string());
            continue;
        }
        if strip_directive(line, "excludes").is_some()
            || strip_directive(line, "todayMarker").is_some()
            || strip_directive(line, "tickInterval").is_some()
            || strip_directive(line, "weekday").is_some()
        {
            continue;
        }
        if let Some(rest) = strip_directive(line, "section") {
            if let Some(sec) = current_section.take() {
                chart.sections.push(sec);
            }
            current_section = Some(Section {
                name: rest.to_string(),
                tasks: Vec::new(),
            });
            continue;
        }

        let task = parse_task_line(line, lineno, &mut absolute_anchor)?;
        match current_section.as_mut() {
            Some(sec) => sec.tasks.push(task),
            None => {
                let sec = current_section.get_or_insert(Section {
                    name: String::new(),
                    tasks: Vec::new(),
                });
                sec.tasks.push(task);
            }
        }
    }

    if let Some(sec) = current_section.take() {
        chart.sections.push(sec);
    }

    resolve_start_days(&mut chart);
    Ok(chart)
}

fn parse_task_line(
    line: &str,
    lineno: usize,
    anchor: &mut Option<NaiveDate>,
) -> Result<Task, AsciiRenderError> {
    let colon = line.find(':').ok_or_else(|| AsciiRenderError::Parse {
        line: lineno,
        msg: format!("expected '<label> :<modifiers>', got: {line:?}"),
    })?;
    let label = line[..colon].trim().to_string();
    let rest = line[colon + 1..].trim();

    if label.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: "empty task label".into(),
        });
    }

    let tokens: Vec<String> = rest
        .split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    if tokens.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: "missing task fields".into(),
        });
    }

    let mut status = TaskStatus::Default;
    let mut id: Option<String> = None;
    let mut depends_on: Option<String> = None;
    let mut absolute_date: Option<NaiveDate> = None;
    let mut duration_days: Option<i64> = None;

    for tok in &tokens {
        if tok.eq_ignore_ascii_case("done") {
            status = TaskStatus::Done;
        } else if tok.eq_ignore_ascii_case("active") {
            status = TaskStatus::Active;
        } else if tok.eq_ignore_ascii_case("crit") {
            status = TaskStatus::Crit;
        } else if tok.eq_ignore_ascii_case("milestone") {
            status = TaskStatus::Milestone;
        } else if let Some(after_id) = tok
            .strip_prefix("after ")
            .or_else(|| tok.strip_prefix("after\t"))
        {
            depends_on = Some(after_id.trim().to_string());
        } else if let Some(days) = parse_duration(tok) {
            duration_days = Some(days);
        } else if let Some(d) = parse_iso_date(tok) {
            absolute_date = Some(d);
        } else {
            id = Some(tok.clone());
        }
    }

    let start_day: i64 = if let Some(date) = absolute_date {
        match anchor {
            Some(a) => days_between(*a, date),
            None => {
                *anchor = Some(date);
                0
            }
        }
    } else if depends_on.is_some() {
        0_i64 - dep_marker_value()
    } else {
        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: "task missing start (absolute date or 'after <id>')".into(),
        });
    };

    let duration_days = match duration_days {
        Some(d) => d,
        None if status == TaskStatus::Milestone => 0,
        None => {
            return Err(AsciiRenderError::Parse {
                line: lineno,
                msg: "task missing duration".into(),
            });
        }
    };

    let id = id.unwrap_or_else(|| slugify(&label));

    Ok(Task {
        id,
        label,
        status,
        start_day,
        duration_days,
        depends_on,
    })
}

fn dep_marker_value() -> i64 {
    1_000_000_000
}

fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if c.is_whitespace() {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("task");
    }
    out
}

fn parse_duration(tok: &str) -> Option<i64> {
    let t = tok.trim();
    if t.len() < 2 {
        return None;
    }
    let (num_str, unit) = t.split_at(t.len() - 1);
    let n: i64 = num_str.parse().ok()?;
    match unit {
        "d" => Some(n.max(0)),
        "w" => Some(n.saturating_mul(7).max(0)),
        "h" => {
            let days = (n + 23) / 24;
            Some(days.max(if n > 0 { 1 } else { 0 }))
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NaiveDate {
    y: i32,
    m: u32,
    d: u32,
}

fn parse_iso_date(s: &str) -> Option<NaiveDate> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: u32 = parts[2].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(NaiveDate { y, m, d })
}

fn days_between(a: NaiveDate, b: NaiveDate) -> i64 {
    julian(b) - julian(a)
}

fn julian(d: NaiveDate) -> i64 {
    let (y, m) = if d.m <= 2 {
        (d.y - 1, d.m + 12)
    } else {
        (d.y, d.m)
    };
    let a = y.div_euclid(100);
    let b = 2 - a + a.div_euclid(4);
    (365.25_f64 * (y as f64 + 4716.0)).floor() as i64
        + (30.6001_f64 * (m as f64 + 1.0)).floor() as i64
        + d.d as i64
        + b as i64
        - 1524
}

fn strip_directive<'a>(line: &'a str, kw: &str) -> Option<&'a str> {
    let s = line.trim();
    let rest = s.strip_prefix(kw)?;
    if rest.is_empty() {
        return Some("");
    }
    let next = rest.chars().next().unwrap();
    if next.is_whitespace() {
        Some(rest.trim())
    } else {
        None
    }
}

fn strip_comment(line: &str) -> &str {
    if let Some(idx) = line.find("%%") {
        &line[..idx]
    } else {
        line
    }
}

fn strip_frontmatter(source: &str) -> &str {
    let s = source.trim_start_matches('\n');
    if let Some(rest) = s.strip_prefix("---\n") {
        if let Some(end_idx) = rest.find("\n---") {
            let after = &rest[end_idx + 4..];
            return after.trim_start_matches('\n');
        }
    }
    source
}

fn resolve_start_days(chart: &mut GanttChart) {
    let marker = -dep_marker_value();
    let mut by_id: std::collections::HashMap<String, (i64, i64)> = std::collections::HashMap::new();
    for section in chart.sections.iter_mut() {
        for task in section.tasks.iter_mut() {
            if task.start_day == marker {
                task.start_day = task
                    .depends_on
                    .as_ref()
                    .and_then(|dep| by_id.get(dep))
                    .map(|(s, dur)| s + dur)
                    .unwrap_or(0);
            }
            by_id.insert(task.id.clone(), (task.start_day, task.duration_days));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_gantt_minimal() {
        let src = "gantt\nsection A\nTask 1 :t1, 2024-01-01, 5d";
        let g = parse(src).unwrap();
        assert_eq!(g.sections.len(), 1);
        assert_eq!(g.sections[0].tasks.len(), 1);
    }

    #[test]
    fn parse_no_header_error() {
        let src = "graph TD\nA --> B";
        let err = parse(src).unwrap_err();
        match err {
            AsciiRenderError::Parse { .. } => {}
            e => panic!("expected Parse error, got {e:?}"),
        }
    }

    #[test]
    fn parse_with_title() {
        let src = "gantt\ntitle My Plan\nsection A\nT :t1, 2024-01-01, 2d";
        let g = parse(src).unwrap();
        assert_eq!(g.title.as_deref(), Some("My Plan"));
    }

    #[test]
    fn parse_date_format_directive() {
        let src = "gantt\ndateFormat YYYY-MM-DD\nsection A\nT :t1, 2024-01-01, 2d";
        let g = parse(src).unwrap();
        assert_eq!(g.date_format.as_deref(), Some("YYYY-MM-DD"));
    }

    #[test]
    fn parse_axis_format_directive() {
        let src = "gantt\naxisFormat %m-%d\nsection A\nT :t1, 2024-01-01, 2d";
        let g = parse(src).unwrap();
        assert_eq!(g.axis_format.as_deref(), Some("%m-%d"));
    }

    #[test]
    fn parse_section_header() {
        let src =
            "gantt\nsection Alpha\nT :t1, 2024-01-01, 1d\nsection Beta\nU :u1, 2024-01-02, 1d";
        let g = parse(src).unwrap();
        assert_eq!(g.sections.len(), 2);
        assert_eq!(g.sections[0].name, "Alpha");
        assert_eq!(g.sections[1].name, "Beta");
    }

    #[test]
    fn parse_task_with_absolute_date() {
        let src = "gantt\nsection A\nFoo :f1, 2024-01-01, 3d\nBar :b1, 2024-01-05, 2d";
        let g = parse(src).unwrap();
        let t0 = &g.sections[0].tasks[0];
        let t1 = &g.sections[0].tasks[1];
        assert_eq!(t0.start_day, 0);
        assert_eq!(t1.start_day, 4);
    }

    #[test]
    fn parse_task_with_after_dependency() {
        let src = "gantt\nsection A\nFoo :f1, 2024-01-01, 3d\nBar :b1, after f1, 2d";
        let g = parse(src).unwrap();
        let t1 = &g.sections[0].tasks[1];
        assert_eq!(t1.depends_on.as_deref(), Some("f1"));
        assert_eq!(t1.start_day, 3);
    }

    #[test]
    fn parse_task_status_active() {
        let src = "gantt\nsection A\nT :active, t1, 2024-01-01, 2d";
        let g = parse(src).unwrap();
        assert_eq!(g.sections[0].tasks[0].status, TaskStatus::Active);
    }

    #[test]
    fn parse_task_status_done() {
        let src = "gantt\nsection A\nT :done, t1, 2024-01-01, 2d";
        let g = parse(src).unwrap();
        assert_eq!(g.sections[0].tasks[0].status, TaskStatus::Done);
    }

    #[test]
    fn parse_task_status_crit() {
        let src = "gantt\nsection A\nT :crit, t1, 2024-01-01, 2d";
        let g = parse(src).unwrap();
        assert_eq!(g.sections[0].tasks[0].status, TaskStatus::Crit);
    }

    #[test]
    fn parse_task_status_milestone() {
        let src = "gantt\nsection A\nM :milestone, m1, 2024-01-01, 0d";
        let g = parse(src).unwrap();
        assert_eq!(g.sections[0].tasks[0].status, TaskStatus::Milestone);
        assert_eq!(g.sections[0].tasks[0].duration_days, 0);
    }

    #[test]
    fn parse_task_duration_days() {
        let src = "gantt\nsection A\nT :t1, 2024-01-01, 5d";
        let g = parse(src).unwrap();
        assert_eq!(g.sections[0].tasks[0].duration_days, 5);
    }

    #[test]
    fn parse_task_duration_hours() {
        let src = "gantt\nsection A\nT :t1, 2024-01-01, 36h";
        let g = parse(src).unwrap();
        assert_eq!(g.sections[0].tasks[0].duration_days, 2);
    }

    #[test]
    fn parse_task_duration_weeks() {
        let src = "gantt\nsection A\nT :t1, 2024-01-01, 2w";
        let g = parse(src).unwrap();
        assert_eq!(g.sections[0].tasks[0].duration_days, 14);
    }

    #[test]
    fn parse_multiple_sections() {
        let src = "gantt\nsection A\nT :t1, 2024-01-01, 2d\nsection B\nU :u1, 2024-01-05, 1d\nV :v1, 2024-01-07, 3d";
        let g = parse(src).unwrap();
        assert_eq!(g.sections.len(), 2);
        assert_eq!(g.sections[1].tasks.len(), 2);
    }

    #[test]
    fn parse_comments_stripped() {
        let src =
            "%% leading\ngantt\n%% inside\ntitle X\nsection A %% inline\nT :t1, 2024-01-01, 1d";
        let g = parse(src).unwrap();
        assert_eq!(g.title.as_deref(), Some("X"));
        assert_eq!(g.sections[0].name, "A");
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let src = "\n\ngantt\n\ntitle Y\n\nsection A\n\nT :t1, 2024-01-01, 1d\n\n";
        let g = parse(src).unwrap();
        assert_eq!(g.title.as_deref(), Some("Y"));
        assert_eq!(g.sections[0].tasks.len(), 1);
    }
}
