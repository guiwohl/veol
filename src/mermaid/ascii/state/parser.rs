use super::ast::{State, StateDiagram, StateDir, StateKind, Transition};
use crate::mermaid::ascii::error::AsciiRenderError;

pub fn parse(source: &str) -> Result<StateDiagram, AsciiRenderError> {
    let raw_lines: Vec<String> = strip_frontmatter(source)
        .lines()
        .map(|l| l.to_string())
        .collect();

    let mut lines: Vec<String> = Vec::new();
    for line in raw_lines {
        let trimmed = line.trim();
        if trimmed.starts_with("%%") {
            continue;
        }
        let cleaned = if let Some(idx) = line.find("%%") {
            line[..idx].trim_end().to_string()
        } else {
            line.clone()
        };
        if cleaned.trim().is_empty() {
            continue;
        }
        lines.push(cleaned);
    }

    if lines.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    let header = lines.remove(0);
    let header_trim = header.trim();
    if !(header_trim.eq_ignore_ascii_case("stateDiagram")
        || header_trim.eq_ignore_ascii_case("stateDiagram-v2"))
    {
        return Err(AsciiRenderError::Parse {
            line: 1,
            msg: format!("missing or invalid stateDiagram header: '{}'", header_trim),
        });
    }

    let mut diag = StateDiagram::default();
    let mut parent_stack: Vec<String> = Vec::new();
    let mut region_stack: Vec<Vec<String>> = Vec::new();

    parse_block(
        &lines,
        &mut 0usize,
        &mut diag,
        &mut parent_stack,
        &mut region_stack,
        2,
    )?;

    Ok(diag)
}

fn strip_frontmatter(src: &str) -> &str {
    let trimmed_start = src.trim_start_matches('\n');
    if !trimmed_start.starts_with("---") {
        return src;
    }
    let after_open = &trimmed_start[3..];
    if let Some(end_rel) = after_open.find("\n---") {
        let after_close = &after_open[end_rel + 4..];
        if let Some(nl) = after_close.find('\n') {
            return &after_close[nl + 1..];
        }
        return "";
    }
    src
}

fn parse_block(
    lines: &[String],
    cursor: &mut usize,
    diag: &mut StateDiagram,
    parent_stack: &mut Vec<String>,
    region_stack: &mut Vec<Vec<String>>,
    line_offset: usize,
) -> Result<(), AsciiRenderError> {
    while *cursor < lines.len() {
        let raw = &lines[*cursor];
        let line = raw.trim();
        *cursor += 1;

        if line.is_empty() {
            continue;
        }

        if line == "}" {
            return Ok(());
        }

        if line == "--" {
            if let Some(parent_id) = parent_stack.last().cloned() {
                if let Some(region) = region_stack.last_mut() {
                    let finished = std::mem::take(region);
                    if let Some(parent) = diag.states.get_mut(&parent_id) {
                        parent.concurrent_regions.push(finished);
                    }
                }
            }
            continue;
        }

        if let Some(rest) = strip_keyword(line, "direction") {
            let dir = rest.trim().to_ascii_uppercase();
            diag.direction = match dir.as_str() {
                "LR" | "RL" => StateDir::LeftRight,
                _ => StateDir::TopDown,
            };
            continue;
        }

        if let Some(rest) = strip_keyword(line, "note") {
            handle_note(rest, diag, line_offset + *cursor)?;
            continue;
        }

        if let Some(rest) = strip_keyword(line, "state") {
            handle_state_decl(rest, lines, cursor, diag, parent_stack, region_stack)?;
            continue;
        }

        if let Some(arrow_idx) = find_arrow(line) {
            handle_transition(line, arrow_idx, diag, parent_stack, region_stack);
            continue;
        }

        if let Some(colon_idx) = line.find(':') {
            let id = line[..colon_idx].trim().to_string();
            let desc = line[colon_idx + 1..].trim().to_string();
            if !id.is_empty() {
                let st = ensure_state(diag, &id, parent_stack, region_stack);
                if st.label.is_empty() {
                    st.label = id.clone();
                }
                st.description = Some(desc);
                continue;
            }
        }

        let id = line.to_string();
        ensure_state(diag, &id, parent_stack, region_stack);
    }
    Ok(())
}

