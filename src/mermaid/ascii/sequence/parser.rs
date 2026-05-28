use super::ast::{
    ArrowKind, Block, BlockBranch, BlockKind, DiagramItem, Message, Note, NotePosition,
    Participant, SequenceDiagram,
};
use crate::mermaid::ascii::error::AsciiRenderError;

pub fn parse(source: &str) -> Result<SequenceDiagram, AsciiRenderError> {
    Parser::new(source).run()
}

struct Parser<'a> {
    source: &'a str,
}

struct ParseCtx {
    diag: SequenceDiagram,
    stack: Vec<Frame>,
}

struct Frame {
    kind: BlockKind,
    label: String,
    branches: Vec<BlockBranch>,
    current_branch_label: String,
    current_items: Vec<DiagramItem>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self { source }
    }

    fn run(self) -> Result<SequenceDiagram, AsciiRenderError> {
        let trimmed = self.source.trim();
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
            if t.starts_with("sequenceDiagram") {
                header_idx = Some(i);
                break;
            } else {
                return Err(AsciiRenderError::Parse {
                    line: i + 1,
                    msg: format!("expected 'sequenceDiagram', got: {t:?}"),
                });
            }
        }
        let header_idx = header_idx.ok_or(AsciiRenderError::Empty)?;

        let mut ctx = ParseCtx {
            diag: SequenceDiagram::default(),
            stack: Vec::new(),
        };

        for (offset, raw) in raw_lines.iter().enumerate().skip(header_idx + 1) {
            let stripped = strip_comment(raw);
            let line = stripped.trim();
            if line.is_empty() {
                continue;
            }
            let lineno = offset + 1;
            parse_line(line, lineno, &mut ctx)?;
        }

        if !ctx.stack.is_empty() {
            return Err(AsciiRenderError::Parse {
                line: raw_lines.len(),
                msg: "unclosed block (missing 'end')".into(),
            });
        }

        Ok(ctx.diag)
    }
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

fn parse_line(line: &str, lineno: usize, ctx: &mut ParseCtx) -> Result<(), AsciiRenderError> {
    if line == "autonumber" {
        ctx.diag.autonumber = !ctx.diag.autonumber;
        return Ok(());
    }

    if let Some(rest) = strip_keyword(line, "participant") {
        return parse_participant_decl(rest, lineno, ctx, false);
    }
    if let Some(rest) = strip_keyword(line, "actor") {
        return parse_participant_decl(rest, lineno, ctx, true);
    }
    if let Some(rest) = strip_keyword(line, "activate") {
        let id = rest.trim();
        let idx = get_or_create_participant(&mut ctx.diag, id);
        push_item(ctx, DiagramItem::Activate(idx));
        return Ok(());
    }
    if let Some(rest) = strip_keyword(line, "deactivate") {
        let id = rest.trim();
        let idx = get_or_create_participant(&mut ctx.diag, id);
        push_item(ctx, DiagramItem::Deactivate(idx));
        return Ok(());
    }
    if let Some(rest) = strip_keyword_ci(line, "note") {
        return parse_note(rest, lineno, ctx);
    }

    if let Some(rest) = strip_keyword(line, "loop") {
        open_block(ctx, BlockKind::Loop, rest.trim().to_string());
        return Ok(());
    }
    if let Some(rest) = strip_keyword(line, "alt") {
        open_block(ctx, BlockKind::Alt, rest.trim().to_string());
        return Ok(());
    }
    if let Some(rest) = strip_keyword(line, "opt") {
        open_block(ctx, BlockKind::Opt, rest.trim().to_string());
        return Ok(());
    }
    if let Some(rest) = strip_keyword(line, "par") {
        open_block(ctx, BlockKind::Par, rest.trim().to_string());
        return Ok(());
    }
    if let Some(rest) = strip_keyword(line, "critical") {
        open_block(ctx, BlockKind::Critical, rest.trim().to_string());
        return Ok(());
    }
    if let Some(rest) = strip_keyword(line, "break") {
        open_block(ctx, BlockKind::Break, rest.trim().to_string());
        return Ok(());
    }
    if let Some(rest) = strip_keyword(line, "else") {
        return new_branch(ctx, rest.trim().to_string(), lineno);
    }
    if line == "else" {
        return new_branch(ctx, String::new(), lineno);
    }
    if let Some(rest) = strip_keyword(line, "and") {
        return new_branch(ctx, rest.trim().to_string(), lineno);
    }
    if line == "end" {
        return close_block(ctx, lineno);
    }

    parse_message(line, lineno, ctx)
}

