use super::ast::{PieChart, Slice};
use crate::mermaid::ascii::error::AsciiRenderError;

pub fn parse(source: &str) -> Result<PieChart, AsciiRenderError> {
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
        if strip_keyword_prefix(t, "pie").is_some() {
            header_idx = Some(i);
            break;
        } else {
            return Err(AsciiRenderError::Parse {
                line: i + 1,
                msg: format!("expected 'pie' header, got: {t:?}"),
            });
        }
    }
    let header_idx = header_idx.ok_or(AsciiRenderError::Empty)?;

    let header = strip_comment(raw_lines[header_idx]).trim().to_string();
    let mut chart = PieChart::default();
    parse_header(&header, &mut chart);

    for (offset, raw) in raw_lines.iter().enumerate().skip(header_idx + 1) {
        let stripped = strip_comment(raw);
        let line = stripped.trim();
        if line.is_empty() {
            continue;
        }
        let lineno = offset + 1;
        let slice = parse_slice(line, lineno)?;
        chart.slices.push(slice);
    }

    Ok(chart)
}

fn parse_header(header: &str, chart: &mut PieChart) {
    let rest = match strip_keyword_prefix(header, "pie") {
        Some(r) => r.trim().to_string(),
        None => return,
    };

    let mut rest = rest;

    if let Some(after) = strip_keyword_prefix(&rest, "showData") {
        chart.show_data = true;
        rest = after.trim().to_string();
    }

    if let Some(after) = strip_keyword_prefix(&rest, "title") {
        let title_raw = after.trim();
        let title = title_raw.trim_matches('"').trim();
        if !title.is_empty() {
            chart.title = Some(title.to_string());
        }
    }
}

fn parse_slice(line: &str, lineno: usize) -> Result<Slice, AsciiRenderError> {
    let colon_idx = line.rfind(':').ok_or_else(|| AsciiRenderError::Parse {
        line: lineno,
        msg: format!("expected '<label> : <value>', got: {line:?}"),
    })?;

    let (label_part, value_part) = line.split_at(colon_idx);
    let value_str = value_part[1..].trim();
    let label_raw = label_part.trim();

    let label = if let Some(inner) = label_raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
    {
        inner.to_string()
    } else {
        label_raw.to_string()
    };

    if label.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: "empty slice label".into(),
        });
    }

    let value: f64 = value_str.parse().map_err(|_| AsciiRenderError::Parse {
        line: lineno,
        msg: format!("invalid numeric value: {value_str:?}"),
    })?;

    Ok(Slice { label, value })
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
    fn parse_pie_minimal() {
        let src = "pie\n\"A\" : 1\n\"B\" : 1";
        let p = parse(src).unwrap();
        assert_eq!(p.title, None);
        assert!(!p.show_data);
        assert_eq!(p.slices.len(), 2);
    }

    #[test]
    fn parse_pie_with_title() {
        let src = "pie title My Chart\n\"A\" : 10";
        let p = parse(src).unwrap();
        assert_eq!(p.title.as_deref(), Some("My Chart"));
        assert!(!p.show_data);
        assert_eq!(p.slices.len(), 1);
    }

    #[test]
    fn parse_pie_with_showdata() {
        let src = "pie showData title Stats\n\"A\" : 10";
        let p = parse(src).unwrap();
        assert!(p.show_data);
        assert_eq!(p.title.as_deref(), Some("Stats"));
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
    fn parse_comments_stripped() {
        let src = "%% leading comment\npie title Stuff\n%% inner\n\"A\" : 1\n\"B\" : 2 %% inline";
        let p = parse(src).unwrap();
        assert_eq!(p.title.as_deref(), Some("Stuff"));
        assert_eq!(p.slices.len(), 2);
        assert_eq!(p.slices[1].label, "B");
        assert_eq!(p.slices[1].value, 2.0);
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let src = "\n\npie\n\n\"A\" : 1\n\n\"B\" : 2\n\n";
        let p = parse(src).unwrap();
        assert_eq!(p.slices.len(), 2);
    }

    #[test]
    fn parse_slice_quoted_label() {
        let src = "pie\n\"Hello World\" : 5";
        let p = parse(src).unwrap();
        assert_eq!(p.slices[0].label, "Hello World");
    }

    #[test]
    fn parse_slice_unquoted_label() {
        let src = "pie\nFoo : 5";
        let p = parse(src).unwrap();
        assert_eq!(p.slices[0].label, "Foo");
    }

    #[test]
    fn parse_slice_decimal_value() {
        let src = "pie\n\"A\" : 2.75";
        let p = parse(src).unwrap();
        assert!((p.slices[0].value - 2.75).abs() < 1e-9);
    }

    #[test]
    fn parse_slice_integer_value() {
        let src = "pie\n\"A\" : 42";
        let p = parse(src).unwrap();
        assert_eq!(p.slices[0].value, 42.0);
    }

    #[test]
    fn parse_multiple_slices() {
        let src = "pie\n\"A\" : 1\n\"B\" : 2\n\"C\" : 3\n\"D\" : 4";
        let p = parse(src).unwrap();
        assert_eq!(p.slices.len(), 4);
        assert_eq!(p.slices[0].label, "A");
        assert_eq!(p.slices[3].value, 4.0);
    }

    #[test]
    fn parse_empty_slices_returns_diagram_with_no_slices() {
        let src = "pie title Empty";
        let p = parse(src).unwrap();
        assert_eq!(p.title.as_deref(), Some("Empty"));
        assert!(p.slices.is_empty());
    }
}