fn handle_state_decl(
    rest: &str,
    lines: &[String],
    cursor: &mut usize,
    diag: &mut StateDiagram,
    parent_stack: &mut Vec<String>,
    region_stack: &mut Vec<Vec<String>>,
) -> Result<(), AsciiRenderError> {
    let rest = rest.trim();

    if let Some(annot_start) = rest.find("<<") {
        if let Some(annot_end_rel) = rest[annot_start..].find(">>") {
            let id_part = rest[..annot_start].trim();
            let annot = &rest[annot_start + 2..annot_start + annot_end_rel];
            let after = rest[annot_start + annot_end_rel + 2..].trim();
            if !id_part.is_empty() && after.is_empty() {
                let kind = match annot.trim() {
                    "choice" => StateKind::Choice,
                    "fork" => StateKind::Fork,
                    "join" => StateKind::Join,
                    _ => StateKind::Normal,
                };
                let st = ensure_state(diag, id_part, parent_stack, region_stack);
                st.kind = kind;
                if st.label.is_empty() {
                    st.label = id_part.to_string();
                }
                return Ok(());
            }
        }
    }

    let (id, label) = if let Some(stripped) = rest.strip_prefix('"') {
        if let Some(close) = stripped.find('"') {
            let label = stripped[..close].to_string();
            let after = stripped[close + 1..].trim();
            if let Some(after_as) = strip_keyword(after, "as") {
                let id_tail = after_as.trim().trim_end_matches('{').trim().to_string();
                (id_tail, label)
            } else {
                (label.clone(), label)
            }
        } else {
            return Err(AsciiRenderError::Parse {
                line: *cursor + 1,
                msg: format!("unclosed quote in state decl: '{}'", rest),
            });
        }
    } else {
        let head = rest.trim_end_matches('{').trim();
        (head.to_string(), head.to_string())
    };

    let has_open_brace = rest.ends_with('{');

    if id.is_empty() {
        return Ok(());
    }

    {
        let st = ensure_state(diag, &id, parent_stack, region_stack);
        if !label.is_empty() && (st.label.is_empty() || st.label == st.id) {
            st.label = label;
        }
    }

    if has_open_brace {
        if let Some(st) = diag.states.get_mut(&id) {
            st.kind = StateKind::Composite;
        }
        parent_stack.push(id);
        region_stack.push(Vec::new());
        parse_block(lines, cursor, diag, parent_stack, region_stack, 2)?;
        let popped = parent_stack.pop().unwrap();
        if let Some(mut last_region) = region_stack.pop() {
            if let Some(p) = diag.states.get_mut(&popped) {
                if !p.concurrent_regions.is_empty() && !last_region.is_empty() {
                    p.concurrent_regions.push(std::mem::take(&mut last_region));
                }
            }
        }
    }

    Ok(())
}

fn handle_note(
    rest: &str,
    diag: &mut StateDiagram,
    _line_no: usize,
) -> Result<(), AsciiRenderError> {
    let rest = rest.trim();
    let (_side, after_side) = if let Some(s) = strip_keyword(rest, "left") {
        ("left", s)
    } else if let Some(s) = strip_keyword(rest, "right") {
        ("right", s)
    } else {
        return Ok(());
    };

    let after_of = match strip_keyword(after_side, "of") {
        Some(s) => s,
        None => return Ok(()),
    };

    let (id, text) = if let Some(colon) = after_of.find(':') {
        (
            after_of[..colon].trim().to_string(),
            after_of[colon + 1..].trim().to_string(),
        )
    } else {
        (after_of.trim().to_string(), String::new())
    };

    if id.is_empty() {
        return Ok(());
    }

    let st = ensure_state(diag, &id, &mut Vec::new(), &mut Vec::new());
    if st.label.is_empty() {
        st.label = id.clone();
    }
    st.note = Some(text);
    Ok(())
}

fn handle_transition(
    line: &str,
    arrow_idx: usize,
    diag: &mut StateDiagram,
    parent_stack: &mut [String],
    region_stack: &mut [Vec<String>],
) {
    let from_raw = line[..arrow_idx].trim();
    let after = line[arrow_idx + 3..].trim();
    let (to_raw, label) = if let Some(colon) = after.find(':') {
        (
            after[..colon].trim().to_string(),
            after[colon + 1..].trim().to_string(),
        )
    } else {
        (after.to_string(), String::new())
    };

    let from_id = canonicalize_endpoint(from_raw, true, parent_stack, diag, region_stack);
    let to_id = canonicalize_endpoint(&to_raw, false, parent_stack, diag, region_stack);

    diag.transitions.push(Transition {
        from: from_id,
        to: to_id,
        label,
    });
}