fn strip_keyword<'a>(line: &'a str, kw: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(kw)?;
    if rest.is_empty() {
        return Some(rest);
    }
    let first = rest.chars().next().unwrap();
    if first.is_whitespace() {
        Some(rest.trim_start())
    } else {
        None
    }
}

fn strip_keyword_ci<'a>(line: &'a str, kw: &str) -> Option<&'a str> {
    if line.len() < kw.len() {
        return None;
    }
    let (head, rest) = line.split_at(kw.len());
    if !head.eq_ignore_ascii_case(kw) {
        return None;
    }
    if rest.is_empty() {
        return Some(rest);
    }
    let first = rest.chars().next().unwrap();
    if first.is_whitespace() {
        Some(rest.trim_start())
    } else {
        None
    }
}

fn parse_participant_decl(
    rest: &str,
    lineno: usize,
    ctx: &mut ParseCtx,
    _is_actor: bool,
) -> Result<(), AsciiRenderError> {
    let rest = rest.trim();
    if rest.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: "participant requires an id".into(),
        });
    }
    let (id, after) = parse_token(rest);
    let label = if let Some(after_as) = strip_keyword(after.trim_start(), "as") {
        after_as.trim().trim_matches('"').to_string()
    } else {
        id.clone()
    };

    if ctx.diag.participants.iter().any(|p| p.id == id) {
        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: format!("duplicate participant: {id}"),
        });
    }

    let index = ctx.diag.participants.len();
    ctx.diag.participants.push(Participant {
        id: id.clone(),
        label,
        index,
    });
    Ok(())
}

fn parse_token(s: &str) -> (String, &str) {
    let s = s.trim_start();
    if let Some(rest) = s.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            let id = rest[..end].to_string();
            return (id, &rest[end + 1..]);
        }
    }
    let end = s.find(char::is_whitespace).unwrap_or(s.len());
    (s[..end].to_string(), &s[end..])
}

fn get_or_create_participant(diag: &mut SequenceDiagram, id: &str) -> usize {
    if let Some(p) = diag.participants.iter().find(|p| p.id == id) {
        return p.index;
    }
    let index = diag.participants.len();
    diag.participants.push(Participant {
        id: id.to_string(),
        label: id.to_string(),
        index,
    });
    index
}

fn parse_note(rest: &str, lineno: usize, ctx: &mut ParseCtx) -> Result<(), AsciiRenderError> {
    let (position, after) = if let Some(r) = strip_keyword_ci(rest, "left of") {
        (NotePosition::LeftOf, r)
    } else if let Some(r) = strip_keyword_ci(rest, "right of") {
        (NotePosition::RightOf, r)
    } else if let Some(r) = strip_keyword_ci(rest, "over") {
        (NotePosition::Over, r)
    } else {
        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: "expected 'left of', 'right of', or 'over' in note".into(),
        });
    };

    let colon = after.find(':').ok_or_else(|| AsciiRenderError::Parse {
        line: lineno,
        msg: "note missing ':text'".into(),
    })?;
    let ids_part = after[..colon].trim();
    let text = after[colon + 1..].trim().to_string();

    let participants: Vec<usize> = ids_part
        .split(',')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| get_or_create_participant(&mut ctx.diag, p))
        .collect();

    if participants.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: "note requires at least one participant".into(),
        });
    }

    push_item(
        ctx,
        DiagramItem::Note(Note {
            position,
            participants,
            text,
        }),
    );
    Ok(())
}

