use super::ast::{ArchEdge, ArchGroup, ArchJunction, ArchService, ArchitectureDiagram, Side};
use crate::mermaid::ascii::error::AsciiRenderError;

pub fn parse(source: &str) -> Result<ArchitectureDiagram, AsciiRenderError> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    let raw_lines: Vec<&str> = trimmed.lines().collect();
    let mut header_seen = false;
    let mut diag = ArchitectureDiagram::default();

    for (i, raw) in raw_lines.iter().enumerate() {
        let stripped = strip_comment(raw);
        let line = stripped.trim();
        if line.is_empty() {
            continue;
        }
        let lineno = i + 1;

        if !header_seen {
            if line.eq_ignore_ascii_case("architecture-beta") {
                header_seen = true;
                continue;
            } else {
                return Err(AsciiRenderError::Parse {
                    line: lineno,
                    msg: format!("expected 'architecture-beta' header, got: {line:?}"),
                });
            }
        }

        if let Some(rest) = strip_keyword(line, "group") {
            let (id, icon, label, parent) = parse_node_decl(rest, lineno, true)?;
            diag.decl_order.push(id.clone());
            diag.groups.insert(
                id.clone(),
                ArchGroup {
                    id,
                    label,
                    icon,
                    parent,
                },
            );
            continue;
        }

        if let Some(rest) = strip_keyword(line, "service") {
            let (id, icon, label, group) = parse_node_decl(rest, lineno, true)?;
            diag.decl_order.push(id.clone());
            diag.services.insert(
                id.clone(),
                ArchService {
                    id,
                    label,
                    icon,
                    group,
                },
            );
            continue;
        }

        if let Some(rest) = strip_keyword(line, "junction") {
            let (id, _icon, _label, group) = parse_node_decl(rest, lineno, true)?;
            diag.decl_order.push(id.clone());
            diag.junctions
                .insert(id.clone(), ArchJunction { id, group });
            continue;
        }

        let edge = parse_edge(line, lineno)?;
        diag.edges.push(edge);
    }

    if !header_seen {
        return Err(AsciiRenderError::Parse {
            line: 1,
            msg: "expected 'architecture-beta' header".into(),
        });
    }

    Ok(diag)
}

fn parse_node_decl(
    rest: &str,
    lineno: usize,
    allow_in: bool,
) -> Result<(String, Option<String>, String, Option<String>), AsciiRenderError> {
    let rest = rest.trim();
    if rest.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: "missing identifier".into(),
        });
    }

    let mut idx = 0;
    let bytes = rest.as_bytes();
    while idx < bytes.len() && is_ident_byte(bytes[idx]) {
        idx += 1;
    }
    if idx == 0 {
        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: format!("invalid identifier in: {rest:?}"),
        });
    }
    let id = rest[..idx].to_string();
    let mut cursor = idx;

    let icon = if cursor < bytes.len() && bytes[cursor] == b'(' {
        let close = rest[cursor + 1..]
            .find(')')
            .ok_or(AsciiRenderError::Parse {
                line: lineno,
                msg: "missing ')' in icon".into(),
            })?;
        let abs_close = cursor + 1 + close;
        let icon_str = rest[cursor + 1..abs_close].trim().to_string();
        cursor = abs_close + 1;
        if icon_str.is_empty() {
            None
        } else {
            Some(icon_str)
        }
    } else {
        None
    };

    let label_raw = if cursor < bytes.len() && bytes[cursor] == b'[' {
        let close = rest[cursor + 1..]
            .find(']')
            .ok_or(AsciiRenderError::Parse {
                line: lineno,
                msg: "missing ']' in label".into(),
            })?;
        let abs_close = cursor + 1 + close;
        let label_str = rest[cursor + 1..abs_close].to_string();
        cursor = abs_close + 1;
        Some(label_str)
    } else {
        None
    };

    let label = label_raw.unwrap_or_else(|| id.clone());

    let tail = rest[cursor..].trim();
    let parent = if !tail.is_empty() {
        if !allow_in {
            return Err(AsciiRenderError::Parse {
                line: lineno,
                msg: format!("unexpected trailing tokens: {tail:?}"),
            });
        }
        if let Some(after_in) = strip_keyword(tail, "in") {
            let pid = after_in.trim();
            if pid.is_empty() {
                return Err(AsciiRenderError::Parse {
                    line: lineno,
                    msg: "missing parent id after 'in'".into(),
                });
            }
            Some(pid.to_string())
        } else {
            return Err(AsciiRenderError::Parse {
                line: lineno,
                msg: format!("unexpected trailing tokens: {tail:?}"),
            });
        }
    } else {
        None
    };

    Ok((id, icon, label, parent))
}

