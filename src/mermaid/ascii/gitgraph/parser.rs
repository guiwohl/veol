use super::ast::{CommitKind, GitGraph, GitOp, Orientation};
use crate::mermaid::ascii::error::AsciiRenderError;

pub fn parse(source: &str) -> Result<GitGraph, AsciiRenderError> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    let lines: Vec<&str> = trimmed.lines().collect();

    let mut header_idx: Option<usize> = None;
    for (i, line) in lines.iter().enumerate() {
        let t = strip_comment(line).trim();
        if t.is_empty() {
            continue;
        }
        if is_gitgraph_header(t) {
            header_idx = Some(i);
            break;
        }
        return Err(AsciiRenderError::Parse {
            line: i + 1,
            msg: format!("expected 'gitGraph' header, got: {t:?}"),
        });
    }
    let header_idx = header_idx.ok_or(AsciiRenderError::Empty)?;

    let mut diag = GitGraph::default();
    let header = strip_comment(lines[header_idx]).trim();
    diag.orientation = parse_orientation(header);

    for (off, raw) in lines.iter().enumerate().skip(header_idx + 1) {
        let stripped = strip_comment(raw);
        let line = stripped.trim().trim_end_matches(';').trim();
        if line.is_empty() {
            continue;
        }
        let lineno = off + 1;
        let op = parse_op(line, lineno)?;
        diag.ops.push(op);
    }

    Ok(diag)
}

fn is_gitgraph_header(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    if !lower.starts_with("gitgraph") {
        return false;
    }
    let rest = &s[8..];
    rest.is_empty() || rest.starts_with(|c: char| c.is_whitespace() || c == ':' || c == '-')
}

fn parse_orientation(header: &str) -> Orientation {
    let after = &header[8..];
    let after = after.trim_start().trim_start_matches(':').trim();
    let first = after.split_whitespace().next().unwrap_or("");
    let first = first.trim_end_matches(':');
    match first.to_ascii_uppercase().as_str() {
        "TD" | "TB" => Orientation::TopDown,
        _ => Orientation::LeftRight,
    }
}

fn parse_op(line: &str, lineno: usize) -> Result<GitOp, AsciiRenderError> {
    let (verb, rest) = split_verb(line);
    match verb.to_ascii_lowercase().as_str() {
        "commit" => parse_commit(rest, lineno),
        "branch" => parse_branch(rest, lineno),
        "checkout" | "switch" => parse_checkout(rest, lineno),
        "merge" => parse_merge(rest, lineno),
        "cherry-pick" | "cherrypick" => parse_cherrypick(rest, lineno),
        _ => Err(AsciiRenderError::Parse {
            line: lineno,
            msg: format!("unknown gitGraph op: {verb:?}"),
        }),
    }
}

fn split_verb(line: &str) -> (&str, &str) {
    match line.find(|c: char| c.is_whitespace()) {
        Some(i) => (&line[..i], line[i..].trim_start()),
        None => (line, ""),
    }
}

fn parse_commit(rest: &str, lineno: usize) -> Result<GitOp, AsciiRenderError> {
    let attrs = parse_attrs(rest, lineno)?;
    let mut id = None;
    let mut kind = CommitKind::Normal;
    let mut tag = None;
    for (k, v) in attrs {
        match k.to_ascii_lowercase().as_str() {
            "id" => id = Some(v),
            "type" => {
                kind = match v.to_ascii_uppercase().as_str() {
                    "REVERSE" => CommitKind::Reverse,
                    "HIGHLIGHT" => CommitKind::Highlight,
                    _ => CommitKind::Normal,
                };
            }
            "tag" => tag = Some(v),
            _ => {}
        }
    }
    Ok(GitOp::Commit { id, kind, tag })
}

fn parse_branch(rest: &str, lineno: usize) -> Result<GitOp, AsciiRenderError> {
    let (name_tok, after) = split_verb(rest);
    if name_tok.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: "branch requires a name".into(),
        });
    }
    let name = unquote(name_tok).to_string();
    let attrs = parse_attrs(after, lineno)?;
    let mut from = None;
    for (k, v) in attrs {
        if k.eq_ignore_ascii_case("from") {
            from = Some(v);
        }
    }
    Ok(GitOp::Branch { name, from })
}

fn parse_checkout(rest: &str, lineno: usize) -> Result<GitOp, AsciiRenderError> {
    let (name_tok, _) = split_verb(rest);
    if name_tok.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: "checkout requires a branch name".into(),
        });
    }
    Ok(GitOp::Checkout {
        branch: unquote(name_tok).to_string(),
    })
}

