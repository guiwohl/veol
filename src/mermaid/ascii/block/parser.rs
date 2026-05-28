use super::ast::{Block, BlockDiagram, BlockEdge, BlockShape};
use crate::mermaid::ascii::error::AsciiRenderError;

pub fn parse(source: &str) -> Result<BlockDiagram, AsciiRenderError> {
    let cleaned = preprocess(source);

    if cleaned.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    let header = cleaned[0].trim();
    if !is_header(header) {
        return Err(AsciiRenderError::Parse {
            line: 1,
            msg: format!("missing or invalid block-beta header: '{}'", header),
        });
    }

    let mut diag = BlockDiagram::default();
    let mut idx = 1usize;

    if idx < cleaned.len() {
        let first = cleaned[idx].trim();
        if let Some(rest) = strip_keyword(first, "columns") {
            let n_str = rest.trim();
            let n: u32 = n_str.parse().map_err(|_| AsciiRenderError::Parse {
                line: idx + 1,
                msg: format!("invalid columns value: '{}'", n_str),
            })?;
            diag.columns = n;
            idx += 1;
        }
    }

    while idx < cleaned.len() {
        let line = cleaned[idx].trim();
        if line.is_empty() {
            idx += 1;
            continue;
        }

        if let Some(edge) = try_parse_edge(line) {
            diag.edges.push(edge);
            idx += 1;
            continue;
        }

        let row_blocks = parse_row(line, idx + 1)?;
        for b in row_blocks {
            diag.blocks.push(b);
        }
        idx += 1;
    }

    if diag.blocks.is_empty() && diag.edges.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    Ok(diag)
}

fn preprocess(source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in source.lines() {
        let trimmed = raw.trim();
        if trimmed.starts_with("%%") {
            continue;
        }
        let cleaned = if let Some(idx) = find_comment_start(raw) {
            raw[..idx].trim_end().to_string()
        } else {
            raw.to_string()
        };
        if cleaned.trim().is_empty() {
            continue;
        }
        out.push(cleaned);
    }
    out
}

fn find_comment_start(line: &str) -> Option<usize> {
    let bytes: Vec<char> = line.chars().collect();
    let mut in_quotes = false;
    let mut i = 0;
    let mut byte_idx = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == '"' {
            in_quotes = !in_quotes;
        } else if !in_quotes && c == '%' && i + 1 < bytes.len() && bytes[i + 1] == '%' {
            return Some(byte_idx);
        }
        byte_idx += c.len_utf8();
        i += 1;
    }
    None
}

fn is_header(line: &str) -> bool {
    let t = line.trim();
    t == "block-beta" || t.starts_with("block-beta ") || t.starts_with("block-beta\t")
}

fn strip_keyword<'a>(line: &'a str, kw: &str) -> Option<&'a str> {
    if line.len() < kw.len() {
        return None;
    }
    if !line[..kw.len()].eq_ignore_ascii_case(kw) {
        return None;
    }
    let rest = &line[kw.len()..];
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

fn try_parse_edge(line: &str) -> Option<BlockEdge> {
    let arrow_idx = find_arrow(line)?;
    let (left, rest) = line.split_at(arrow_idx);
    let from = left.trim().to_string();
    if from.is_empty() || !is_simple_id(&from) {
        return None;
    }
    let after_arrow = &rest[3..];
    let (label, to_part) = if let Some(rest_pipe) = after_arrow.strip_prefix('|') {
        let end = rest_pipe.find('|')?;
        let lbl = rest_pipe[..end].trim().to_string();
        (lbl, rest_pipe[end + 1..].trim().to_string())
    } else {
        (String::new(), after_arrow.trim().to_string())
    };
    if !is_simple_id(&to_part) {
        return None;
    }
    Some(BlockEdge {
        from,
        to: to_part,
        label,
        arrow: true,
    })
}