fn parse_edge(line: &str, lineno: usize) -> Result<ArchEdge, AsciiRenderError> {
    let (left, right, label) = split_edge_with_label(line, lineno)?;

    let (from_id, from_side) = split_id_side(&left, lineno, true)?;
    let (to_id, to_side) = split_id_side(&right, lineno, false)?;

    Ok(ArchEdge {
        from: from_id,
        from_side,
        to: to_id,
        to_side,
        label,
    })
}

fn split_edge_with_label(
    line: &str,
    lineno: usize,
) -> Result<(String, String, String), AsciiRenderError> {
    let arrow_pos = line.find("--").ok_or(AsciiRenderError::Parse {
        line: lineno,
        msg: format!("expected '--' edge in: {line:?}"),
    })?;
    let left = line[..arrow_pos].trim().to_string();
    let after_arrow = &line[arrow_pos + 2..];

    let (after_arrow, label) = if let Some(rest) = after_arrow.trim_start().strip_prefix('[') {
        let close = rest.find(']').ok_or(AsciiRenderError::Parse {
            line: lineno,
            msg: "missing ']' in edge label".into(),
        })?;
        let lbl = rest[..close].to_string();
        (&rest[close + 1..], lbl)
    } else {
        (after_arrow, String::new())
    };

    let right = after_arrow.trim().to_string();

    Ok((left, right, label))
}

fn split_id_side(
    seg: &str,
    lineno: usize,
    id_first: bool,
) -> Result<(String, Side), AsciiRenderError> {
    let colon = seg.find(':').ok_or(AsciiRenderError::Parse {
        line: lineno,
        msg: format!("expected '<id>:<side>' or '<side>:<id>' in: {seg:?}"),
    })?;
    let (a, b) = (seg[..colon].trim(), seg[colon + 1..].trim());
    if a.is_empty() || b.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: format!("empty side or id in: {seg:?}"),
        });
    }
    if id_first {
        let side = parse_side(b, lineno)?;
        Ok((a.to_string(), side))
    } else {
        let side = parse_side(a, lineno)?;
        Ok((b.to_string(), side))
    }
}

fn parse_side(s: &str, lineno: usize) -> Result<Side, AsciiRenderError> {
    match s.trim() {
        "L" | "l" => Ok(Side::Left),
        "R" | "r" => Ok(Side::Right),
        "T" | "t" => Ok(Side::Top),
        "B" | "b" => Ok(Side::Bottom),
        other => Err(AsciiRenderError::Parse {
            line: lineno,
            msg: format!("expected one of L/R/T/B, got: {other:?}"),
        }),
    }
}

fn strip_comment(line: &str) -> &str {
    if let Some(idx) = line.find("%%") {
        &line[..idx]
    } else {
        line
    }
}

