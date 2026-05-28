use super::ast::{PacketDiagram, PacketField};
use crate::mermaid::ascii::error::AsciiRenderError;

const DEFAULT_BITS_PER_ROW: u32 = 32;

pub fn parse(source: &str) -> Result<PacketDiagram, AsciiRenderError> {
    let mut lines = source.lines().enumerate();
    let mut header_seen = false;

    for (idx, raw) in lines.by_ref() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if line.eq_ignore_ascii_case("packet-beta") {
            header_seen = true;
            break;
        }
        return Err(AsciiRenderError::Parse {
            line: idx + 1,
            msg: format!("expected 'packet-beta' header, got: {line}"),
        });
    }

    if !header_seen {
        return Err(AsciiRenderError::Parse {
            line: 1,
            msg: "missing 'packet-beta' header".into(),
        });
    }

    let mut title: Option<String> = None;
    let mut fields: Vec<PacketField> = Vec::new();

    for (idx, raw) in lines {
        let line = strip_comment(raw);
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = strip_prefix_ci(trimmed, "title") {
            let t = rest.trim();
            if t.is_empty() {
                return Err(AsciiRenderError::Parse {
                    line: idx + 1,
                    msg: "empty title".into(),
                });
            }
            title = Some(t.to_string());
            continue;
        }

        let field = parse_field_line(trimmed)
            .map_err(|msg| AsciiRenderError::Parse { line: idx + 1, msg })?;
        fields.push(field);
    }

    Ok(PacketDiagram {
        title,
        bits_per_row: DEFAULT_BITS_PER_ROW,
        fields,
    })
}

fn strip_comment(line: &str) -> &str {
    if let Some(idx) = line.find("%%") {
        &line[..idx]
    } else {
        line
    }
}

fn strip_prefix_ci<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    if line.len() < keyword.len() {
        return None;
    }
    let (head, rest) = line.split_at(keyword.len());
    if !head.eq_ignore_ascii_case(keyword) {
        return None;
    }
    let next = rest.chars().next();
    match next {
        None => Some(rest),
        Some(c) if c.is_whitespace() => Some(rest),
        _ => None,
    }
}

fn parse_field_line(line: &str) -> Result<PacketField, String> {
    let colon = line
        .find(':')
        .ok_or_else(|| format!("expected ':' in field line: {line}"))?;
    let range_part = line[..colon].trim();
    let label_part = line[colon + 1..].trim();

    if range_part.is_empty() {
        return Err("missing bit range before ':'".into());
    }
    if label_part.is_empty() {
        return Err("missing label after ':'".into());
    }

    let (start_bit, end_bit) = parse_bit_range(range_part)?;
    if end_bit < start_bit {
        return Err(format!("end bit {end_bit} less than start bit {start_bit}"));
    }
    let label = unquote(label_part);
    Ok(PacketField {
        start_bit,
        end_bit,
        label,
    })
}

fn parse_bit_range(s: &str) -> Result<(u32, u32), String> {
    if let Some(dash_idx) = s.find('-') {
        let start_str = s[..dash_idx].trim();
        let end_str = s[dash_idx + 1..].trim();
        let start: u32 = start_str
            .parse()
            .map_err(|_| format!("invalid start bit: {start_str}"))?;
        let end: u32 = end_str
            .parse()
            .map_err(|_| format!("invalid end bit: {end_str}"))?;
        Ok((start, end))
    } else {
        let bit: u32 = s.parse().map_err(|_| format!("invalid bit number: {s}"))?;
        Ok((bit, bit))
    }
}

fn unquote(s: &str) -> String {
    let bytes = s.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_packet_minimal() {
        let src = "packet-beta\n0-15: \"Source Port\"\n";
        let d = parse(src).unwrap();
        assert_eq!(d.bits_per_row, 32);
        assert_eq!(d.fields.len(), 1);
        assert_eq!(d.fields[0].start_bit, 0);
        assert_eq!(d.fields[0].end_bit, 15);
        assert_eq!(d.fields[0].label, "Source Port");
        assert!(d.title.is_none());
    }

    #[test]
    fn parse_no_header_error() {
        let src = "0-15: \"Source Port\"\n";
        let err = parse(src).unwrap_err();
        match err {
            AsciiRenderError::Parse { .. } => {}
            other => panic!("expected Parse error, got {other:?}"),
        }
    }

    #[test]
    fn parse_with_title() {
        let src = "packet-beta\n    title TCP Packet\n0-15: \"Source Port\"\n";
        let d = parse(src).unwrap();
        assert_eq!(d.title.as_deref(), Some("TCP Packet"));
        assert_eq!(d.fields.len(), 1);
    }

    #[test]
    fn parse_field_range() {
        let src = "packet-beta\n32-63: \"Sequence Number\"\n";
        let d = parse(src).unwrap();
        assert_eq!(d.fields[0].start_bit, 32);
        assert_eq!(d.fields[0].end_bit, 63);
        assert_eq!(d.fields[0].label, "Sequence Number");
    }

    #[test]
    fn parse_field_single_bit() {
        let src = "packet-beta\n106: \"URG\"\n";
        let d = parse(src).unwrap();
        assert_eq!(d.fields.len(), 1);
        assert_eq!(d.fields[0].start_bit, 106);
        assert_eq!(d.fields[0].end_bit, 106);
        assert_eq!(d.fields[0].label, "URG");
    }

    #[test]
    fn parse_field_quoted_label() {
        let src = "packet-beta\n0-7: \"hello world\"\n";
        let d = parse(src).unwrap();
        assert_eq!(d.fields[0].label, "hello world");
    }

    #[test]
    fn parse_multiple_fields() {
        let src = "packet-beta\n0-15: \"Source Port\"\n16-31: \"Destination Port\"\n32-63: \"Sequence Number\"\n";
        let d = parse(src).unwrap();
        assert_eq!(d.fields.len(), 3);
        assert_eq!(d.fields[0].label, "Source Port");
        assert_eq!(d.fields[1].label, "Destination Port");
        assert_eq!(d.fields[2].label, "Sequence Number");
        assert_eq!(d.fields[2].start_bit, 32);
        assert_eq!(d.fields[2].end_bit, 63);
    }

    #[test]
    fn parse_overlapping_fields_kept_as_is() {
        let src = "packet-beta\n0-15: \"A\"\n10-20: \"B\"\n";
        let d = parse(src).unwrap();
        assert_eq!(d.fields.len(), 2);
        assert_eq!(d.fields[0].start_bit, 0);
        assert_eq!(d.fields[0].end_bit, 15);
        assert_eq!(d.fields[1].start_bit, 10);
        assert_eq!(d.fields[1].end_bit, 20);
    }

    #[test]
    fn parse_comments_stripped() {
        let src = "packet-beta\n%% leading comment\n0-15: \"Source Port\" %% trailing\n";
        let d = parse(src).unwrap();
        assert_eq!(d.fields.len(), 1);
        assert_eq!(d.fields[0].label, "Source Port");
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let src = "packet-beta\n\n\n0-15: \"A\"\n\n\n16-31: \"B\"\n\n";
        let d = parse(src).unwrap();
        assert_eq!(d.fields.len(), 2);
        assert_eq!(d.fields[0].label, "A");
        assert_eq!(d.fields[1].label, "B");
    }
}