fn find_arrow(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = 0;
    let mut in_quotes = false;
    let mut paren_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;
    let mut brace_depth: i32 = 0;
    while i + 2 < bytes.len() {
        let c = bytes[i] as char;
        if c == '"' {
            in_quotes = !in_quotes;
        }
        if !in_quotes {
            match c {
                '(' => paren_depth += 1,
                ')' => paren_depth -= 1,
                '[' => bracket_depth += 1,
                ']' => bracket_depth -= 1,
                '{' => brace_depth += 1,
                '}' => brace_depth -= 1,
                _ => {}
            }
            if paren_depth == 0
                && bracket_depth == 0
                && brace_depth == 0
                && bytes[i] == b'-'
                && bytes[i + 1] == b'-'
                && bytes[i + 2] == b'>'
            {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn is_simple_id(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn parse_row(line: &str, line_no: usize) -> Result<Vec<Block>, AsciiRenderError> {
    let mut blocks: Vec<Block> = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        let id_start = i;
        while i < chars.len()
            && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '-')
        {
            i += 1;
        }
        if i == id_start {
            return Err(AsciiRenderError::Parse {
                line: line_no,
                msg: format!("expected block id at position {}: '{}'", id_start, line),
            });
        }
        let id: String = chars[id_start..i].iter().collect();

        let (shape, label, consumed) = if i < chars.len() {
            parse_shape_label(&chars, i)?
        } else {
            (BlockShape::Square, String::new(), 0)
        };
        i += consumed;

        blocks.push(Block {
            id: id.clone(),
            label: if label.is_empty() { id } else { label },
            shape,
            width: 0,
            height: 0,
        });
    }

    if blocks.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: line_no,
            msg: format!("no blocks parsed from row: '{}'", line),
        });
    }

    Ok(blocks)
}

fn parse_shape_label(
    chars: &[char],
    start: usize,
) -> Result<(BlockShape, String, usize), AsciiRenderError> {
    if start >= chars.len() {
        return Ok((BlockShape::Square, String::new(), 0));
    }
    let c0 = chars[start];
    let c1 = if start + 1 < chars.len() {
        chars[start + 1]
    } else {
        '\0'
    };

    match (c0, c1) {
        ('(', '(') => {
            let (lbl, used) = read_label(chars, start + 2, "))")?;
            Ok((BlockShape::Circle, lbl, used + 2))
        }
        ('(', '[') => {
            let (lbl, used) = read_label(chars, start + 2, "])")?;
            Ok((BlockShape::Stadium, lbl, used + 2))
        }
        ('[', _) => {
            let (lbl, used) = read_label(chars, start + 1, "]")?;
            Ok((BlockShape::Square, lbl, used + 1))
        }
        ('(', _) => {
            let (lbl, used) = read_label(chars, start + 1, ")")?;
            Ok((BlockShape::Round, lbl, used + 1))
        }
        ('{', _) => {
            let (lbl, used) = read_label(chars, start + 1, "}")?;
            Ok((BlockShape::Rhombus, lbl, used + 1))
        }
        _ => Ok((BlockShape::Square, String::new(), 0)),
    }
}

fn read_label(
    chars: &[char],
    start: usize,
    close: &str,
) -> Result<(String, usize), AsciiRenderError> {
    let close_chars: Vec<char> = close.chars().collect();
    let mut i = start;
    let mut buf = String::new();
    let mut in_quotes = false;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            in_quotes = !in_quotes;
            i += 1;
            continue;
        }
        if !in_quotes && match_at(chars, i, &close_chars) {
            let label = buf.trim().trim_matches('"').to_string();
            return Ok((label, i - start + close_chars.len()));
        }
        buf.push(c);
        i += 1;
    }
    Err(AsciiRenderError::Parse {
        line: 0,
        msg: format!("unterminated label, expected '{}'", close),
    })
}