fn strip_keyword<'a>(line: &'a str, kw: &str) -> Option<&'a str> {
    let s = line.trim_start();
    let rest = s.strip_prefix(kw)?;
    match rest.chars().next() {
        None => Some(rest),
        Some(c) if c.is_whitespace() => Some(rest),
        _ => None,
    }
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_architecture_minimal() {
        let src = "architecture-beta\nservice db";
        let d = parse(src).unwrap();
        assert_eq!(d.services.len(), 1);
        assert!(d.services.contains_key("db"));
        assert!(d.groups.is_empty());
        assert!(d.edges.is_empty());
    }

    #[test]
    fn parse_no_header_error() {
        let src = "service db\ngroup api";
        let err = parse(src).unwrap_err();
        match err {
            AsciiRenderError::Parse { .. } => {}
            e => panic!("expected Parse error, got {e:?}"),
        }
    }

    #[test]
    fn parse_group_with_icon() {
        let src = "architecture-beta\ngroup api(cloud)";
        let d = parse(src).unwrap();
        let g = &d.groups["api"];
        assert_eq!(g.icon.as_deref(), Some("cloud"));
        assert_eq!(g.id, "api");
    }

    #[test]
    fn parse_group_with_label() {
        let src = "architecture-beta\ngroup api(cloud)[API Cluster]";
        let d = parse(src).unwrap();
        let g = &d.groups["api"];
        assert_eq!(g.label, "API Cluster");
        assert_eq!(g.icon.as_deref(), Some("cloud"));
    }

    #[test]
    fn parse_service_with_icon() {
        let src = "architecture-beta\nservice db(database)";
        let d = parse(src).unwrap();
        let s = &d.services["db"];
        assert_eq!(s.icon.as_deref(), Some("database"));
        assert!(s.group.is_none());
    }

    #[test]
    fn parse_service_in_group() {
        let src = "architecture-beta\ngroup api(cloud)[API]\nservice db(database)[DB] in api";
        let d = parse(src).unwrap();
        let s = &d.services["db"];
        assert_eq!(s.group.as_deref(), Some("api"));
        assert_eq!(s.label, "DB");
    }

    #[test]
    fn parse_nested_group() {
        let src =
            "architecture-beta\ngroup outer(cloud)[Outer]\ngroup inner(cloud)[Inner] in outer";
        let d = parse(src).unwrap();
        let inner = &d.groups["inner"];
        assert_eq!(inner.parent.as_deref(), Some("outer"));
    }

    #[test]
    fn parse_junction() {
        let src = "architecture-beta\njunction j1\njunction j2 in api";
        let d = parse(src).unwrap();
        assert_eq!(d.junctions.len(), 2);
        assert!(d.junctions["j1"].group.is_none());
        assert_eq!(d.junctions["j2"].group.as_deref(), Some("api"));
    }

    #[test]
    fn parse_edge_with_sides() {
        let src = "architecture-beta\nservice a\nservice b\na:R -- L:b";
        let d = parse(src).unwrap();
        assert_eq!(d.edges.len(), 1);
        let e = &d.edges[0];
        assert_eq!(e.from, "a");
        assert_eq!(e.to, "b");
        assert_eq!(e.from_side, Side::Right);
        assert_eq!(e.to_side, Side::Left);
        assert_eq!(e.label, "");
    }

    #[test]
    fn parse_edge_with_label() {
        let src = "architecture-beta\nservice a\nservice b\na:R --[talks to] L:b";
        let d = parse(src).unwrap();
        let e = &d.edges[0];
        assert_eq!(e.label, "talks to");
        assert_eq!(e.from, "a");
        assert_eq!(e.to, "b");
    }

    #[test]
    fn parse_arrow_directions_lrtb() {
        let src = "architecture-beta\nservice a\nservice b\nservice c\nservice d\na:L -- R:b\na:T -- B:c\nb:R -- L:d";
        let d = parse(src).unwrap();
        assert_eq!(d.edges.len(), 3);
        assert_eq!(d.edges[0].from_side, Side::Left);
        assert_eq!(d.edges[0].to_side, Side::Right);
        assert_eq!(d.edges[1].from_side, Side::Top);
        assert_eq!(d.edges[1].to_side, Side::Bottom);
        assert_eq!(d.edges[2].from_side, Side::Right);
        assert_eq!(d.edges[2].to_side, Side::Left);
    }

    #[test]
    fn parse_comments_stripped() {
        let src = "%% top comment\narchitecture-beta\n%% mid\nservice db %% trailing\n";
        let d = parse(src).unwrap();
        assert_eq!(d.services.len(), 1);
        assert!(d.services.contains_key("db"));
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let src = "\n\narchitecture-beta\n\nservice a\n\nservice b\n\n";
        let d = parse(src).unwrap();
        assert_eq!(d.services.len(), 2);
    }
}
