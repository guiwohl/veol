use super::ast::{Class, ClassDiagram, Member, Relation, RelationKind, Visibility};
use crate::mermaid::ascii::error::AsciiRenderError;

pub fn parse(source: &str) -> Result<ClassDiagram, AsciiRenderError> {
    let cleaned = preprocess(source);
    let mut lines = cleaned.iter();

    let header = lines.next().ok_or(AsciiRenderError::Empty)?;
    if !header.trim_start().starts_with("classDiagram") {
        return Err(AsciiRenderError::Parse {
            line: 1,
            msg: format!("missing classDiagram header: '{}'", header.trim()),
        });
    }

    let mut diag = ClassDiagram::default();
    let mut current_block: Option<String> = None;
    let mut line_no = 1usize;

    for raw in lines {
        line_no += 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(block_class) = current_block.as_ref().cloned() {
            if line == "}" {
                current_block = None;
                continue;
            }
            if let Some(ann) = parse_annotation_only(line) {
                if let Some(c) = diag.classes.get_mut(&block_class) {
                    c.annotation = Some(ann);
                }
                continue;
            }
            if let Some(member) = parse_member(line) {
                if let Some(c) = diag.classes.get_mut(&block_class) {
                    if member.is_method {
                        c.methods.push(member);
                    } else {
                        c.attributes.push(member);
                    }
                }
                continue;
            }
            return Err(AsciiRenderError::Parse {
                line: line_no,
                msg: format!("invalid member: '{}'", line),
            });
        }

        if let Some(ann_for) = parse_annotation_standalone(line) {
            let (ann, class_name) = ann_for;
            let entry = diag
                .classes
                .entry(class_name.clone())
                .or_insert_with(|| Class {
                    name: class_name,
                    ..Default::default()
                });
            entry.annotation = Some(ann);
            continue;
        }

        if let Some(rest) = strip_keyword(line, "class") {
            parse_class_decl(rest, &mut diag, &mut current_block)?;
            continue;
        }

        if let Some(rel) = try_parse_relation(line) {
            ensure_class(&mut diag, &rel.from);
            ensure_class(&mut diag, &rel.to);
            diag.relations.push(rel);
            continue;
        }

        if let Some((class_name, member_text)) = split_top_level_member(line) {
            ensure_class(&mut diag, &class_name);
            if let Some(ann) = parse_annotation_only(member_text) {
                if let Some(c) = diag.classes.get_mut(&class_name) {
                    c.annotation = Some(ann);
                }
                continue;
            }
            if let Some(member) = parse_member(member_text) {
                if let Some(c) = diag.classes.get_mut(&class_name) {
                    if member.is_method {
                        c.methods.push(member);
                    } else {
                        c.attributes.push(member);
                    }
                }
                continue;
            }
        }

        if is_bare_identifier(line) {
            ensure_class(&mut diag, line);
            continue;
        }
    }

    Ok(diag)
}

fn preprocess(source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in source.lines() {
        let stripped = if let Some(idx) = line.find("%%") {
            &line[..idx]
        } else {
            line
        };
        out.push(stripped.to_string());
    }
    if let Some(first_non_blank) = out.iter().position(|l| !l.trim().is_empty()) {
        if out[first_non_blank].trim() == "---" {
            if let Some(end) = out
                .iter()
                .enumerate()
                .skip(first_non_blank + 1)
                .find(|(_, l)| l.trim() == "---")
                .map(|(i, _)| i)
            {
                for line in out.iter_mut().take(end + 1).skip(first_non_blank) {
                    line.clear();
                }
            }
        }
    }
    let first = out
        .iter()
        .position(|l| !l.trim().is_empty())
        .unwrap_or(out.len());
    out.into_iter().skip(first).collect()
}

fn strip_keyword<'a>(line: &'a str, kw: &str) -> Option<&'a str> {
    let line = line.trim_start();
    if let Some(rest) = line.strip_prefix(kw) {
        if rest.is_empty() {
            return Some("");
        }
        let first = rest.chars().next().unwrap();
        if first.is_whitespace() || first == '{' {
            return Some(rest.trim_start());
        }
    }
    None
}