fn match_at(chars: &[char], at: usize, needle: &[char]) -> bool {
    if at + needle.len() > chars.len() {
        return false;
    }
    for (k, &n) in needle.iter().enumerate() {
        if chars[at + k] != n {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_block_minimal() {
        let src = "block-beta\na";
        let d = parse(src).unwrap();
        assert_eq!(d.blocks.len(), 1);
        assert_eq!(d.blocks[0].id, "a");
        assert_eq!(d.blocks[0].shape, BlockShape::Square);
    }

    #[test]
    fn parse_no_header_error() {
        let src = "a\nb";
        let err = parse(src).unwrap_err();
        assert!(matches!(err, AsciiRenderError::Parse { line: 1, .. }));
    }

    #[test]
    fn parse_columns_directive() {
        let src = "block-beta\ncolumns 3\na b c";
        let d = parse(src).unwrap();
        assert_eq!(d.columns, 3);
        assert_eq!(d.blocks.len(), 3);
    }

    #[test]
    fn parse_single_block_square() {
        let src = "block-beta\na[\"Hello\"]";
        let d = parse(src).unwrap();
        assert_eq!(d.blocks.len(), 1);
        assert_eq!(d.blocks[0].id, "a");
        assert_eq!(d.blocks[0].label, "Hello");
        assert_eq!(d.blocks[0].shape, BlockShape::Square);
    }

    #[test]
    fn parse_block_round() {
        let src = "block-beta\nb(\"Round\")";
        let d = parse(src).unwrap();
        assert_eq!(d.blocks.len(), 1);
        assert_eq!(d.blocks[0].shape, BlockShape::Round);
        assert_eq!(d.blocks[0].label, "Round");
    }

    #[test]
    fn parse_block_stadium() {
        let src = "block-beta\ne([Stadium])";
        let d = parse(src).unwrap();
        assert_eq!(d.blocks.len(), 1);
        assert_eq!(d.blocks[0].shape, BlockShape::Stadium);
        assert_eq!(d.blocks[0].label, "Stadium");
    }

    #[test]
    fn parse_block_diamond() {
        let src = "block-beta\nd{\"Diamond\"}";
        let d = parse(src).unwrap();
        assert_eq!(d.blocks.len(), 1);
        assert_eq!(d.blocks[0].shape, BlockShape::Rhombus);
        assert_eq!(d.blocks[0].label, "Diamond");
    }

    #[test]
    fn parse_block_circle() {
        let src = "block-beta\nc((\"Circle\"))";
        let d = parse(src).unwrap();
        assert_eq!(d.blocks.len(), 1);
        assert_eq!(d.blocks[0].shape, BlockShape::Circle);
        assert_eq!(d.blocks[0].label, "Circle");
    }

    #[test]
    fn parse_multiple_blocks_one_row() {
        let src = "block-beta\na b c";
        let d = parse(src).unwrap();
        assert_eq!(d.blocks.len(), 3);
        assert_eq!(d.blocks[0].id, "a");
        assert_eq!(d.blocks[1].id, "b");
        assert_eq!(d.blocks[2].id, "c");
    }

    #[test]
    fn parse_multiple_rows() {
        let src = "block-beta\na b\nc d";
        let d = parse(src).unwrap();
        assert_eq!(d.blocks.len(), 4);
        assert_eq!(d.blocks[0].id, "a");
        assert_eq!(d.blocks[3].id, "d");
    }

    #[test]
    fn parse_edge_simple() {
        let src = "block-beta\na b\nf --> g";
        let d = parse(src).unwrap();
        assert_eq!(d.edges.len(), 1);
        assert_eq!(d.edges[0].from, "f");
        assert_eq!(d.edges[0].to, "g");
        assert_eq!(d.edges[0].label, "");
        assert!(d.edges[0].arrow);
    }

    #[test]
    fn parse_edge_with_label() {
        let src = "block-beta\na -->|hello| b";
        let d = parse(src).unwrap();
        assert_eq!(d.edges.len(), 1);
        assert_eq!(d.edges[0].from, "a");
        assert_eq!(d.edges[0].to, "b");
        assert_eq!(d.edges[0].label, "hello");
    }

    #[test]
    fn parse_comments_stripped() {
        let src = "block-beta\n%% this is a comment\na b";
        let d = parse(src).unwrap();
        assert_eq!(d.blocks.len(), 2);
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let src = "block-beta\n\n\na b\n\nc d\n";
        let d = parse(src).unwrap();
        assert_eq!(d.blocks.len(), 4);
    }
}