fn parse_merge(rest: &str, lineno: usize) -> Result<GitOp, AsciiRenderError> {
    let (name_tok, after) = split_verb(rest);
    if name_tok.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: "merge requires a branch name".into(),
        });
    }
    let branch = unquote(name_tok).to_string();
    let attrs = parse_attrs(after, lineno)?;
    let mut id = None;
    let mut tag = None;
    for (k, v) in attrs {
        match k.to_ascii_lowercase().as_str() {
            "id" => id = Some(v),
            "tag" => tag = Some(v),
            _ => {}
        }
    }
    Ok(GitOp::Merge { branch, id, tag })
}

fn parse_cherrypick(rest: &str, lineno: usize) -> Result<GitOp, AsciiRenderError> {
    let attrs = parse_attrs(rest, lineno)?;
    let mut id = None;
    for (k, v) in attrs {
        if k.eq_ignore_ascii_case("id") {
            id = Some(v);
        }
    }
    let id = id.ok_or_else(|| AsciiRenderError::Parse {
        line: lineno,
        msg: "cherry-pick requires id".into(),
    })?;
    Ok(GitOp::CherryPick { id })
}

fn parse_attrs(s: &str, lineno: usize) -> Result<Vec<(String, String)>, AsciiRenderError> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(Vec::new());
    }
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        while i < chars.len() && (chars[i].is_whitespace() || chars[i] == ',') {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        let key_start = i;
        while i < chars.len() && chars[i] != ':' && !chars[i].is_whitespace() && chars[i] != ',' {
            i += 1;
        }
        let key: String = chars[key_start..i].iter().collect();
        if key.is_empty() {
            break;
        }
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() || chars[i] != ':' {
            return Err(AsciiRenderError::Parse {
                line: lineno,
                msg: format!("expected ':' after key {key:?}"),
            });
        }
        i += 1;
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let value = if i < chars.len() && chars[i] == '"' {
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != '"' {
                i += 1;
            }
            let v: String = chars[start..i].iter().collect();
            if i < chars.len() {
                i += 1;
            }
            v
        } else {
            let start = i;
            while i < chars.len() && !chars[i].is_whitespace() && chars[i] != ',' {
                i += 1;
            }
            chars[start..i].iter().collect()
        };
        out.push((key, value));
    }
    Ok(out)
}

fn unquote(s: &str) -> &str {
    let s = s.trim_end_matches(',');
    if let Some(rest) = s.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        rest
    } else {
        s
    }
}

