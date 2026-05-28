use super::ast::{Attribute, Cardinality, Entity, ErDiagram, Relationship};
use crate::mermaid::ascii::error::AsciiRenderError;

pub fn parse(source: &str) -> Result<ErDiagram, AsciiRenderError> {
    let lines = preprocess(source);
    if lines.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: 0,
            msg: "empty source".into(),
        });
    }

    let mut idx = 0;
    let header = lines[idx].1.trim();
    if header != "erDiagram" {
        return Err(AsciiRenderError::Parse {
            line: lines[idx].0,
            msg: format!("expected 'erDiagram' header, found '{header}'"),
        });
    }
    idx += 1;

    let mut diag = ErDiagram::default();

    while idx < lines.len() {
        let (lineno, line) = &lines[idx];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            idx += 1;
            continue;
        }

        if trimmed.ends_with('{') {
            let name = trimmed.trim_end_matches('{').trim().to_string();
            if name.is_empty() {
                return Err(AsciiRenderError::Parse {
                    line: *lineno,
                    msg: "entity block missing name".into(),
                });
            }
            idx += 1;
            let mut attrs: Vec<Attribute> = Vec::new();
            let mut closed = false;
            while idx < lines.len() {
                let (in_lineno, in_line) = &lines[idx];
                let in_trim = in_line.trim();
                idx += 1;
                if in_trim.is_empty() {
                    continue;
                }
                if in_trim == "}" {
                    closed = true;
                    break;
                }
                let attr = parse_attribute(in_trim).map_err(|m| AsciiRenderError::Parse {
                    line: *in_lineno,
                    msg: m,
                })?;
                attrs.push(attr);
            }
            if !closed {
                return Err(AsciiRenderError::Parse {
                    line: *lineno,
                    msg: format!("unterminated entity block '{name}'"),
                });
            }
            let entry = diag.entities.entry(name.clone()).or_insert_with(|| Entity {
                name: name.clone(),
                attributes: Vec::new(),
            });
            entry.attributes = attrs;
            continue;
        }

        if let Some(rel) = try_parse_relationship(trimmed) {
            let rel = rel.map_err(|m| AsciiRenderError::Parse {
                line: *lineno,
                msg: m,
            })?;
            if !diag.entities.contains_key(&rel.left) {
                diag.entities.insert(
                    rel.left.clone(),
                    Entity {
                        name: rel.left.clone(),
                        attributes: Vec::new(),
                    },
                );
            }
            if !diag.entities.contains_key(&rel.right) {
                diag.entities.insert(
                    rel.right.clone(),
                    Entity {
                        name: rel.right.clone(),
                        attributes: Vec::new(),
                    },
                );
            }
            diag.relationships.push(rel);
            idx += 1;
            continue;
        }

        return Err(AsciiRenderError::Parse {
            line: *lineno,
            msg: format!("unrecognized line '{trimmed}'"),
        });
    }

    if diag.entities.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    Ok(diag)
}

fn preprocess(source: &str) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = Vec::new();
    let mut in_frontmatter = false;
    let mut frontmatter_seen = false;
    for (i, raw) in source.lines().enumerate() {
        let stripped = strip_comment(raw);
        let trimmed = stripped.trim();
        if !frontmatter_seen && trimmed == "---" {
            in_frontmatter = true;
            frontmatter_seen = true;
            continue;
        }
        if in_frontmatter {
            if trimmed == "---" {
                in_frontmatter = false;
            }
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        out.push((i + 1, stripped));
    }
    out
}

fn strip_comment(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut in_str: Option<char> = None;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if let Some(q) = in_str {
            if c == q {
                in_str = None;
            }
        } else if c == '"' {
            in_str = Some('"');
        } else if c == '%' && i + 1 < bytes.len() && bytes[i + 1] as char == '%' {
            return line[..i].to_string();
        }
        i += 1;
    }
    line.to_string()
}