fn canonicalize_endpoint(
    raw: &str,
    is_from: bool,
    parent_stack: &mut [String],
    diag: &mut StateDiagram,
    region_stack: &mut [Vec<String>],
) -> String {
    if raw == "[*]" {
        let parent_tag = parent_stack
            .last()
            .cloned()
            .unwrap_or_else(|| "__root".to_string());
        let id = if is_from {
            format!("__start_{}", parent_tag)
        } else {
            format!("__end_{}", parent_tag)
        };
        let exists = diag.states.contains_key(&id);
        if !exists {
            let kind = if is_from {
                StateKind::Start
            } else {
                StateKind::End
            };
            let parent = parent_stack.last().cloned();
            diag.states.insert(
                id.clone(),
                State {
                    id: id.clone(),
                    label: String::new(),
                    kind,
                    children: Vec::new(),
                    parent: parent.clone(),
                    description: None,
                    note: None,
                    concurrent_regions: Vec::new(),
                },
            );
            if let Some(p) = parent_stack.last() {
                let parent_id = p.clone();
                if let Some(parent_state) = diag.states.get_mut(&parent_id) {
                    if !parent_state.children.contains(&id) {
                        parent_state.children.push(id.clone());
                    }
                }
                if let Some(region) = region_stack.last_mut() {
                    region.push(id.clone());
                }
            }
        }
        return id;
    }
    let id = raw.to_string();
    ensure_state(diag, &id, parent_stack, region_stack);
    id
}

fn ensure_state<'a>(
    diag: &'a mut StateDiagram,
    id: &str,
    parent_stack: &mut [String],
    region_stack: &mut [Vec<String>],
) -> &'a mut State {
    let existed = diag.states.contains_key(id);
    if !existed {
        let parent = parent_stack.last().cloned();
        diag.states.insert(
            id.to_string(),
            State {
                id: id.to_string(),
                label: id.to_string(),
                kind: StateKind::Normal,
                children: Vec::new(),
                parent: parent.clone(),
                description: None,
                note: None,
                concurrent_regions: Vec::new(),
            },
        );
        if let Some(p) = parent_stack.last() {
            let parent_id = p.clone();
            if let Some(parent_state) = diag.states.get_mut(&parent_id) {
                if !parent_state.children.contains(&id.to_string()) {
                    parent_state.children.push(id.to_string());
                }
            }
            if let Some(region) = region_stack.last_mut() {
                region.push(id.to_string());
            }
        }
    }
    diag.states.get_mut(id).expect("just inserted")
}

fn strip_keyword<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    if let Some(rest) = line.strip_prefix(keyword) {
        if rest.is_empty() {
            return Some("");
        }
        let next = rest.chars().next().unwrap();
        if next.is_whitespace() {
            return Some(rest.trim_start());
        }
    }
    None
}