fn ensure_class(diag: &mut ClassDiagram, name: &str) {
    if !diag.classes.contains_key(name) {
        diag.classes.insert(
            name.to_string(),
            Class {
                name: name.to_string(),
                ..Default::default()
            },
        );
    }
}

fn parse_class_decl(
    rest: &str,
    diag: &mut ClassDiagram,
    current_block: &mut Option<String>,
) -> Result<(), AsciiRenderError> {
    let mut rest = rest.trim().to_string();
    let opens_block = if let Some(stripped) = rest.strip_suffix('{') {
        rest = stripped.trim_end().to_string();
        true
    } else {
        false
    };

    let (name, generic) = parse_name_generic(&rest);
    if name.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: 0,
            msg: "empty class name".into(),
        });
    }

    let entry = diag.classes.entry(name.clone()).or_insert_with(|| Class {
        name: name.clone(),
        ..Default::default()
    });
    if generic.is_some() {
        entry.generic = generic;
    }

    if opens_block {
        *current_block = Some(name);
    }
    Ok(())
}

fn parse_name_generic(s: &str) -> (String, Option<String>) {
    let s = s.trim();
    if let Some(start) = s.find('~') {
        if let Some(end_rel) = s[start + 1..].find('~') {
            let name = s[..start].trim().to_string();
            let generic = s[start + 1..start + 1 + end_rel].trim().to_string();
            return (name, Some(generic));
        }
    }
    (s.to_string(), None)
}

fn parse_annotation_only(line: &str) -> Option<String> {
    let line = line.trim();
    let inner = line.strip_prefix("<<")?.strip_suffix(">>")?;
    Some(inner.trim().to_string())
}

fn parse_annotation_standalone(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    let rest = line.strip_prefix("<<")?;
    let end = rest.find(">>")?;
    let ann = rest[..end].trim().to_string();
    let after = rest[end + 2..].trim();
    if after.is_empty() {
        return None;
    }
    Some((ann, after.to_string()))
}

fn split_top_level_member(line: &str) -> Option<(String, &str)> {
    let idx = line.find(" : ")?;
    let name = line[..idx].trim().to_string();
    if name.is_empty() || !is_bare_identifier(&name) {
        return None;
    }
    Some((name, line[idx + 3..].trim_start()))
}

fn is_bare_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '~')
}

fn parse_member(raw: &str) -> Option<Member> {
    let mut s = raw.trim().to_string();
    if s.is_empty() {
        return None;
    }
    let mut visibility = None;
    if let Some(first) = s.chars().next() {
        visibility = match first {
            '+' => Some(Visibility::Public),
            '-' => Some(Visibility::Private),
            '#' => Some(Visibility::Protected),
            '~' => Some(Visibility::Package),
            _ => None,
        };
        if visibility.is_some() {
            s = s[1..].trim_start().to_string();
        }
    }

    let has_paren = s.contains('(');
    if has_paren {
        let paren_open = s.find('(').unwrap();
        let head = s[..paren_open].trim().to_string();
        let after = &s[paren_open..];
        let close = after.find(')')?;
        let _args = &after[1..close];
        let tail = after[close + 1..].trim();

        let (name, is_static, is_abstract) = strip_member_suffixes(&head);
        let (is_static, is_abstract, mut return_hint) = (is_static, is_abstract, None);

        let tail_clean = tail.trim();
        let return_hint_str = if let Some(stripped) = tail_clean.strip_prefix(':') {
            Some(stripped.trim().to_string())
        } else if !tail_clean.is_empty() {
            Some(tail_clean.to_string())
        } else {
            None
        };
        let (return_hint_clean, ts2, ta2) = match return_hint_str {
            Some(r) => {
                let (clean, s2, a2) = strip_member_suffixes(&r);
                if clean.is_empty() {
                    (None, s2, a2)
                } else {
                    (Some(clean), s2, a2)
                }
            }
            None => (None, false, false),
        };
        if return_hint_clean.is_some() {
            return_hint = return_hint_clean;
        }
        Some(Member {
            visibility,
            name,
            type_hint: return_hint,
            is_method: true,
            is_static: is_static || ts2,
            is_abstract: is_abstract || ta2,
        })
    } else {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }
        let (type_hint, name_raw) = if parts.len() >= 2 {
            (Some(parts[0].to_string()), parts[1..].join(" "))
        } else {
            (None, parts[0].to_string())
        };
        let (name, is_static, is_abstract) = strip_member_suffixes(&name_raw);
        Some(Member {
            visibility,
            name,
            type_hint,
            is_method: false,
            is_static,
            is_abstract,
        })
    }
}