fn open_block(ctx: &mut ParseCtx, kind: BlockKind, label: String) {
    ctx.stack.push(Frame {
        kind,
        label: label.clone(),
        branches: Vec::new(),
        current_branch_label: label,
        current_items: Vec::new(),
    });
}

fn new_branch(ctx: &mut ParseCtx, label: String, lineno: usize) -> Result<(), AsciiRenderError> {
    let frame = ctx
        .stack
        .last_mut()
        .ok_or_else(|| AsciiRenderError::Parse {
            line: lineno,
            msg: "'else'/'and' outside of block".into(),
        })?;
    let prev_label = std::mem::take(&mut frame.current_branch_label);
    let prev_items = std::mem::take(&mut frame.current_items);
    frame.branches.push(BlockBranch {
        label: prev_label,
        items: prev_items,
    });
    frame.current_branch_label = label;
    Ok(())
}

fn close_block(ctx: &mut ParseCtx, lineno: usize) -> Result<(), AsciiRenderError> {
    let mut frame = ctx.stack.pop().ok_or_else(|| AsciiRenderError::Parse {
        line: lineno,
        msg: "unexpected 'end' (no open block)".into(),
    })?;
    let last_label = std::mem::take(&mut frame.current_branch_label);
    let last_items = std::mem::take(&mut frame.current_items);
    frame.branches.push(BlockBranch {
        label: last_label,
        items: last_items,
    });
    let block = Block {
        kind: frame.kind,
        label: frame.label,
        branches: frame.branches,
    };
    push_item(ctx, DiagramItem::Block(block));
    Ok(())
}

fn push_item(ctx: &mut ParseCtx, item: DiagramItem) {
    if let DiagramItem::Message(ref m) = item {
        ctx.diag.messages.push(m.clone());
    }
    if let Some(frame) = ctx.stack.last_mut() {
        frame.current_items.push(item);
    } else {
        ctx.diag.items.push(item);
    }
}

const ARROW_OPS: &[(&str, ArrowKind)] = &[
    ("--))", ArrowKind::AsyncDotted),
    ("--x", ArrowKind::AsyncDotted),
    ("-->>", ArrowKind::Dotted),
    ("--)", ArrowKind::AsyncDotted),
    ("-->", ArrowKind::Dotted),
    ("->>", ArrowKind::Solid),
    ("-))", ArrowKind::AsyncSolid),
    ("-x", ArrowKind::AsyncSolid),
    ("-)", ArrowKind::AsyncSolid),
    ("->", ArrowKind::Solid),
];