fn find_arrow(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i + 2 < bytes.len() {
        if bytes[i] == b'-' && bytes[i + 1] == b'-' && bytes[i + 2] == b'>' {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(src: &str) -> StateDiagram {
        parse(src).expect("expected ok parse")
    }

    #[test]
    fn parse_state_diagram_minimal() {
        let d = parse_ok("stateDiagram\n[*] --> A");
        assert!(d.states.values().any(|s| s.kind == StateKind::Start));
        assert!(d.states.contains_key("A"));
        assert_eq!(d.transitions.len(), 1);
    }

    #[test]
    fn parse_state_diagram_v2_minimal() {
        let d = parse_ok("stateDiagram-v2\n[*] --> A");
        assert!(d.states.contains_key("A"));
    }

    #[test]
    fn parse_no_header_error() {
        match parse("A --> B") {
            Err(AsciiRenderError::Parse { .. }) => {}
            other => panic!("expected Parse error, got {:?}", other),
        }
    }

    #[test]
    fn parse_comments_stripped() {
        let d = parse_ok("stateDiagram\n%% comment\nA --> B");
        assert_eq!(d.transitions.len(), 1);
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let d = parse_ok("stateDiagram\n\n\nA --> B\n\n");
        assert_eq!(d.transitions.len(), 1);
    }

    #[test]
    fn parse_state_declaration() {
        let d = parse_ok("stateDiagram\nstate Idle");
        assert!(d.states.contains_key("Idle"));
        assert_eq!(d.states["Idle"].kind, StateKind::Normal);
    }

    #[test]
    fn parse_state_with_description() {
        let d = parse_ok("stateDiagram\nIdle\nIdle : waiting for input");
        assert_eq!(
            d.states["Idle"].description.as_deref(),
            Some("waiting for input")
        );
    }

    #[test]
    fn parse_state_with_id_and_label_via_as() {
        let d = parse_ok("stateDiagram\nstate \"Long Label\" as L1");
        assert!(d.states.contains_key("L1"));
        assert_eq!(d.states["L1"].label, "Long Label");
    }

    #[test]
    fn parse_transition_basic() {
        let d = parse_ok("stateDiagram\nA --> B");
        assert_eq!(d.transitions.len(), 1);
        assert_eq!(d.transitions[0].from, "A");
        assert_eq!(d.transitions[0].to, "B");
        assert!(d.transitions[0].label.is_empty());
    }

    #[test]
    fn parse_transition_with_label() {
        let d = parse_ok("stateDiagram\nA --> B : go");
        assert_eq!(d.transitions[0].label, "go");
    }

    #[test]
    fn parse_start_state_brackets() {
        let d = parse_ok("stateDiagram\n[*] --> A");
        let start = d
            .states
            .values()
            .find(|s| s.kind == StateKind::Start)
            .expect("start");
        assert_eq!(d.transitions[0].from, start.id);
    }

    #[test]
    fn parse_end_state_brackets() {
        let d = parse_ok("stateDiagram\nA --> [*]");
        let end = d
            .states
            .values()
            .find(|s| s.kind == StateKind::End)
            .expect("end");
        assert_eq!(d.transitions[0].to, end.id);
    }

    #[test]
    fn parse_composite_state_with_substates() {
        let src = "stateDiagram\nstate Outer {\n  [*] --> Inner\n  Inner --> [*]\n}";
        let d = parse_ok(src);
        let outer = d.states.get("Outer").expect("Outer");
        assert_eq!(outer.kind, StateKind::Composite);
        assert!(outer.children.contains(&"Inner".to_string()));
        assert_eq!(d.states["Inner"].parent.as_deref(), Some("Outer"));
    }

    #[test]
    fn parse_choice_state_annotation() {
        let d = parse_ok("stateDiagram\nstate C <<choice>>");
        assert_eq!(d.states["C"].kind, StateKind::Choice);
    }

    #[test]
    fn parse_fork_state_annotation() {
        let d = parse_ok("stateDiagram\nstate F <<fork>>");
        assert_eq!(d.states["F"].kind, StateKind::Fork);
    }

    #[test]
    fn parse_join_state_annotation() {
        let d = parse_ok("stateDiagram\nstate J <<join>>");
        assert_eq!(d.states["J"].kind, StateKind::Join);
    }

    #[test]
    fn parse_concurrent_state_with_double_dash() {
        let src = "stateDiagram\nstate Both {\n  A\n  --\n  B\n}";
        let d = parse_ok(src);
        let outer = &d.states["Both"];
        assert_eq!(outer.kind, StateKind::Composite);
        assert!(!outer.concurrent_regions.is_empty());
        assert!(d.states.contains_key("A"));
        assert!(d.states.contains_key("B"));
    }

    #[test]
    fn parse_state_note_left() {
        let d = parse_ok("stateDiagram\nA\nnote left of A : hello");
        assert_eq!(d.states["A"].note.as_deref(), Some("hello"));
    }

    #[test]
    fn parse_state_note_right() {
        let d = parse_ok("stateDiagram\nA\nnote right of A : hi");
        assert_eq!(d.states["A"].note.as_deref(), Some("hi"));
    }

    #[test]
    fn parse_direction_lr() {
        let d = parse_ok("stateDiagram-v2\ndirection LR\nA --> B");
        assert_eq!(d.direction, StateDir::LeftRight);
    }

    #[test]
    fn parse_direction_td() {
        let d = parse_ok("stateDiagram-v2\ndirection TD\nA --> B");
        assert_eq!(d.direction, StateDir::TopDown);
    }
}
