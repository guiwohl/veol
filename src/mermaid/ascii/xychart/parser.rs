use super::ast::{Orientation, Series, XyChart};
use crate::mermaid::ascii::error::AsciiRenderError;

pub fn parse(source: &str) -> Result<XyChart, AsciiRenderError> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    let raw_lines: Vec<&str> = trimmed.lines().collect();

    let mut header_idx: Option<usize> = None;
    for (i, line) in raw_lines.iter().enumerate() {
        let t = strip_comment(line).trim();
        if t.is_empty() {
            continue;
        }
        if let Some(rest) = strip_keyword_prefix(t, "xychart-beta") {
            header_idx = Some(i);
            // capture orientation from header tail
            let _ = rest;
            break;
        } else {
            return Err(AsciiRenderError::Parse {
                line: i + 1,
                msg: format!("expected 'xychart-beta' header, got: {t:?}"),
            });
        }
    }
    let header_idx = header_idx.ok_or(AsciiRenderError::Empty)?;

    let mut chart = XyChart::default();
    let header = strip_comment(raw_lines[header_idx]).trim();
    if let Some(rest) = strip_keyword_prefix(header, "xychart-beta") {
        let tail = rest.trim();
        if !tail.is_empty() {
            match tail {
                "horizontal" => chart.orientation = Some(Orientation::Horizontal),
                "vertical" => chart.orientation = Some(Orientation::Vertical),
                other => {
                    return Err(AsciiRenderError::Parse {
                        line: header_idx + 1,
                        msg: format!("unknown orientation: {other:?}"),
                    });
                }
            }
        }
    }

    for (offset, raw) in raw_lines.iter().enumerate().skip(header_idx + 1) {
        let stripped = strip_comment(raw);
        let line = stripped.trim();
        if line.is_empty() {
            continue;
        }
        let lineno = offset + 1;
        parse_body_line(line, lineno, &mut chart)?;
    }

    Ok(chart)
}

fn parse_body_line(line: &str, lineno: usize, chart: &mut XyChart) -> Result<(), AsciiRenderError> {
    if let Some(rest) = strip_keyword_prefix(line, "title") {
        chart.title = Some(parse_title(rest.trim()));
        return Ok(());
    }
    if let Some(rest) = strip_keyword_prefix(line, "x-axis") {
        parse_axis(rest.trim(), lineno, chart, true)?;
        return Ok(());
    }
    if let Some(rest) = strip_keyword_prefix(line, "y-axis") {
        parse_axis(rest.trim(), lineno, chart, false)?;
        return Ok(());
    }
    if let Some(rest) = strip_keyword_prefix(line, "bar") {
        let values = parse_values(rest.trim(), lineno)?;
        chart.series.push(Series::Bar(values));
        return Ok(());
    }
    if let Some(rest) = strip_keyword_prefix(line, "line") {
        let values = parse_values(rest.trim(), lineno)?;
        chart.series.push(Series::Line(values));
        return Ok(());
    }
    Err(AsciiRenderError::Parse {
        line: lineno,
        msg: format!("unexpected line: {line:?}"),
    })
}

fn parse_title(s: &str) -> String {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        inner.to_string()
    } else {
        s.to_string()
    }
}

fn parse_axis(
    rest: &str,
    lineno: usize,
    chart: &mut XyChart,
    is_x: bool,
) -> Result<(), AsciiRenderError> {
    let mut cursor = rest.trim().to_string();

    // optional quoted title
    let mut title: Option<String> = None;
    if cursor.starts_with('"') {
        let after_first = &cursor[1..];
        if let Some(end) = after_first.find('"') {
            title = Some(after_first[..end].to_string());
            cursor = after_first[end + 1..].trim().to_string();
        } else {
            return Err(AsciiRenderError::Parse {
                line: lineno,
                msg: "unterminated quoted axis title".into(),
            });
        }
    }

    if cursor.starts_with('[') {
        let end = cursor.rfind(']').ok_or_else(|| AsciiRenderError::Parse {
            line: lineno,
            msg: "missing closing bracket for axis categories".into(),
        })?;
        let inner = &cursor[1..end];
        let cats: Vec<String> = inner
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !is_x {
            return Err(AsciiRenderError::Parse {
                line: lineno,
                msg: "y-axis with categories not supported".into(),
            });
        }
        chart.x_categories = cats;
        if let Some(t) = title {
            chart.x_axis_title = Some(t);
        }
        return Ok(());
    }

    // expect numeric range: lo --> hi
    let arrow_idx = cursor.find("-->").ok_or_else(|| AsciiRenderError::Parse {
        line: lineno,
        msg: format!("expected '[...]' or '<lo> --> <hi>' axis spec, got: {cursor:?}"),
    })?;
    let lo_str = cursor[..arrow_idx].trim();
    let hi_str = cursor[arrow_idx + 3..].trim();
    let lo: f64 = lo_str.parse().map_err(|_| AsciiRenderError::Parse {
        line: lineno,
        msg: format!("invalid axis low: {lo_str:?}"),
    })?;
    let hi: f64 = hi_str.parse().map_err(|_| AsciiRenderError::Parse {
        line: lineno,
        msg: format!("invalid axis high: {hi_str:?}"),
    })?;
    if is_x {
        chart.x_range = Some((lo, hi));
        if let Some(t) = title {
            chart.x_axis_title = Some(t);
        }
    } else {
        chart.y_range = Some((lo, hi));
        if let Some(t) = title {
            chart.y_axis_title = Some(t);
        }
    }
    Ok(())
}

