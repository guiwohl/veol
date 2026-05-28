use super::ast::{SankeyDiagram, SankeyFlow};
use crate::mermaid::ascii::error::AsciiRenderError;

pub fn parse(source: &str) -> Result<SankeyDiagram, AsciiRenderError> {
    let mut lines = source.lines().enumerate();
    let mut header_seen = false;

    for (idx, raw) in lines.by_ref() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if line.eq_ignore_ascii_case("sankey-beta") {
            header_seen = true;
            break;
        }
        return Err(AsciiRenderError::Parse {
            line: idx + 1,
            msg: format!("expected 'sankey-beta' header, got: {line}"),
        });
    }

    if !header_seen {
        return Err(AsciiRenderError::Parse {
            line: 1,
            msg: "missing 'sankey-beta' header".into(),
        });
    }

    let mut flows: Vec<SankeyFlow> = Vec::new();

    for (idx, raw) in lines {
        let line = strip_comment(raw);
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let fields =
            split_csv(trimmed).map_err(|msg| AsciiRenderError::Parse { line: idx + 1, msg })?;
        if fields.len() != 3 {
            return Err(AsciiRenderError::Parse {
                line: idx + 1,
                msg: format!("expected 3 CSV fields, got {}", fields.len()),
            });
        }
        let value: f64 = fields[2]
            .trim()
            .parse()
            .map_err(|_| AsciiRenderError::Parse {
                line: idx + 1,
                msg: format!("invalid numeric value: {}", fields[2]),
            })?;
        if value < 0.0 {
            return Err(AsciiRenderError::Parse {
                line: idx + 1,
                msg: format!("negative value not allowed: {value}"),
            });
        }
        flows.push(SankeyFlow {
            source: fields[0].trim().to_string(),
            target: fields[1].trim().to_string(),
            value,
        });
    }

    Ok(SankeyDiagram { flows })
}

fn strip_comment(line: &str) -> &str {
    if let Some(idx) = line.find("%%") {
        &line[..idx]
    } else {
        line
    }
}

fn split_csv(line: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut quote_ch = '"';
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == quote_ch {
                if chars.peek() == Some(&quote_ch) {
                    cur.push(quote_ch);
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                cur.push(c);
            }
        } else if c == '"' && cur.trim().is_empty() {
            cur.clear();
            in_quotes = true;
            quote_ch = '"';
        } else if c == ',' {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }

    if in_quotes {
        return Err("unterminated quoted field".into());
    }
    out.push(cur);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sankey_minimal() {
        let src = "sankey-beta\n\nA,B,1\n";
        let d = parse(src).unwrap();
        assert_eq!(d.flows.len(), 1);
        assert_eq!(d.flows[0].source, "A");
        assert_eq!(d.flows[0].target, "B");
        assert_eq!(d.flows[0].value, 1.0);
    }

    #[test]
    fn parse_no_header_error() {
        let src = "A,B,1\n";
        let err = parse(src).unwrap_err();
        match err {
            AsciiRenderError::Parse { .. } => {}
            other => panic!("expected Parse error, got {other:?}"),
        }
    }

    #[test]
    fn parse_csv_three_fields() {
        let src = "sankey-beta\nfoo,bar,42\n";
        let d = parse(src).unwrap();
        assert_eq!(d.flows[0].source, "foo");
        assert_eq!(d.flows[0].target, "bar");
        assert_eq!(d.flows[0].value, 42.0);
    }

    #[test]
    fn parse_quoted_source() {
        let src = "sankey-beta\n\"Bio, conv\",Liquid,0.5\n";
        let d = parse(src).unwrap();
        assert_eq!(d.flows[0].source, "Bio, conv");
        assert_eq!(d.flows[0].target, "Liquid");
    }

    #[test]
    fn parse_quoted_target() {
        let src = "sankey-beta\nA,\"Big, target\",10\n";
        let d = parse(src).unwrap();
        assert_eq!(d.flows[0].source, "A");
        assert_eq!(d.flows[0].target, "Big, target");
    }

    #[test]
    fn parse_decimal_value() {
        let src = "sankey-beta\nA,B,124.729\n";
        let d = parse(src).unwrap();
        assert!((d.flows[0].value - 124.729).abs() < 1e-9);
    }

    #[test]
    fn parse_negative_value_invalid() {
        let src = "sankey-beta\nA,B,-3\n";
        let err = parse(src).unwrap_err();
        match err {
            AsciiRenderError::Parse { msg, .. } => assert!(msg.contains("negative")),
            other => panic!("expected Parse error, got {other:?}"),
        }
    }

    #[test]
    fn parse_multiple_flows() {
        let src = "sankey-beta\nA,B,1\nA,C,2\nB,C,3\n";
        let d = parse(src).unwrap();
        assert_eq!(d.flows.len(), 3);
        assert_eq!(d.flows[2].value, 3.0);
    }

    #[test]
    fn parse_comments_stripped() {
        let src = "sankey-beta\n%% this is a comment\nA,B,1 %% trailing\n";
        let d = parse(src).unwrap();
        assert_eq!(d.flows.len(), 1);
        assert_eq!(d.flows[0].source, "A");
        assert_eq!(d.flows[0].target, "B");
        assert_eq!(d.flows[0].value, 1.0);
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let src = "sankey-beta\n\n\nA,B,1\n\n\nA,C,2\n\n";
        let d = parse(src).unwrap();
        assert_eq!(d.flows.len(), 2);
    }

    #[test]
    fn parse_uses_apostrophes_in_label() {
        let src = "sankey-beta\nAgricultural 'waste',Bio-conversion,124.729\n";
        let d = parse(src).unwrap();
        assert_eq!(d.flows[0].source, "Agricultural 'waste'");
        assert_eq!(d.flows[0].target, "Bio-conversion");
        assert!((d.flows[0].value - 124.729).abs() < 1e-9);
    }
}
