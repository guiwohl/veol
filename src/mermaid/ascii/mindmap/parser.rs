use super::ast::{MindNode, Mindmap, NodeShape};
use crate::mermaid::ascii::error::AsciiRenderError;

pub fn parse(source: &str) -> Result<Mindmap, AsciiRenderError> {
    let trimmed = source.trim_matches('\n');
    if trimmed.trim().is_empty() {
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
        if t == "mindmap" || t.starts_with("mindmap ") {
            header_idx = Some(i);
            break;
        } else {
            return Err(AsciiRenderError::Parse {
                line: i + 1,
                msg: format!("expected 'mindmap' header, got: {t:?}"),
            });
        }
    }
    let header_idx = header_idx.ok_or(AsciiRenderError::Empty)?;

    let mut content: Vec<(usize, usize, String)> = Vec::new();
    for (off, raw) in raw_lines.iter().enumerate().skip(header_idx + 1) {
        let no_comment = strip_comment(raw);
        if no_comment.trim().is_empty() {
            continue;
        }
        let indent = leading_indent_cols(no_comment);
        let lineno = off + 1;
        content.push((lineno, indent, no_comment.trim().to_string()));
    }

    if content.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: header_idx + 1,
            msg: "mindmap requires a root node".into(),
        });
    }

    let root_indent = content[0].1;
    let root_node = parse_label(&content[0].2);

    let mut root = root_node;
    let mut stack: Vec<(usize, *mut MindNode)> = Vec::new();
    stack.push((root_indent, &mut root as *mut MindNode));

    for (_lineno, indent, label) in content.iter().skip(1) {
        let node = parse_label(label);
        while let Some(&(top_indent, _)) = stack.last() {
            if *indent > top_indent {
                break;
            }
            stack.pop();
        }
        let parent_ptr = match stack.last() {
            Some(&(_, p)) => p,
            None => {
                return Err(AsciiRenderError::Parse {
                    line: *_lineno,
                    msg: "node indent lower than root".into(),
                });
            }
        };
        let parent = unsafe { &mut *parent_ptr };
        parent.children.push(node);
        let last_child = parent.children.last_mut().unwrap() as *mut MindNode;
        stack.push((*indent, last_child));
    }

    Ok(Mindmap { root: Some(root) })
}

fn parse_label(raw: &str) -> MindNode {
    let s = raw.trim();
    let (label, shape) = detect_shape(s);
    let (label, icon) = extract_icon(&label);
    MindNode {
        label: clean_label(&label),
        shape,
        icon,
        children: Vec::new(),
    }
}

fn detect_shape(s: &str) -> (String, NodeShape) {
    if let Some(inner) = strip_pair(s, "((", "))") {
        return (inner, NodeShape::Circle);
    }
    if let Some(inner) = strip_pair(s, "))", "((") {
        return (inner, NodeShape::Bang);
    }
    if let Some(inner) = strip_pair(s, "{{", "}}") {
        return (inner, NodeShape::Hexagon);
    }
    if let Some(inner) = strip_pair(s, ")", "(") {
        return (inner, NodeShape::Cloud);
    }
    if let Some(inner) = strip_pair(s, "(", ")") {
        return (inner, NodeShape::Round);
    }
    if let Some(inner) = strip_pair(s, "[", "]") {
        return (inner, NodeShape::Square);
    }

    if let Some(idx) = s.find(|c: char| "([{)".contains(c)) {
        let (name, rest) = s.split_at(idx);
        let name = name.trim();
        if !name.is_empty() {
            let rest = rest.trim();
            if let Some(inner) = strip_pair(rest, "((", "))") {
                return (inner, NodeShape::Circle);
            }
            if let Some(inner) = strip_pair(rest, "))", "((") {
                return (inner, NodeShape::Bang);
            }
            if let Some(inner) = strip_pair(rest, "{{", "}}") {
                return (inner, NodeShape::Hexagon);
            }
            if let Some(inner) = strip_pair(rest, ")", "(") {
                return (inner, NodeShape::Cloud);
            }
            if let Some(inner) = strip_pair(rest, "(", ")") {
                return (inner, NodeShape::Round);
            }
            if let Some(inner) = strip_pair(rest, "[", "]") {
                return (inner, NodeShape::Square);
            }
        }
    }

    (s.to_string(), NodeShape::Default)
}

fn strip_pair(s: &str, open: &str, close: &str) -> Option<String> {
    let s = s.trim();
    let after = s.strip_prefix(open)?;
    let inner = after.strip_suffix(close)?;
    Some(inner.trim().to_string())
}

fn extract_icon(s: &str) -> (String, Option<String>) {
    let lower = s.to_ascii_lowercase();
    if let Some(idx) = lower.find("::icon(") {
        let before = &s[..idx];
        let after = &s[idx + "::icon(".len()..];
        if let Some(end) = after.find(')') {
            let icon = after[..end].trim().to_string();
            let rest = &after[end + 1..];
            let combined = format!("{}{}", before, rest);
            return (combined.trim().to_string(), Some(icon));
        }
    }
    (s.to_string(), None)
}

