use super::ast::{QuadrantChart, QuadrantPoint};
use crate::mermaid::ascii::error::AsciiRenderError;

pub fn parse(source: &str) -> Result<QuadrantChart, AsciiRenderError> {
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
        if strip_keyword_prefix(t, "quadrantChart").is_some() {
            header_idx = Some(i);
            break;
        } else {
            return Err(AsciiRenderError::Parse {
                line: i + 1,
                msg: format!("expected 'quadrantChart' header, got: {t:?}"),
            });
        }
    }
    let header_idx = header_idx.ok_or(AsciiRenderError::Empty)?;

    let mut chart = QuadrantChart::default();

    for (offset, raw) in raw_lines.iter().enumerate().skip(header_idx + 1) {
        let stripped = strip_comment(raw);
        let line = stripped.trim();
        if line.is_empty() {
            continue;
        }
        let lineno = offset + 1;
        parse_directive(line, lineno, &mut chart)?;
    }

    Ok(chart)
}

fn parse_directive(
    line: &str,
    lineno: usize,
    chart: &mut QuadrantChart,
) -> Result<(), AsciiRenderError> {
    if let Some(rest) = strip_keyword_prefix(line, "title") {
        let val = rest.trim();
        if !val.is_empty() {
            chart.title = Some(val.to_string());
        }
        return Ok(());
    }
    if let Some(rest) = strip_keyword_prefix(line, "x-axis") {
        let (left, right) = split_arrow(rest.trim());
        if !left.is_empty() {
            chart.x_axis_left = Some(left.to_string());
        }
        if let Some(r) = right {
            if !r.is_empty() {
                chart.x_axis_right = Some(r.to_string());
            }
        }
        return Ok(());
    }
    if let Some(rest) = strip_keyword_prefix(line, "y-axis") {
        let (bottom, top) = split_arrow(rest.trim());
        if !bottom.is_empty() {
            chart.y_axis_bottom = Some(bottom.to_string());
        }
        if let Some(t) = top {
            if !t.is_empty() {
                chart.y_axis_top = Some(t.to_string());
            }
        }
        return Ok(());
    }
    if let Some(rest) = strip_keyword_prefix(line, "quadrant-1") {
        chart.q1_label = non_empty(rest.trim());
        return Ok(());
    }
    if let Some(rest) = strip_keyword_prefix(line, "quadrant-2") {
        chart.q2_label = non_empty(rest.trim());
        return Ok(());
    }
    if let Some(rest) = strip_keyword_prefix(line, "quadrant-3") {
        chart.q3_label = non_empty(rest.trim());
        return Ok(());
    }
    if let Some(rest) = strip_keyword_prefix(line, "quadrant-4") {
        chart.q4_label = non_empty(rest.trim());
        return Ok(());
    }

    if let Some(point) = parse_point(line, lineno)? {
        chart.points.push(point);
        return Ok(());
    }

    Err(AsciiRenderError::Parse {
        line: lineno,
        msg: format!("unrecognized directive: {line:?}"),
    })
}

fn parse_point(line: &str, lineno: usize) -> Result<Option<QuadrantPoint>, AsciiRenderError> {
    let colon_idx = match line.find(':') {
        Some(i) => i,
        None => return Ok(None),
    };
    let (label_part, value_part) = line.split_at(colon_idx);
    let label = label_part.trim();
    let value = value_part[1..].trim();

    if !value.starts_with('[') || !value.ends_with(']') {
        return Ok(None);
    }

    let inner = &value[1..value.len() - 1];
    let comma_idx = inner.find(',').ok_or_else(|| AsciiRenderError::Parse {
        line: lineno,
        msg: format!("point requires two coords: {line:?}"),
    })?;
    let (x_str, y_str_with_comma) = inner.split_at(comma_idx);
    let y_str = y_str_with_comma[1..].trim();
    let x_str = x_str.trim();

    let x: f64 = x_str.parse().map_err(|_| AsciiRenderError::Parse {
        line: lineno,
        msg: format!("invalid x coordinate: {x_str:?}"),
    })?;
    let y: f64 = y_str.parse().map_err(|_| AsciiRenderError::Parse {
        line: lineno,
        msg: format!("invalid y coordinate: {y_str:?}"),
    })?;

    if label.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: "empty point label".into(),
        });
    }

    Ok(Some(QuadrantPoint {
        label: label.to_string(),
        x,
        y,
    }))
}