fn parse_attribute(s: &str) -> Result<Attribute, String> {
    let mut comment: Option<String> = None;
    let mut rest = s.to_string();
    if let Some(start) = rest.find('"') {
        if let Some(end_rel) = rest[start + 1..].find('"') {
            let end = start + 1 + end_rel;
            comment = Some(rest[start + 1..end].to_string());
            let mut new_rest = String::with_capacity(rest.len());
            new_rest.push_str(&rest[..start]);
            new_rest.push_str(&rest[end + 1..]);
            rest = new_rest.trim().to_string();
        }
    }
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(format!(
            "attribute needs <type> <name> [PK|FK|UK] [\"comment\"], got '{s}'"
        ));
    }
    let type_name = parts[0].to_string();
    let name = parts[1].to_string();
    let mut key: Option<String> = None;
    if parts.len() >= 3 {
        let k = parts[2].to_uppercase();
        if matches!(k.as_str(), "PK" | "FK" | "UK") {
            key = Some(k);
        } else {
            return Err(format!("unknown attribute key marker '{}'", parts[2]));
        }
    }
    if parts.len() > 3 {
        return Err(format!("attribute has too many tokens: '{s}'"));
    }
    Ok(Attribute {
        type_name,
        name,
        key,
        comment,
    })
}

fn try_parse_relationship(s: &str) -> Option<Result<Relationship, String>> {
    let (body, label) = match s.find(':') {
        Some(p) => {
            let raw_label = s[p + 1..].trim();
            let lbl = strip_quotes(raw_label).to_string();
            (s[..p].trim().to_string(), lbl)
        }
        None => (s.to_string(), String::new()),
    };

    let tokens: Vec<&str> = body.split_whitespace().collect();
    if tokens.len() != 3 {
        return None;
    }
    let left_name = tokens[0].to_string();
    let connector = tokens[1];
    let right_name = tokens[2].to_string();

    let parsed = parse_connector(connector);
    parsed.map(|res| {
        res.map(|(lc, rc, ident)| Relationship {
            left: left_name,
            right: right_name,
            left_card: lc,
            right_card: rc,
            identifying: ident,
            label,
        })
    })
}

fn parse_connector(c: &str) -> Option<Result<(Cardinality, Cardinality, bool), String>> {
    let chars: Vec<char> = c.chars().collect();
    if chars.len() < 6 {
        return None;
    }
    let left = (chars[0], chars[1]);
    let line_a = chars[2];
    let line_b = chars[3];
    let right = (chars[chars.len() - 2], chars[chars.len() - 1]);
    let middle_len = chars.len() - 4;
    if middle_len < 2 {
        return None;
    }
    if line_a != line_b {
        return None;
    }
    let identifying = match line_a {
        '-' => true,
        '.' => false,
        _ => return None,
    };
    for &m in &chars[2..chars.len() - 2] {
        if m != line_a {
            return None;
        }
    }
    let left_card = match left {
        ('|', '|') => Cardinality::ExactlyOne,
        ('|', 'o') => Cardinality::ZeroOrOne,
        ('}', '|') => Cardinality::OneOrMany,
        ('}', 'o') => Cardinality::ZeroOrMany,
        _ => {
            return Some(Err(format!(
                "invalid left cardinality '{}{}'",
                left.0, left.1
            )))
        }
    };
    let right_card = match right {
        ('|', '|') => Cardinality::ExactlyOne,
        ('o', '|') => Cardinality::ZeroOrOne,
        ('|', '{') => Cardinality::OneOrMany,
        ('o', '{') => Cardinality::ZeroOrMany,
        _ => {
            return Some(Err(format!(
                "invalid right cardinality '{}{}'",
                right.0, right.1
            )))
        }
    };
    Some(Ok((left_card, right_card, identifying)))
}

fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_er_minimal() {
        let d = parse("erDiagram\nUSER ||--o{ POST : has").unwrap();
        assert!(d.entities.contains_key("USER"));
        assert!(d.entities.contains_key("POST"));
        assert_eq!(d.relationships.len(), 1);
    }

    #[test]
    fn parse_no_header_error() {
        let err = parse("USER ||--o{ POST : has").unwrap_err();
        assert!(matches!(err, AsciiRenderError::Parse { .. }));
    }

    #[test]
    fn parse_comments_stripped() {
        let d = parse("erDiagram\n%% comment here\nUSER ||--o{ POST : has").unwrap();
        assert_eq!(d.relationships.len(), 1);
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let d = parse("erDiagram\n\n\nUSER ||--o{ POST : has\n\n").unwrap();
        assert_eq!(d.relationships.len(), 1);
    }

    #[test]
    fn parse_relationship_one_to_many() {
        let d = parse("erDiagram\nA ||--o{ B : x").unwrap();
        let r = &d.relationships[0];
        assert_eq!(r.left_card, Cardinality::ExactlyOne);
        assert_eq!(r.right_card, Cardinality::ZeroOrMany);
        assert!(r.identifying);
    }

    #[test]
    fn parse_relationship_zero_to_one() {
        let d = parse("erDiagram\nA |o--o| B : x").unwrap();
        let r = &d.relationships[0];
        assert_eq!(r.left_card, Cardinality::ZeroOrOne);
        assert_eq!(r.right_card, Cardinality::ZeroOrOne);
    }

    #[test]
    fn parse_relationship_zero_to_many() {
        let d = parse("erDiagram\nA }o--o{ B : x").unwrap();
        let r = &d.relationships[0];
        assert_eq!(r.left_card, Cardinality::ZeroOrMany);
        assert_eq!(r.right_card, Cardinality::ZeroOrMany);
    }

    #[test]
    fn parse_relationship_one_to_one_identifying() {
        let d = parse("erDiagram\nA ||--|| B : x").unwrap();
        let r = &d.relationships[0];
        assert_eq!(r.left_card, Cardinality::ExactlyOne);
        assert_eq!(r.right_card, Cardinality::ExactlyOne);
        assert!(r.identifying);
    }

    #[test]
    fn parse_relationship_with_label() {
        let d = parse(
            r#"erDiagram
USER ||--o{ POST : "creates many""#,
        )
        .unwrap();
        assert_eq!(d.relationships[0].label, "creates many");
    }

    #[test]
    fn parse_entity_with_attributes_block() {
        let src = "erDiagram\nUSER {\n  int id\n  string name\n}";
        let d = parse(src).unwrap();
        let u = &d.entities["USER"];
        assert_eq!(u.attributes.len(), 2);
    }

    #[test]
    fn parse_attribute_with_type() {
        let src = "erDiagram\nUSER {\n  int id\n}";
        let d = parse(src).unwrap();
        assert_eq!(d.entities["USER"].attributes[0].type_name, "int");
        assert_eq!(d.entities["USER"].attributes[0].name, "id");
    }

    #[test]
    fn parse_attribute_with_pk_marker() {
        let src = "erDiagram\nUSER {\n  int id PK\n}";
        let d = parse(src).unwrap();
        assert_eq!(d.entities["USER"].attributes[0].key.as_deref(), Some("PK"));
    }

    #[test]
    fn parse_attribute_with_fk_marker() {
        let src = "erDiagram\nPOST {\n  int user_id FK\n}";
        let d = parse(src).unwrap();
        assert_eq!(d.entities["POST"].attributes[0].key.as_deref(), Some("FK"));
    }

    #[test]
    fn parse_attribute_with_uk_marker() {
        let src = "erDiagram\nUSER {\n  string email UK\n}";
        let d = parse(src).unwrap();
        assert_eq!(d.entities["USER"].attributes[0].key.as_deref(), Some("UK"));
    }

    #[test]
    fn parse_attribute_with_comment() {
        let src = "erDiagram\nUSER {\n  string name \"the display name\"\n}";
        let d = parse(src).unwrap();
        assert_eq!(
            d.entities["USER"].attributes[0].comment.as_deref(),
            Some("the display name")
        );
    }

    #[test]
    fn parse_multiple_entities_one_relationship() {
        let src = "erDiagram\nUSER {\n  int id PK\n}\nPOST {\n  int id PK\n  int user_id FK\n}\nUSER ||--o{ POST : has";
        let d = parse(src).unwrap();
        assert_eq!(d.entities.len(), 2);
        assert_eq!(d.relationships.len(), 1);
    }

    #[test]
    fn parse_entity_implicitly_via_relationship() {
        let d = parse("erDiagram\nA ||--o{ B : x").unwrap();
        assert!(d.entities.contains_key("A"));
        assert!(d.entities.contains_key("B"));
        assert!(d.entities["A"].attributes.is_empty());
    }
}