fn find_arrow(line: &str) -> Option<(usize, &'static str, ArrowKind)> {
    let mut best: Option<(usize, &'static str, ArrowKind)> = None;
    for (op, kind) in ARROW_OPS {
        if let Some(pos) = line.find(op) {
            match best {
                None => best = Some((pos, op, *kind)),
                Some((bp, bop, _)) => {
                    if pos < bp || (pos == bp && op.len() > bop.len()) {
                        best = Some((pos, op, *kind));
                    }
                }
            }
        }
    }
    best
}

fn parse_message(line: &str, lineno: usize, ctx: &mut ParseCtx) -> Result<(), AsciiRenderError> {
    let (apos, op, arrow) = find_arrow(line).ok_or_else(|| AsciiRenderError::Parse {
        line: lineno,
        msg: format!("invalid syntax: {line:?}"),
    })?;

    let from_part = line[..apos].trim();
    let after_arrow = &line[apos + op.len()..];

    let (to_part, label) = if let Some(colon) = after_arrow.find(':') {
        (after_arrow[..colon].trim(), after_arrow[colon + 1..].trim())
    } else {
        (after_arrow.trim(), "")
    };

    if from_part.is_empty() || to_part.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: format!("malformed message: {line:?}"),
        });
    }

    let (from_id, _) = parse_token(from_part);

    let (to_id_raw, _) = parse_token(to_part);
    let mut activate_after = false;
    let mut deactivate_before = false;
    let to_id = if let Some(stripped) = to_id_raw.strip_prefix('+') {
        activate_after = true;
        stripped.to_string()
    } else if let Some(stripped) = to_id_raw.strip_prefix('-') {
        deactivate_before = true;
        stripped.to_string()
    } else {
        to_id_raw
    };

    let from_idx = get_or_create_participant(&mut ctx.diag, &from_id);
    let to_idx = get_or_create_participant(&mut ctx.diag, &to_id);

    if deactivate_before {
        push_item(ctx, DiagramItem::Deactivate(to_idx));
    }

    push_item(
        ctx,
        DiagramItem::Message(Message {
            from: from_idx,
            to: to_idx,
            label: label.to_string(),
            arrow,
        }),
    );

    if activate_after {
        push_item(ctx, DiagramItem::Activate(to_idx));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diag(s: &str) -> SequenceDiagram {
        parse(s).expect("parse ok")
    }

    // ---------- Header & basics ----------

    #[test]
    fn parse_header_minimal() {
        let d = diag("sequenceDiagram\n");
        assert!(d.participants.is_empty());
        assert!(d.messages.is_empty());
        assert!(d.items.is_empty());
    }

    #[test]
    fn parse_no_header_error() {
        let err = parse("A->>B: hi").unwrap_err();
        assert!(matches!(err, AsciiRenderError::Parse { .. }));
    }

    #[test]
    fn parse_empty_error() {
        let err = parse("   \n\n").unwrap_err();
        assert!(matches!(err, AsciiRenderError::Empty));
    }

    #[test]
    fn parse_comments_stripped() {
        let d = diag("sequenceDiagram\n%% a comment\nA->>B: hi\n");
        assert_eq!(d.messages.len(), 1);
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let d = diag("sequenceDiagram\n\n\nA->>B: hi\n\n");
        assert_eq!(d.messages.len(), 1);
    }

    #[test]
    fn parse_autonumber_toggle() {
        let d = diag("sequenceDiagram\nautonumber\nA->>B: hi\n");
        assert!(d.autonumber);
    }

    // ---------- Participants ----------

    #[test]
    fn participant_implicit_via_message() {
        let d = diag("sequenceDiagram\nAlice->>Bob: hi\n");
        assert_eq!(d.participants.len(), 2);
        assert_eq!(d.participants[0].id, "Alice");
        assert_eq!(d.participants[1].id, "Bob");
    }

    #[test]
    fn participant_explicit_declaration() {
        let d = diag("sequenceDiagram\nparticipant Alice\nparticipant Bob\n");
        assert_eq!(d.participants.len(), 2);
        assert_eq!(d.participants[0].label, "Alice");
    }

    #[test]
    fn participant_with_label_via_as() {
        let d = diag("sequenceDiagram\nparticipant A as Alice\n");
        assert_eq!(d.participants[0].id, "A");
        assert_eq!(d.participants[0].label, "Alice");
    }

    #[test]
    fn participant_with_quoted_id() {
        let d = diag("sequenceDiagram\nparticipant \"John Doe\" as JD\n");
        assert_eq!(d.participants[0].id, "John Doe");
        assert_eq!(d.participants[0].label, "JD");
    }

    #[test]
    fn actor_alias_for_participant() {
        let d = diag("sequenceDiagram\nactor A as Alice\n");
        assert_eq!(d.participants[0].id, "A");
        assert_eq!(d.participants[0].label, "Alice");
    }

    #[test]
    fn duplicate_participant_error() {
        let err = parse("sequenceDiagram\nparticipant A\nparticipant A\n").unwrap_err();
        assert!(matches!(err, AsciiRenderError::Parse { .. }));
    }

    // ---------- Messages ----------

    #[test]
    fn msg_solid_arrow_double_angle() {
        let d = diag("sequenceDiagram\nA->>B: hi\n");
        assert_eq!(d.messages[0].arrow, ArrowKind::Solid);
    }

    #[test]
    fn msg_dotted_arrow_double_angle() {
        let d = diag("sequenceDiagram\nA-->>B: hi\n");
        assert_eq!(d.messages[0].arrow, ArrowKind::Dotted);
    }

    #[test]
    fn msg_async_solid_dash_x() {
        let d = diag("sequenceDiagram\nA-xB: hi\n");
        assert_eq!(d.messages[0].arrow, ArrowKind::AsyncSolid);
    }

    #[test]
    fn msg_async_dotted_dash_dash_x() {
        let d = diag("sequenceDiagram\nA--xB: hi\n");
        assert_eq!(d.messages[0].arrow, ArrowKind::AsyncDotted);
    }

    #[test]
    fn msg_async_paren_close() {
        let d = diag("sequenceDiagram\nA-)B: hi\n");
        assert_eq!(d.messages[0].arrow, ArrowKind::AsyncSolid);
    }

    #[test]
    fn msg_async_dotted_paren_close() {
        let d = diag("sequenceDiagram\nA--)B: hi\n");
        assert_eq!(d.messages[0].arrow, ArrowKind::AsyncDotted);
    }

    #[test]
    fn msg_with_label() {
        let d = diag("sequenceDiagram\nA->>B: hello world\n");
        assert_eq!(d.messages[0].label, "hello world");
    }

    #[test]
    fn msg_label_with_colon_in_label() {
        let d = diag("sequenceDiagram\nA->>B: time: 12:30\n");
        assert_eq!(d.messages[0].label, "time: 12:30");
    }

    #[test]
    fn msg_label_empty() {
        let d = diag("sequenceDiagram\nA->>B\n");
        assert_eq!(d.messages[0].label, "");
    }

    #[test]
    fn self_message_same_participant() {
        let d = diag("sequenceDiagram\nA->>A: self\n");
        assert_eq!(d.messages[0].from, d.messages[0].to);
    }

    #[test]
    fn autonumber_numbers_messages() {
        let d = diag("sequenceDiagram\nautonumber\nA->>B: a\nB->>A: b\n");
        assert!(d.autonumber);
        assert_eq!(d.messages.len(), 2);
    }

    // ---------- Notes ----------

    #[test]
    fn note_left_of() {
        let d = diag("sequenceDiagram\nA->>B: hi\nNote left of A: hello\n");
        let n = d
            .items
            .iter()
            .find_map(|i| match i {
                DiagramItem::Note(n) => Some(n),
                _ => None,
            })
            .unwrap();
        assert_eq!(n.position, NotePosition::LeftOf);
        assert_eq!(n.text, "hello");
        assert_eq!(n.participants.len(), 1);
    }

    #[test]
    fn note_right_of() {
        let d = diag("sequenceDiagram\nA->>B: hi\nNote right of B: bye\n");
        let n = d
            .items
            .iter()
            .find_map(|i| match i {
                DiagramItem::Note(n) => Some(n),
                _ => None,
            })
            .unwrap();
        assert_eq!(n.position, NotePosition::RightOf);
        assert_eq!(n.text, "bye");
    }

    #[test]
    fn note_over_one() {
        let d = diag("sequenceDiagram\nA->>B: hi\nNote over A: solo\n");
        let n = d
            .items
            .iter()
            .find_map(|i| match i {
                DiagramItem::Note(n) => Some(n),
                _ => None,
            })
            .unwrap();
        assert_eq!(n.position, NotePosition::Over);
        assert_eq!(n.participants.len(), 1);
    }

    #[test]
    fn note_over_two() {
        let d = diag("sequenceDiagram\nA->>B: hi\nNote over A,B: spans\n");
        let n = d
            .items
            .iter()
            .find_map(|i| match i {
                DiagramItem::Note(n) => Some(n),
                _ => None,
            })
            .unwrap();
        assert_eq!(n.position, NotePosition::Over);
        assert_eq!(n.participants.len(), 2);
        assert_eq!(n.text, "spans");
    }

    // ---------- Activations ----------

    #[test]
    fn activate_deactivate() {
        let d = diag("sequenceDiagram\nactivate A\ndeactivate A\n");
        let activates = d
            .items
            .iter()
            .filter(|i| matches!(i, DiagramItem::Activate(_)))
            .count();
        let deactivates = d
            .items
            .iter()
            .filter(|i| matches!(i, DiagramItem::Deactivate(_)))
            .count();
        assert_eq!(activates, 1);
        assert_eq!(deactivates, 1);
    }

    #[test]
    fn activate_via_plus_in_arrow() {
        let d = diag("sequenceDiagram\nA->>+B: hi\n");
        let kinds: Vec<&str> = d
            .items
            .iter()
            .map(|i| match i {
                DiagramItem::Message(_) => "msg",
                DiagramItem::Activate(_) => "act",
                DiagramItem::Deactivate(_) => "deact",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, vec!["msg", "act"]);
    }

    #[test]
    fn deactivate_via_minus_in_arrow() {
        let d = diag("sequenceDiagram\nA->>-B: bye\n");
        let kinds: Vec<&str> = d
            .items
            .iter()
            .map(|i| match i {
                DiagramItem::Message(_) => "msg",
                DiagramItem::Activate(_) => "act",
                DiagramItem::Deactivate(_) => "deact",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, vec!["deact", "msg"]);
    }

    // ---------- Structured blocks ----------

    fn first_block(d: &SequenceDiagram) -> &Block {
        d.items
            .iter()
            .find_map(|i| match i {
                DiagramItem::Block(b) => Some(b),
                _ => None,
            })
            .expect("a block")
    }

    #[test]
    fn block_loop() {
        let d = diag("sequenceDiagram\nloop every minute\nA->>B: tick\nend\n");
        let b = first_block(&d);
        assert_eq!(b.kind, BlockKind::Loop);
        assert_eq!(b.label, "every minute");
        assert_eq!(b.branches.len(), 1);
        assert_eq!(b.branches[0].items.len(), 1);
    }

    #[test]
    fn block_alt_with_else() {
        let d = diag("sequenceDiagram\nalt success\nA->>B: ok\nelse failure\nA->>B: err\nend\n");
        let b = first_block(&d);
        assert_eq!(b.kind, BlockKind::Alt);
        assert_eq!(b.branches.len(), 2);
        assert_eq!(b.branches[0].label, "success");
        assert_eq!(b.branches[1].label, "failure");
    }

    #[test]
    fn block_opt() {
        let d = diag("sequenceDiagram\nopt maybe\nA->>B: do\nend\n");
        let b = first_block(&d);
        assert_eq!(b.kind, BlockKind::Opt);
    }

    #[test]
    fn block_par_with_and() {
        let d = diag("sequenceDiagram\npar a\nA->>B: x\nand b\nB->>C: y\nend\n");
        let b = first_block(&d);
        assert_eq!(b.kind, BlockKind::Par);
        assert_eq!(b.branches.len(), 2);
        assert_eq!(b.branches[0].label, "a");
        assert_eq!(b.branches[1].label, "b");
    }

    #[test]
    fn block_critical() {
        let d = diag("sequenceDiagram\ncritical danger\nA->>B: care\nend\n");
        let b = first_block(&d);
        assert_eq!(b.kind, BlockKind::Critical);
    }

    #[test]
    fn block_break() {
        let d = diag("sequenceDiagram\nbreak abort\nA->>B: stop\nend\n");
        let b = first_block(&d);
        assert_eq!(b.kind, BlockKind::Break);
    }

    #[test]
    fn nested_blocks() {
        let d = diag("sequenceDiagram\nloop outer\nA->>B: a\nopt inner\nA->>B: b\nend\nend\n");
        let outer = first_block(&d);
        assert_eq!(outer.kind, BlockKind::Loop);
        assert_eq!(outer.branches.len(), 1);
        let inner = outer.branches[0]
            .items
            .iter()
            .find_map(|i| match i {
                DiagramItem::Block(b) => Some(b),
                _ => None,
            })
            .expect("inner block");
        assert_eq!(inner.kind, BlockKind::Opt);
    }
}