fn clean_label(s: &str) -> String {
    s.replace("<br/>", " ")
        .replace("<br>", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn leading_indent_cols(line: &str) -> usize {
    let mut cols = 0usize;
    for c in line.chars() {
        match c {
            ' ' => cols += 1,
            '\t' => cols += 4,
            _ => break,
        }
    }
    cols
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mindmap_minimal() {
        let src = "mindmap\n  root\n    child";
        let m = parse(src).unwrap();
        let r = m.root.unwrap();
        assert_eq!(r.label, "root");
        assert_eq!(r.shape, NodeShape::Default);
        assert_eq!(r.children.len(), 1);
        assert_eq!(r.children[0].label, "child");
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
    fn parse_root_with_double_paren_shape() {
        let src = "mindmap\n  root((my mindmap))";
        let m = parse(src).unwrap();
        let r = m.root.unwrap();
        assert_eq!(r.shape, NodeShape::Circle);
        assert!(r.label.contains("my mindmap") || r.label == "my mindmap");
    }

    #[test]
    fn parse_root_with_square_shape() {
        let src = "mindmap\n  root[Brainstorm]";
        let m = parse(src).unwrap();
        let r = m.root.unwrap();
        assert_eq!(r.shape, NodeShape::Square);
        assert_eq!(r.label, "Brainstorm");
    }

    #[test]
    fn parse_root_with_cloud_shape() {
        let src = "mindmap\n  root)A cloud(";
        let m = parse(src).unwrap();
        let r = m.root.unwrap();
        assert_eq!(r.shape, NodeShape::Cloud);
        assert_eq!(r.label, "A cloud");
    }

    #[test]
    fn parse_node_with_round_shape() {
        let src = "mindmap\n  root\n    Idea(idea text)";
        let m = parse(src).unwrap();
        let r = m.root.unwrap();
        assert_eq!(r.children.len(), 1);
        assert_eq!(r.children[0].shape, NodeShape::Round);
        assert_eq!(r.children[0].label, "idea text");
    }

    #[test]
    fn parse_node_with_bang_shape() {
        let src = "mindmap\n  root\n    alert))Watch out((";
        let m = parse(src).unwrap();
        let r = m.root.unwrap();
        assert_eq!(r.children[0].shape, NodeShape::Bang);
        assert_eq!(r.children[0].label, "Watch out");
    }

    #[test]
    fn parse_node_with_hexagon_shape() {
        let src = "mindmap\n  root\n    Hex{{value}}";
        let m = parse(src).unwrap();
        let r = m.root.unwrap();
        assert_eq!(r.children[0].shape, NodeShape::Hexagon);
        assert_eq!(r.children[0].label, "value");
    }

    #[test]
    fn parse_nesting_two_deep() {
        let src = "mindmap\n  root\n    Origins\n      Long history";
        let m = parse(src).unwrap();
        let r = m.root.unwrap();
        assert_eq!(r.children.len(), 1);
        assert_eq!(r.children[0].label, "Origins");
        assert_eq!(r.children[0].children.len(), 1);
        assert_eq!(r.children[0].children[0].label, "Long history");
    }

    #[test]
    fn parse_nesting_three_deep() {
        let src = "mindmap\n  root\n    A\n      B\n        C";
        let m = parse(src).unwrap();
        let r = m.root.unwrap();
        let a = &r.children[0];
        assert_eq!(a.label, "A");
        let b = &a.children[0];
        assert_eq!(b.label, "B");
        let c = &b.children[0];
        assert_eq!(c.label, "C");
    }

    #[test]
    fn parse_sibling_nodes_same_indent() {
        let src = "mindmap\n  root\n    A\n    B\n    C";
        let m = parse(src).unwrap();
        let r = m.root.unwrap();
        assert_eq!(r.children.len(), 3);
        assert_eq!(r.children[0].label, "A");
        assert_eq!(r.children[1].label, "B");
        assert_eq!(r.children[2].label, "C");
    }

    #[test]
    fn parse_uses_tabs_or_spaces() {
        let src = "mindmap\n\troot\n\t\tA\n\t\tB";
        let m = parse(src).unwrap();
        let r = m.root.unwrap();
        assert_eq!(r.label, "root");
        assert_eq!(r.children.len(), 2);
    }

    #[test]
    fn parse_comments_stripped() {
        let src = "%% top\nmindmap\n  root %% inline\n    A %% another\n  %% standalone";
        let m = parse(src).unwrap();
        let r = m.root.unwrap();
        assert_eq!(r.label, "root");
        assert_eq!(r.children.len(), 1);
        assert_eq!(r.children[0].label, "A");
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let src = "mindmap\n\n  root\n\n    A\n\n    B\n";
        let m = parse(src).unwrap();
        let r = m.root.unwrap();
        assert_eq!(r.children.len(), 2);
    }

    #[test]
    fn parse_no_root_error() {
        let src = "mindmap\n";
        let err = parse(src).unwrap_err();
        match err {
            AsciiRenderError::Parse { .. } => {}
            AsciiRenderError::Empty => {}
            e => panic!("expected Parse or Empty error, got {e:?}"),
        }
    }
}