fn strip_comment(line: &str) -> &str {
    if let Some(idx) = line.find("%%") {
        &line[..idx]
    } else {
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_gitgraph_minimal() {
        let src = "gitGraph";
        let g = parse(src).unwrap();
        assert_eq!(g.orientation, Orientation::LeftRight);
        assert!(g.ops.is_empty());
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
    fn parse_commit_no_args() {
        let src = "gitGraph\n    commit";
        let g = parse(src).unwrap();
        assert_eq!(g.ops.len(), 1);
        match &g.ops[0] {
            GitOp::Commit { id, kind, tag } => {
                assert!(id.is_none());
                assert_eq!(*kind, CommitKind::Normal);
                assert!(tag.is_none());
            }
            o => panic!("expected Commit, got {o:?}"),
        }
    }

    #[test]
    fn parse_commit_with_id() {
        let src = "gitGraph\n    commit id: \"abc\"";
        let g = parse(src).unwrap();
        match &g.ops[0] {
            GitOp::Commit { id, .. } => assert_eq!(id.as_deref(), Some("abc")),
            o => panic!("expected Commit, got {o:?}"),
        }
    }

    #[test]
    fn parse_commit_with_type_highlight() {
        let src = "gitGraph\n    commit type: HIGHLIGHT";
        let g = parse(src).unwrap();
        match &g.ops[0] {
            GitOp::Commit { kind, .. } => assert_eq!(*kind, CommitKind::Highlight),
            o => panic!("expected Commit, got {o:?}"),
        }
    }

    #[test]
    fn parse_commit_with_type_reverse() {
        let src = "gitGraph\n    commit type: REVERSE";
        let g = parse(src).unwrap();
        match &g.ops[0] {
            GitOp::Commit { kind, .. } => assert_eq!(*kind, CommitKind::Reverse),
            o => panic!("expected Commit, got {o:?}"),
        }
    }

    #[test]
    fn parse_commit_with_tag() {
        let src = "gitGraph\n    commit tag: \"v1\"";
        let g = parse(src).unwrap();
        match &g.ops[0] {
            GitOp::Commit { tag, .. } => assert_eq!(tag.as_deref(), Some("v1")),
            o => panic!("expected Commit, got {o:?}"),
        }
    }

    #[test]
    fn parse_commit_with_all_attrs() {
        let src = "gitGraph\n    commit id: \"x\" type: HIGHLIGHT tag: \"v1\"";
        let g = parse(src).unwrap();
        match &g.ops[0] {
            GitOp::Commit { id, kind, tag } => {
                assert_eq!(id.as_deref(), Some("x"));
                assert_eq!(*kind, CommitKind::Highlight);
                assert_eq!(tag.as_deref(), Some("v1"));
            }
            o => panic!("expected Commit, got {o:?}"),
        }
    }

    #[test]
    fn parse_branch() {
        let src = "gitGraph\n    branch feature";
        let g = parse(src).unwrap();
        match &g.ops[0] {
            GitOp::Branch { name, from } => {
                assert_eq!(name, "feature");
                assert!(from.is_none());
            }
            o => panic!("expected Branch, got {o:?}"),
        }
    }

    #[test]
    fn parse_branch_with_from() {
        let src = "gitGraph\n    branch hotfix from: main";
        let g = parse(src).unwrap();
        match &g.ops[0] {
            GitOp::Branch { name, from } => {
                assert_eq!(name, "hotfix");
                assert_eq!(from.as_deref(), Some("main"));
            }
            o => panic!("expected Branch, got {o:?}"),
        }
    }

    #[test]
    fn parse_checkout() {
        let src = "gitGraph\n    checkout main";
        let g = parse(src).unwrap();
        match &g.ops[0] {
            GitOp::Checkout { branch } => assert_eq!(branch, "main"),
            o => panic!("expected Checkout, got {o:?}"),
        }
    }

    #[test]
    fn parse_merge() {
        let src = "gitGraph\n    merge feature";
        let g = parse(src).unwrap();
        match &g.ops[0] {
            GitOp::Merge { branch, id, tag } => {
                assert_eq!(branch, "feature");
                assert!(id.is_none());
                assert!(tag.is_none());
            }
            o => panic!("expected Merge, got {o:?}"),
        }
    }

    #[test]
    fn parse_merge_with_id_and_tag() {
        let src = "gitGraph\n    merge feature id: \"m1\" tag: \"release\"";
        let g = parse(src).unwrap();
        match &g.ops[0] {
            GitOp::Merge { branch, id, tag } => {
                assert_eq!(branch, "feature");
                assert_eq!(id.as_deref(), Some("m1"));
                assert_eq!(tag.as_deref(), Some("release"));
            }
            o => panic!("expected Merge, got {o:?}"),
        }
    }

    #[test]
    fn parse_cherrypick() {
        let src = "gitGraph\n    cherry-pick id: \"abc123\"";
        let g = parse(src).unwrap();
        match &g.ops[0] {
            GitOp::CherryPick { id } => assert_eq!(id, "abc123"),
            o => panic!("expected CherryPick, got {o:?}"),
        }
    }

    #[test]
    fn parse_orientation_lr() {
        let src = "gitGraph LR:\n    commit";
        let g = parse(src).unwrap();
        assert_eq!(g.orientation, Orientation::LeftRight);
    }

    #[test]
    fn parse_orientation_td() {
        let src = "gitGraph TD:\n    commit";
        let g = parse(src).unwrap();
        assert_eq!(g.orientation, Orientation::TopDown);
    }

    #[test]
    fn parse_full_workflow() {
        let src = "gitGraph\n    commit\n    commit id: \"abc\"\n    branch feature\n    checkout feature\n    commit\n    commit id: \"x\" type: HIGHLIGHT tag: \"v1\"\n    checkout main\n    merge feature";
        let g = parse(src).unwrap();
        assert_eq!(g.ops.len(), 8);
        match &g.ops[2] {
            GitOp::Branch { name, .. } => assert_eq!(name, "feature"),
            o => panic!("expected Branch, got {o:?}"),
        }
        match &g.ops[7] {
            GitOp::Merge { branch, .. } => assert_eq!(branch, "feature"),
            o => panic!("expected Merge, got {o:?}"),
        }
    }

    #[test]
    fn parse_comments_stripped() {
        let src = "%% leading\ngitGraph\n%% inner\n    commit %% inline\n    commit id: \"y\"";
        let g = parse(src).unwrap();
        assert_eq!(g.ops.len(), 2);
        match &g.ops[1] {
            GitOp::Commit { id, .. } => assert_eq!(id.as_deref(), Some("y")),
            o => panic!("expected Commit, got {o:?}"),
        }
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let src = "\n\ngitGraph\n\n    commit\n\n    commit\n\n";
        let g = parse(src).unwrap();
        assert_eq!(g.ops.len(), 2);
    }
}