fn strip_member_suffixes(s: &str) -> (String, bool, bool) {
    let mut s = s.trim().to_string();
    let mut is_static = false;
    let mut is_abstract = false;
    loop {
        if let Some(rest) = s.strip_suffix('$') {
            is_static = true;
            s = rest.trim_end().to_string();
            continue;
        }
        if let Some(rest) = s.strip_suffix('*') {
            is_abstract = true;
            s = rest.trim_end().to_string();
            continue;
        }
        break;
    }
    (s, is_static, is_abstract)
}

const RELATION_OPS: &[(&str, RelationKind, bool, bool, bool)] = &[
    ("<|--", RelationKind::Inheritance, false, false, true),
    ("--|>", RelationKind::Inheritance, false, true, false),
    ("<|..", RelationKind::Realization, true, false, true),
    ("..|>", RelationKind::Realization, true, true, false),
    ("*--", RelationKind::Composition, false, false, true),
    ("--*", RelationKind::Composition, false, true, false),
    ("o--", RelationKind::Aggregation, false, false, true),
    ("--o", RelationKind::Aggregation, false, true, false),
    ("..>", RelationKind::Dependency, true, true, false),
    ("<..", RelationKind::Dependency, true, false, true),
    ("-->", RelationKind::Association, false, true, false),
    ("<--", RelationKind::Association, false, false, true),
    ("--", RelationKind::Link, false, false, false),
    ("..", RelationKind::Link, true, false, false),
];

fn try_parse_relation(line: &str) -> Option<Relation> {
    let (body, label) = if let Some(idx) = line.find(" : ") {
        (line[..idx].trim(), line[idx + 3..].trim().to_string())
    } else if let Some(idx) = line.find(':') {
        let head = line[..idx].trim();
        let possible_label = line[idx + 1..].trim();
        if head.chars().any(|c| !c.is_ascii_alphanumeric() && c != '_') {
            (head, possible_label.to_string())
        } else {
            (line, String::new())
        }
    } else {
        (line, String::new())
    };

    for (op, kind, dotted, _arrow_right, _arrow_left) in RELATION_OPS {
        if let Some(pos) = body.find(op) {
            let left = body[..pos].trim();
            let right = body[pos + op.len()..].trim();

            let (from, from_card) = split_endpoint(left, true);
            let (to, to_card) = split_endpoint(right, false);

            if from.is_empty() || to.is_empty() {
                return None;
            }
            return Some(Relation {
                from,
                to,
                kind: *kind,
                label,
                from_card,
                to_card,
                dotted: *dotted,
            });
        }
    }
    None
}