fn parse_values(rest: &str, lineno: usize) -> Result<Vec<f64>, AsciiRenderError> {
    let s = rest.trim();
    let inner = s
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| AsciiRenderError::Parse {
            line: lineno,
            msg: format!("expected '[v1, v2, ...]', got: {s:?}"),
        })?;
    let mut out = Vec::new();
    for part in inner.split(',') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        let v: f64 = p.parse().map_err(|_| AsciiRenderError::Parse {
            line: lineno,
            msg: format!("invalid numeric value: {p:?}"),
        })?;
        out.push(v);
    }
    Ok(out)
}

fn strip_comment(line: &str) -> &str {
    if let Some(idx) = line.find("%%") {
        &line[..idx]
    } else {
        line
    }
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
    fn parse_xychart_minimal() {
        let src = "xychart-beta\nbar [1, 2, 3]";
        let c = parse(src).unwrap();
        assert!(c.title.is_none());
        assert_eq!(c.series.len(), 1);
        match &c.series[0] {
            Series::Bar(v) => assert_eq!(v, &vec![1.0, 2.0, 3.0]),
            _ => panic!("expected Bar"),
        }
    }

    #[test]
    fn parse_no_header_error() {
        let src = "pie\n\"A\" : 1";
        let err = parse(src).unwrap_err();
        match err {
            AsciiRenderError::Parse { .. } => {}
            e => panic!("expected Parse, got {e:?}"),
        }
    }

    #[test]
    fn parse_horizontal_orientation() {
        let src = "xychart-beta horizontal\nbar [1, 2]";
        let c = parse(src).unwrap();
        assert_eq!(c.orientation, Some(Orientation::Horizontal));
    }

    #[test]
    fn parse_title_quoted() {
        let src = "xychart-beta\ntitle \"Sales Revenue\"\nbar [1]";
        let c = parse(src).unwrap();
        assert_eq!(c.title.as_deref(), Some("Sales Revenue"));
    }

    #[test]
    fn parse_x_axis_categories() {
        let src = "xychart-beta\nx-axis [jan, feb, mar]\nbar [1, 2, 3]";
        let c = parse(src).unwrap();
        assert_eq!(c.x_categories, vec!["jan", "feb", "mar"]);
        assert!(c.x_range.is_none());
    }

    #[test]
    fn parse_x_axis_range() {
        let src = "xychart-beta\nx-axis \"Months\" 1 --> 12\nbar [1]";
        let c = parse(src).unwrap();
        assert_eq!(c.x_range, Some((1.0, 12.0)));
        assert_eq!(c.x_axis_title.as_deref(), Some("Months"));
    }

    #[test]
    fn parse_y_axis_with_title_and_range() {
        let src = "xychart-beta\ny-axis \"Revenue (in $)\" 4000 --> 11000\nbar [5000]";
        let c = parse(src).unwrap();
        assert_eq!(c.y_axis_title.as_deref(), Some("Revenue (in $)"));
        assert_eq!(c.y_range, Some((4000.0, 11000.0)));
    }

    #[test]
    fn parse_bar_series() {
        let src = "xychart-beta\nbar [1, 2, 3, 4]";
        let c = parse(src).unwrap();
        assert_eq!(c.series.len(), 1);
        match &c.series[0] {
            Series::Bar(v) => assert_eq!(v, &vec![1.0, 2.0, 3.0, 4.0]),
            _ => panic!("expected Bar"),
        }
    }

    #[test]
    fn parse_line_series() {
        let src = "xychart-beta\nline [10.5, 20.25, 30.75]";
        let c = parse(src).unwrap();
        match &c.series[0] {
            Series::Line(v) => {
                assert!((v[0] - 10.5).abs() < 1e-9);
                assert!((v[1] - 20.25).abs() < 1e-9);
                assert!((v[2] - 30.75).abs() < 1e-9);
            }
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn parse_multiple_series() {
        let src = "xychart-beta\nbar [1, 2, 3]\nline [4, 5, 6]";
        let c = parse(src).unwrap();
        assert_eq!(c.series.len(), 2);
        assert!(matches!(c.series[0], Series::Bar(_)));
        assert!(matches!(c.series[1], Series::Line(_)));
    }

    #[test]
    fn parse_comments_stripped() {
        let src = "%% top\nxychart-beta\n%% mid\nbar [1, 2] %% inline\n";
        let c = parse(src).unwrap();
        assert_eq!(c.series.len(), 1);
    }
}