fn split_arrow(s: &str) -> (&str, Option<&str>) {
    if let Some(idx) = s.find("-->") {
        let left = s[..idx].trim();
        let right = s[idx + 3..].trim();
        (left, Some(right))
    } else {
        (s, None)
    }
}

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
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

fn strip_keyword_prefix<'a>(s: &'a str, kw: &str) -> Option<&'a str> {
    let s = s.trim_start();
    let rest = s.strip_prefix(kw)?;
    if rest.is_empty() {
        return Some(rest);
    }
    let next = rest.chars().next().unwrap();
    if next.is_whitespace() {
        Some(rest)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_quadrant_minimal() {
        let src = "quadrantChart";
        let q = parse(src).unwrap();
        assert_eq!(q.title, None);
        assert!(q.points.is_empty());
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
        let src = "quadrantChart\n    title Reach and engagement of campaigns";
        let q = parse(src).unwrap();
        assert_eq!(
            q.title.as_deref(),
            Some("Reach and engagement of campaigns")
        );
    }

    #[test]
    fn parse_x_axis_with_arrow() {
        let src = "quadrantChart\n    x-axis Low Reach --> High Reach";
        let q = parse(src).unwrap();
        assert_eq!(q.x_axis_left.as_deref(), Some("Low Reach"));
        assert_eq!(q.x_axis_right.as_deref(), Some("High Reach"));
    }

    #[test]
    fn parse_y_axis_with_arrow() {
        let src = "quadrantChart\n    y-axis Low Engagement --> High Engagement";
        let q = parse(src).unwrap();
        assert_eq!(q.y_axis_bottom.as_deref(), Some("Low Engagement"));
        assert_eq!(q.y_axis_top.as_deref(), Some("High Engagement"));
    }

    #[test]
    fn parse_x_axis_simple() {
        let src = "quadrantChart\n    x-axis Reach";
        let q = parse(src).unwrap();
        assert_eq!(q.x_axis_left.as_deref(), Some("Reach"));
        assert_eq!(q.x_axis_right, None);
    }

    #[test]
    fn parse_y_axis_simple() {
        let src = "quadrantChart\n    y-axis Engagement";
        let q = parse(src).unwrap();
        assert_eq!(q.y_axis_bottom.as_deref(), Some("Engagement"));
        assert_eq!(q.y_axis_top, None);
    }

    #[test]
    fn parse_quadrant_labels_all_four() {
        let src = "quadrantChart\n    quadrant-1 We should expand\n    quadrant-2 Need to promote\n    quadrant-3 Re-evaluate\n    quadrant-4 May be improved";
        let q = parse(src).unwrap();
        assert_eq!(q.q1_label.as_deref(), Some("We should expand"));
        assert_eq!(q.q2_label.as_deref(), Some("Need to promote"));
        assert_eq!(q.q3_label.as_deref(), Some("Re-evaluate"));
        assert_eq!(q.q4_label.as_deref(), Some("May be improved"));
    }

    #[test]
    fn parse_point_with_coords() {
        let src = "quadrantChart\n    Campaign A: [0.3, 0.6]";
        let q = parse(src).unwrap();
        assert_eq!(q.points.len(), 1);
        assert_eq!(q.points[0].label, "Campaign A");
        assert!((q.points[0].x - 0.3).abs() < 1e-9);
        assert!((q.points[0].y - 0.6).abs() < 1e-9);
    }

    #[test]
    fn parse_multiple_points() {
        let src = "quadrantChart\n    Campaign A: [0.3, 0.6]\n    Campaign B: [0.45, 0.23]\n    Campaign C: [0.8, 0.9]";
        let q = parse(src).unwrap();
        assert_eq!(q.points.len(), 3);
        assert_eq!(q.points[0].label, "Campaign A");
        assert_eq!(q.points[2].label, "Campaign C");
        assert!((q.points[2].y - 0.9).abs() < 1e-9);
    }

    #[test]
    fn parse_comments_stripped() {
        let src =
            "%% top comment\nquadrantChart\n    title Stuff %% inline\n    Campaign A: [0.5, 0.5]";
        let q = parse(src).unwrap();
        assert_eq!(q.title.as_deref(), Some("Stuff"));
        assert_eq!(q.points.len(), 1);
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let src = "\n\nquadrantChart\n\n    title X\n\n    Campaign A: [0.1, 0.1]\n\n";
        let q = parse(src).unwrap();
        assert_eq!(q.title.as_deref(), Some("X"));
        assert_eq!(q.points.len(), 1);
    }
}