fn split_endpoint(part: &str, is_left: bool) -> (String, String) {
    let part = part.trim();
    if is_left {
        if let Some(end_quote) = part.rfind('"') {
            if let Some(start_quote) = part[..end_quote].rfind('"') {
                let card = part[start_quote + 1..end_quote].to_string();
                let name = part[..start_quote].trim().to_string();
                let after = part[end_quote + 1..].trim();
                if after.is_empty() {
                    return (name, card);
                }
            }
        }
        (part.to_string(), String::new())
    } else {
        if let Some(start_quote) = part.find('"') {
            if let Some(end_rel) = part[start_quote + 1..].find('"') {
                let card = part[start_quote + 1..start_quote + 1 + end_rel].to_string();
                let after = part[start_quote + 1 + end_rel + 1..].trim().to_string();
                let before = part[..start_quote].trim();
                if before.is_empty() {
                    return (after, card);
                }
            }
        }
        (part.to_string(), String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(src: &str) -> ClassDiagram {
        parse(src).expect("parse ok")
    }

    #[test]
    fn parse_class_diagram_minimal() {
        let d = p("classDiagram\nclass Foo");
        assert!(d.classes.contains_key("Foo"));
    }

    #[test]
    fn parse_no_header_returns_error() {
        let err = parse("class Foo\n").unwrap_err();
        matches!(err, AsciiRenderError::Parse { .. });
    }

    #[test]
    fn parse_comments_stripped() {
        let d = p("classDiagram\n%% a comment\nclass Foo %% inline\n");
        assert!(d.classes.contains_key("Foo"));
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let d = p("classDiagram\n\n\nclass Foo\n\n");
        assert!(d.classes.contains_key("Foo"));
    }

    #[test]
    fn parse_class_with_no_members() {
        let d = p("classDiagram\nclass Foo {\n}\n");
        let c = &d.classes["Foo"];
        assert!(c.attributes.is_empty());
        assert!(c.methods.is_empty());
    }

    #[test]
    fn parse_class_with_attributes() {
        let d = p("classDiagram\nclass Foo {\n  String name\n  int age\n}\n");
        let c = &d.classes["Foo"];
        assert_eq!(c.attributes.len(), 2);
        assert_eq!(c.attributes[0].name, "name");
        assert_eq!(c.attributes[0].type_hint.as_deref(), Some("String"));
        assert_eq!(c.attributes[1].name, "age");
    }

    #[test]
    fn parse_class_with_methods() {
        let d = p("classDiagram\nclass Foo {\n  greet()\n  shout() String\n}\n");
        let c = &d.classes["Foo"];
        assert_eq!(c.methods.len(), 2);
        assert!(c.methods[0].is_method);
        assert_eq!(c.methods[0].name, "greet");
        assert_eq!(c.methods[1].type_hint.as_deref(), Some("String"));
    }

    #[test]
    fn parse_class_with_mixed_members() {
        let d = p("classDiagram\nclass Foo {\n  String name\n  greet()\n}\n");
        let c = &d.classes["Foo"];
        assert_eq!(c.attributes.len(), 1);
        assert_eq!(c.methods.len(), 1);
    }

    #[test]
    fn parse_visibility_public_plus() {
        let d = p("classDiagram\nclass Foo {\n  +String name\n}\n");
        assert_eq!(
            d.classes["Foo"].attributes[0].visibility,
            Some(Visibility::Public)
        );
    }

    #[test]
    fn parse_visibility_private_minus() {
        let d = p("classDiagram\nclass Foo {\n  -int age\n}\n");
        assert_eq!(
            d.classes["Foo"].attributes[0].visibility,
            Some(Visibility::Private)
        );
    }

    #[test]
    fn parse_visibility_protected_hash() {
        let d = p("classDiagram\nclass Foo {\n  #String pwd\n}\n");
        assert_eq!(
            d.classes["Foo"].attributes[0].visibility,
            Some(Visibility::Protected)
        );
    }

    #[test]
    fn parse_visibility_package_tilde() {
        let d = p("classDiagram\nclass Foo {\n  ~bool flag\n}\n");
        assert_eq!(
            d.classes["Foo"].attributes[0].visibility,
            Some(Visibility::Package)
        );
    }

    #[test]
    fn parse_generic_class_with_tilde() {
        let d = p("classDiagram\nclass List~T~");
        let c = &d.classes["List"];
        assert_eq!(c.generic.as_deref(), Some("T"));
    }

    #[test]
    fn parse_annotation_interface() {
        let d = p("classDiagram\nclass Foo {\n  <<interface>>\n}\n");
        assert_eq!(d.classes["Foo"].annotation.as_deref(), Some("interface"));
    }

    #[test]
    fn parse_annotation_abstract() {
        let d = p("classDiagram\n<<abstract>> Foo");
        assert_eq!(d.classes["Foo"].annotation.as_deref(), Some("abstract"));
    }

    #[test]
    fn parse_member_with_type_hint() {
        let d = p("classDiagram\nclass Foo {\n  String name\n}\n");
        assert_eq!(
            d.classes["Foo"].attributes[0].type_hint.as_deref(),
            Some("String")
        );
    }

    #[test]
    fn parse_static_member_dollar() {
        let d = p("classDiagram\nclass Foo {\n  +int count$\n}\n");
        let m = &d.classes["Foo"].attributes[0];
        assert!(m.is_static);
        assert_eq!(m.name, "count");
    }

    #[test]
    fn parse_abstract_method_star() {
        let d = p("classDiagram\nclass Foo {\n  +draw()*\n}\n");
        let m = &d.classes["Foo"].methods[0];
        assert!(m.is_abstract);
    }

    #[test]
    fn parse_class_outside_block_inline_member() {
        let d = p("classDiagram\nFoo : +String name\nFoo : +greet()");
        let c = &d.classes["Foo"];
        assert_eq!(c.attributes.len(), 1);
        assert_eq!(c.methods.len(), 1);
    }

    #[test]
    fn parse_multiple_classes() {
        let d = p("classDiagram\nclass Foo\nclass Bar");
        assert!(d.classes.contains_key("Foo"));
        assert!(d.classes.contains_key("Bar"));
    }

    #[test]
    fn parse_relation_inheritance() {
        let d = p("classDiagram\nFoo <|-- Bar");
        let r = &d.relations[0];
        assert_eq!(r.kind, RelationKind::Inheritance);
        assert_eq!(r.from, "Foo");
        assert_eq!(r.to, "Bar");
    }

    #[test]
    fn parse_relation_composition() {
        let d = p("classDiagram\nFoo *-- Bar");
        assert_eq!(d.relations[0].kind, RelationKind::Composition);
    }

    #[test]
    fn parse_relation_aggregation() {
        let d = p("classDiagram\nFoo o-- Bar");
        assert_eq!(d.relations[0].kind, RelationKind::Aggregation);
    }

    #[test]
    fn parse_relation_association_arrow() {
        let d = p("classDiagram\nFoo --> Bar");
        assert_eq!(d.relations[0].kind, RelationKind::Association);
    }

    #[test]
    fn parse_relation_dependency_dotted() {
        let d = p("classDiagram\nFoo ..> Bar");
        let r = &d.relations[0];
        assert_eq!(r.kind, RelationKind::Dependency);
        assert!(r.dotted);
    }

    #[test]
    fn parse_relation_realization_dotted_inheritance() {
        let d = p("classDiagram\nFoo <|.. Bar");
        let r = &d.relations[0];
        assert_eq!(r.kind, RelationKind::Realization);
        assert!(r.dotted);
    }

    #[test]
    fn parse_relation_link_plain() {
        let d = p("classDiagram\nFoo -- Bar");
        assert_eq!(d.relations[0].kind, RelationKind::Link);
    }

    #[test]
    fn parse_relation_with_label() {
        let d = p("classDiagram\nFoo --> Bar : owns");
        let r = &d.relations[0];
        assert_eq!(r.label, "owns");
    }

    #[test]
    fn parse_relation_with_cardinality_left_right() {
        let d = p("classDiagram\nFoo \"1\" --> \"many\" Bar : has");
        let r = &d.relations[0];
        assert_eq!(r.from, "Foo");
        assert_eq!(r.to, "Bar");
        assert_eq!(r.from_card, "1");
        assert_eq!(r.to_card, "many");
        assert_eq!(r.label, "has");
    }

    #[test]
    fn parse_relation_dotted_variant() {
        let d = p("classDiagram\nFoo .. Bar");
        let r = &d.relations[0];
        assert_eq!(r.kind, RelationKind::Link);
        assert!(r.dotted);
    }
}
