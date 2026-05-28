use super::ast::{ClassDef, Edge, EdgeArrow, EdgeStyle, Flowchart, Node, NodeShape, Subgraph};
use crate::mermaid::ascii::detect::FlowDirection;
use crate::mermaid::ascii::error::AsciiRenderError;
use crate::mermaid::ascii::label::Label;

pub fn parse(source: &str) -> Result<Flowchart, AsciiRenderError> {
    let raw_lines = split_lines(source);

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
    let direction = parse_header(&header).ok_or_else(|| AsciiRenderError::Parse {
        line: 1,
        msg: format!("missing or invalid graph header: '{}'", header.trim()),
    })?;

    let mut flow = Flowchart {
        direction,
        ..Default::default()
    };

    let mut subgraph_stack: Vec<String> = Vec::new();
    let mut auto_id_counter: usize = 0;

    for (i, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = strip_keyword(line, "subgraph") {
            let header = parse_subgraph_header(rest, &mut auto_id_counter);
            let parent = subgraph_stack.last().cloned();
            let id = header.id.clone();
            let mut sg = header;
            sg.parent = parent.clone();
            if let Some(parent_id) = parent {
                if let Some(p) = flow.subgraphs.get_mut(&parent_id) {
                    p.child_subgraphs.push(id.clone());
                }
            }
            flow.subgraphs.insert(id.clone(), sg);
            subgraph_stack.push(id);
            continue;
        }

        if line == "end" || line.eq_ignore_ascii_case("end") {
            subgraph_stack.pop();
            continue;
        }

        if let Some(rest) = strip_keyword(line, "classDef") {
            if let Some(cd) = parse_class_def(rest) {
                flow.class_defs.insert(cd.name.clone(), cd);
            } else {
                return Err(AsciiRenderError::Parse {
                    line: i + 2,
                    msg: format!("invalid classDef: '{}'", line),
                });
            }
            continue;
        }

        parse_statement(line, &mut flow, &subgraph_stack)
            .map_err(|msg| AsciiRenderError::Parse { line: i + 2, msg })?;
    }

    Ok(flow)
}

fn split_lines(src: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut bracket_depth: i32 = 0;
    let mut in_quotes = false;
    let bytes: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            '"' => {
                in_quotes = !in_quotes;
                current.push(c);
            }
            '[' | '(' => {
                if !in_quotes {
                    bracket_depth += 1;
                }
                current.push(c);
            }
            ']' | ')' => {
                if !in_quotes && bracket_depth > 0 {
                    bracket_depth -= 1;
                }
                current.push(c);
            }
            '\n' => {
                if bracket_depth == 0 && !in_quotes {
                    lines.push(std::mem::take(&mut current));
                } else {
                    current.push(c);
                }
            }
            '\\' => {
                if i + 1 < bytes.len() && bytes[i + 1] == 'n' && bracket_depth == 0 && !in_quotes {
                    lines.push(std::mem::take(&mut current));
                    i += 2;
                    continue;
                } else {
                    current.push(c);
                }
            }
            _ => current.push(c),
        }
        i += 1;
    }
    lines.push(current);
    lines
}

fn parse_header(line: &str) -> Option<FlowDirection> {
    let trimmed = line.trim();
    let rest = if let Some(r) = strip_keyword_ci(trimmed, "graph") {
        r
    } else if let Some(r) = strip_keyword_ci(trimmed, "flowchart") {
        r
    } else {
        return None;
    };
    let dir = rest.trim();
    match dir.to_ascii_uppercase().as_str() {
        "TD" | "TB" => Some(FlowDirection::TopDown),
        "LR" => Some(FlowDirection::LeftRight),
        "BT" => Some(FlowDirection::BottomTop),
        "RL" => Some(FlowDirection::RightLeft),
        "" => Some(FlowDirection::TopDown),
        _ => None,
    }
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

fn strip_keyword_ci<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    if line.len() < keyword.len() {
        return None;
    }
    let head = &line[..keyword.len()];
    if !head.eq_ignore_ascii_case(keyword) {
        return None;
    }
    let rest = &line[keyword.len()..];
    if rest.is_empty() {
        return Some("");
    }
    let next = rest.chars().next().unwrap();
    if next.is_whitespace() {
        Some(rest.trim_start())
    } else {
        None
    }
}

fn parse_subgraph_header(header: &str, counter: &mut usize) -> Subgraph {
    let trimmed = header.trim();

    if let Some(stripped) = trimmed.strip_prefix('"') {
        if let Some(close) = stripped.find('"') {
            let title = stripped[..close].to_string();
            *counter += 1;
            return Subgraph {
                id: format!("_sg{}", *counter),
                title,
                ..Default::default()
            };
        }
    }

    if let Some(open) = trimmed.find('[') {
        if trimmed.ends_with(']') && open > 0 {
            let id = trimmed[..open].trim().to_string();
            let mut title = trimmed[open + 1..trimmed.len() - 1].trim().to_string();
            if title.starts_with('"') && title.ends_with('"') && title.len() >= 2 {
                title = title[1..title.len() - 1].to_string();
            }
            return Subgraph {
                id,
                title,
                ..Default::default()
            };
        }
    }

    Subgraph {
        id: trimmed.to_string(),
        title: trimmed.to_string(),
        ..Default::default()
    }
}

fn parse_class_def(rest: &str) -> Option<ClassDef> {
    let rest = rest.trim();
    let space = rest.find(char::is_whitespace)?;
    let name = rest[..space].to_string();
    let style_str = rest[space..].trim();
    let mut styles = std::collections::HashMap::new();
    for part in style_str.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(colon) = part.find(':') {
            let k = part[..colon].trim().to_string();
            let v = part[colon + 1..].trim().to_string();
            styles.insert(k, v);
        }
    }
    Some(ClassDef { name, styles })
}

#[derive(Debug, Clone)]
struct EdgeOp {
    style: EdgeStyle,
    arrow: EdgeArrow,
    label: Option<String>,
    end: usize,
}

#[derive(Debug, Clone)]
enum ChainSegment {
    Nodes(Vec<Node>),
    Edge(EdgeOp),
}

fn parse_statement(
    line: &str,
    flow: &mut Flowchart,
    subgraph_stack: &[String],
) -> Result<(), String> {
    let chain = parse_chain(line)?;
    apply_chain(chain, flow, subgraph_stack);
    Ok(())
}

fn parse_chain(line: &str) -> Result<Vec<ChainSegment>, String> {
    let mut segments: Vec<ChainSegment> = Vec::new();
    let mut cursor = 0;
    let chars: Vec<char> = line.chars().collect();

    while cursor < chars.len() {
        while cursor < chars.len() && chars[cursor].is_whitespace() {
            cursor += 1;
        }
        if cursor >= chars.len() {
            break;
        }
        if let Some(edge) = try_match_edge(&chars, cursor) {
            cursor = edge.end;
            segments.push(ChainSegment::Edge(edge));
            continue;
        }
        let (nodes, next) = parse_nodes_lhs(&chars, cursor)?;
        cursor = next;
        segments.push(ChainSegment::Nodes(nodes));
    }

    Ok(segments)
}

fn try_match_edge(chars: &[char], start: usize) -> Option<EdgeOp> {
    let s: String = chars[start..].iter().collect();

    if let Some(rest) = s.strip_prefix("<-->") {
        let (label, after) = read_pipe_label(rest);
        return Some(EdgeOp {
            style: EdgeStyle::Solid,
            arrow: EdgeArrow::Bidirectional,
            label,
            end: start + 4 + after,
        });
    }
    if let Some(rest) = s.strip_prefix("<==>") {
        let (label, after) = read_pipe_label(rest);
        return Some(EdgeOp {
            style: EdgeStyle::Thick,
            arrow: EdgeArrow::Bidirectional,
            label,
            end: start + 4 + after,
        });
    }

    if s.starts_with("-.") {
        if let Some((len, arrow, label)) = parse_dotted(&s) {
            return Some(EdgeOp {
                style: EdgeStyle::Dotted,
                arrow,
                label,
                end: start + len,
            });
        }
    }

    if s.starts_with("==") {
        if let Some((len, arrow, label)) = parse_thick(&s) {
            return Some(EdgeOp {
                style: EdgeStyle::Thick,
                arrow,
                label,
                end: start + len,
            });
        }
    }

    if s.starts_with("~~~") {
        let bytes = s.as_bytes();
        let mut idx = 0;
        while idx < bytes.len() && bytes[idx] == b'~' {
            idx += 1;
        }
        return Some(EdgeOp {
            style: EdgeStyle::Invisible,
            arrow: EdgeArrow::Open,
            label: None,
            end: start + idx,
        });
    }

    if s.starts_with("--") {
        if let Some((len, arrow, label, style)) = parse_solid(&s) {
            return Some(EdgeOp {
                style,
                arrow,
                label,
                end: start + len,
            });
        }
    }

    None
}

fn parse_dotted(s: &str) -> Option<(usize, EdgeArrow, Option<String>)> {
    let bytes = s.as_bytes();
    if !s.starts_with("-.") {
        return None;
    }
    let mut idx = 1;
    while idx < bytes.len() && bytes[idx] == b'.' {
        idx += 1;
    }
    if idx >= bytes.len() {
        return None;
    }
    if bytes[idx] == b'-' {
        while idx < bytes.len() && bytes[idx] == b'-' {
            idx += 1;
        }
        let (arrow, marker_len) = match bytes.get(idx).copied() {
            Some(b'>') => (EdgeArrow::Closed, 1),
            Some(b'o') => (EdgeArrow::Circle, 1),
            Some(b'x') => (EdgeArrow::Cross, 1),
            _ => (EdgeArrow::Open, 0),
        };
        let total = idx + marker_len;
        let rest = &s[total..];
        let (label, after_label) = read_pipe_label(rest);
        return Some((total + after_label, arrow, label));
    }
    let label_start = idx;
    while idx < bytes.len() {
        if bytes[idx] == b'\n' {
            return None;
        }
        if bytes[idx] == b'.' {
            let mut probe = idx;
            while probe < bytes.len() && bytes[probe] == b'.' {
                probe += 1;
            }
            if probe < bytes.len() && bytes[probe] == b'-' {
                let label_text = s[label_start..idx].trim();
                let label = if label_text.is_empty() {
                    None
                } else {
                    Some(label_text.to_string())
                };
                let mut close = probe;
                while close < bytes.len() && bytes[close] == b'-' {
                    close += 1;
                }
                let (arrow, marker_len) = match bytes.get(close).copied() {
                    Some(b'>') => (EdgeArrow::Closed, 1),
                    Some(b'o') => (EdgeArrow::Circle, 1),
                    Some(b'x') => (EdgeArrow::Cross, 1),
                    _ => (EdgeArrow::Open, 0),
                };
                let total = close + marker_len;
                return Some((total, arrow, label));
            }
        }
        idx += 1;
    }
    None
}

fn parse_thick(s: &str) -> Option<(usize, EdgeArrow, Option<String>)> {
    let bytes = s.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() && bytes[idx] == b'=' {
        idx += 1;
    }
    if idx < 2 {
        return None;
    }
    let (arrow, marker_len) = match bytes.get(idx).copied() {
        Some(b'>') => (EdgeArrow::Closed, 1),
        Some(b'o') => (EdgeArrow::Circle, 1),
        Some(b'x') => (EdgeArrow::Cross, 1),
        _ => (EdgeArrow::Open, 0),
    };
    let total = idx + marker_len;
    let rest = &s[total..];
    let (label, after_label) = read_pipe_label(rest);
    Some((total + after_label, arrow, label))
}

fn parse_solid(s: &str) -> Option<(usize, EdgeArrow, Option<String>, EdgeStyle)> {
    let bytes = s.as_bytes();
    if !s.starts_with("--") {
        return None;
    }
    let mut idx = 0;
    while idx < bytes.len() && bytes[idx] == b'-' {
        idx += 1;
    }
    let (arrow, marker_len) = match bytes.get(idx).copied() {
        Some(b'>') => (EdgeArrow::Closed, 1),
        Some(b'o') => (EdgeArrow::Circle, 1),
        Some(b'x') => (EdgeArrow::Cross, 1),
        _ => (EdgeArrow::Open, 0),
    };
    let total = idx + marker_len;
    let rest = &s[total..];
    let (label, after_label) = read_pipe_label(rest);
    Some((total + after_label, arrow, label, EdgeStyle::Solid))
}

fn read_pipe_label(s: &str) -> (Option<String>, usize) {
    let trimmed = s.trim_start();
    let leading_ws = s.len() - trimmed.len();
    if !trimmed.starts_with('|') {
        return (None, 0);
    }
    let bytes = trimmed.as_bytes();
    let mut idx = 1;
    let mut buf = String::new();
    let mut in_quotes = false;
    while idx < bytes.len() {
        let b = bytes[idx];
        if b == b'"' {
            in_quotes = !in_quotes;
            buf.push('"');
            idx += 1;
            continue;
        }
        if b == b'|' && !in_quotes {
            idx += 1;
            let mut label = buf.trim().to_string();
            if label.starts_with('"') && label.ends_with('"') && label.len() >= 2 {
                label = label[1..label.len() - 1].to_string();
            }
            return (Some(label), leading_ws + idx);
        }
        buf.push(b as char);
        idx += 1;
    }
    (None, 0)
}

fn parse_nodes_lhs(chars: &[char], start: usize) -> Result<(Vec<Node>, usize), String> {
    let mut cursor = start;
    let mut nodes: Vec<Node> = Vec::new();
    loop {
        while cursor < chars.len() && chars[cursor].is_whitespace() {
            cursor += 1;
        }
        if cursor >= chars.len() {
            break;
        }
        if find_edge_at(chars, cursor) {
            break;
        }
        let (node, next) = parse_single_node(chars, cursor)?;
        nodes.push(node);
        cursor = next;
        let saved = cursor;
        while cursor < chars.len() && chars[cursor].is_whitespace() {
            cursor += 1;
        }
        if cursor < chars.len() && chars[cursor] == '&' {
            cursor += 1;
            continue;
        }
        cursor = saved;
        break;
    }
    if nodes.is_empty() {
        return Err(format!(
            "expected node at position {}, got: {}",
            start,
            chars[start..].iter().collect::<String>()
        ));
    }
    Ok((nodes, cursor))
}

fn find_edge_at(chars: &[char], pos: usize) -> bool {
    try_match_edge(chars, pos).is_some()
}

fn parse_single_node(chars: &[char], start: usize) -> Result<(Node, usize), String> {
    let mut idx = start;
    let mut name = String::new();
    while idx < chars.len() {
        let c = chars[idx];
        if c.is_whitespace() {
            break;
        }
        if matches!(c, '[' | '(' | '{' | '>') {
            break;
        }
        if c == '&' {
            break;
        }
        if c == ':' && idx + 2 < chars.len() && chars[idx + 1] == ':' && chars[idx + 2] == ':' {
            break;
        }
        if c == '|' {
            break;
        }
        if find_edge_at(chars, idx) {
            break;
        }
        name.push(c);
        idx += 1;
    }

    if name.is_empty() {
        return Err(format!(
            "expected identifier at position {}: '{}'",
            start,
            chars[start..].iter().collect::<String>()
        ));
    }

    let mut shape = NodeShape::Square;
    let mut label_text = name.clone();
    let mut has_explicit_label = false;

    if idx < chars.len() && matches!(chars[idx], '[' | '(' | '{' | '>') {
        let (sh, lbl, next) = parse_shape(chars, idx)?;
        shape = sh;
        label_text = lbl;
        has_explicit_label = true;
        idx = next;
    }

    let mut class_name: Option<String> = None;
    if idx + 2 < chars.len() && chars[idx] == ':' && chars[idx + 1] == ':' && chars[idx + 2] == ':'
    {
        idx += 3;
        let mut cn = String::new();
        while idx < chars.len() {
            let c = chars[idx];
            if c.is_whitespace() || find_edge_at(chars, idx) || c == '&' {
                break;
            }
            cn.push(c);
            idx += 1;
        }
        if !cn.is_empty() {
            class_name = Some(cn);
        }
    }

    let label = if has_explicit_label {
        Label::new(label_text)
    } else {
        Label::new(name.clone())
    };

    Ok((
        Node {
            id: name,
            label,
            shape,
            class_name,
        },
        idx,
    ))
}

fn parse_shape(chars: &[char], start: usize) -> Result<(NodeShape, String, usize), String> {
    let s: String = chars[start..].iter().collect();
    let bytes = s.as_bytes();

    if s.starts_with("([") {
        if let Some(end) = find_close_seq(&s, "])") {
            let inner = strip_quotes(&s[2..end]);
            return Ok((NodeShape::Stadium, inner, start + end + 2));
        }
    }
    if s.starts_with("[[") {
        if let Some(end) = find_close_seq(&s, "]]") {
            let inner = strip_quotes(&s[2..end]);
            return Ok((NodeShape::Subroutine, inner, start + end + 2));
        }
    }
    if s.starts_with("[(") {
        if let Some(end) = find_close_seq(&s, ")]") {
            let inner = strip_quotes(&s[2..end]);
            return Ok((NodeShape::Cylinder, inner, start + end + 2));
        }
    }
    if s.starts_with("[/") {
        if let Some(end) = find_close_seq(&s, "/]") {
            let inner = strip_quotes(&s[2..end]);
            return Ok((NodeShape::Parallelogram, inner, start + end + 2));
        }
        if let Some(end) = find_close_seq(&s, "\\]") {
            let inner = strip_quotes(&s[2..end]);
            return Ok((NodeShape::Trapezoid, inner, start + end + 2));
        }
    }
    if s.starts_with("[\\") {
        if let Some(end) = find_close_seq(&s, "\\]") {
            let inner = strip_quotes(&s[2..end]);
            return Ok((NodeShape::ParallelogramAlt, inner, start + end + 2));
        }
        if let Some(end) = find_close_seq(&s, "/]") {
            let inner = strip_quotes(&s[2..end]);
            return Ok((NodeShape::TrapezoidAlt, inner, start + end + 2));
        }
    }
    if s.starts_with("((") {
        if let Some(end) = find_close_seq(&s, "))") {
            let inner = strip_quotes(&s[2..end]);
            return Ok((NodeShape::Circle, inner, start + end + 2));
        }
    }
    if s.starts_with("{{") {
        if let Some(end) = find_close_seq(&s, "}}") {
            let inner = strip_quotes(&s[2..end]);
            return Ok((NodeShape::Hexagon, inner, start + end + 2));
        }
    }
    if bytes.first() == Some(&b'[') {
        if let Some(end) = find_close_char(&s, '[', ']') {
            let inner = strip_quotes(&s[1..end]);
            return Ok((NodeShape::Square, inner, start + end + 1));
        }
    }
    if bytes.first() == Some(&b'(') {
        if let Some(end) = find_close_char(&s, '(', ')') {
            let inner = strip_quotes(&s[1..end]);
            return Ok((NodeShape::Round, inner, start + end + 1));
        }
    }
    if bytes.first() == Some(&b'{') {
        if let Some(end) = find_close_char(&s, '{', '}') {
            let inner = strip_quotes(&s[1..end]);
            return Ok((NodeShape::Rhombus, inner, start + end + 1));
        }
    }
    if bytes.first() == Some(&b'>') {
        if let Some(end) = s.find(']') {
            let inner = strip_quotes(&s[1..end]);
            return Ok((NodeShape::Asymmetric, inner, start + end + 1));
        }
    }

    Err(format!("unable to parse shape at: {}", s))
}

fn find_close_seq(s: &str, close: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let cbytes = close.as_bytes();
    let mut i = close.len();
    let mut in_quotes = false;
    while i + cbytes.len() <= bytes.len() {
        let b = bytes[i];
        if b == b'"' {
            in_quotes = !in_quotes;
        }
        if !in_quotes && &bytes[i..i + cbytes.len()] == cbytes {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_close_char(s: &str, open: char, close: char) -> Option<usize> {
    let bytes = s.as_bytes();
    let open_b = open as u8;
    let close_b = close as u8;
    let mut depth = 1;
    let mut i = 1;
    let mut in_quotes = false;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'"' {
            in_quotes = !in_quotes;
        } else if !in_quotes {
            if b == open_b {
                depth += 1;
            } else if b == close_b {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
        }
        i += 1;
    }
    None
}

fn strip_quotes(s: &str) -> String {
    let t = s.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

fn apply_chain(segments: Vec<ChainSegment>, flow: &mut Flowchart, subgraph_stack: &[String]) {
    let mut prev_nodes: Option<Vec<Node>> = None;
    let mut pending_edge: Option<EdgeOp> = None;

    for seg in segments {
        match seg {
            ChainSegment::Nodes(nodes) => {
                register_nodes(&nodes, flow, subgraph_stack);
                if let (Some(prev), Some(edge)) = (prev_nodes.take(), pending_edge.take()) {
                    for l in &prev {
                        for r in &nodes {
                            flow.edges.push(Edge {
                                from: l.id.clone(),
                                to: r.id.clone(),
                                label: edge.label.clone().unwrap_or_default(),
                                style: edge.style,
                                arrow: edge.arrow,
                            });
                        }
                    }
                }
                prev_nodes = Some(nodes);
            }
            ChainSegment::Edge(edge) => {
                pending_edge = Some(edge);
            }
        }
    }
}

fn register_nodes(nodes: &[Node], flow: &mut Flowchart, subgraph_stack: &[String]) {
    for node in nodes {
        let id = node.id.clone();
        let existed = flow.nodes.contains_key(&id);
        if !existed {
            flow.nodes.insert(id.clone(), node.clone());
        } else if let Some(existing) = flow.nodes.get_mut(&id) {
            if !node.label.lines.is_empty() && node.label.lines != vec![id.clone()] {
                existing.label = node.label.clone();
                existing.shape = node.shape.clone();
            }
            if node.class_name.is_some() {
                existing.class_name = node.class_name.clone();
            }
        }
        if !existed {
            for sg_id in subgraph_stack {
                if let Some(sg) = flow.subgraphs.get_mut(sg_id) {
                    if !sg.node_ids.contains(&id) {
                        sg.node_ids.push(id.clone());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(src: &str) -> Flowchart {
        parse(src).expect("expected ok parse")
    }

    // --- Header & basics ------------------------------------------------

    #[test]
    fn parse_graph_td_minimal() {
        let f = parse_ok("graph TD\nA --> B");
        assert_eq!(f.direction, FlowDirection::TopDown);
        assert!(f.nodes.contains_key("A"));
        assert!(f.nodes.contains_key("B"));
        assert_eq!(f.edges.len(), 1);
        assert_eq!(f.edges[0].from, "A");
        assert_eq!(f.edges[0].to, "B");
    }

    #[test]
    fn parse_flowchart_lr_minimal() {
        let f = parse_ok("flowchart LR\nA --> B");
        assert_eq!(f.direction, FlowDirection::LeftRight);
    }

    #[test]
    fn parse_graph_bt_direction() {
        let f = parse_ok("graph BT\nA --> B");
        assert_eq!(f.direction, FlowDirection::BottomTop);
    }

    #[test]
    fn parse_graph_rl_direction() {
        let f = parse_ok("graph RL\nA --> B");
        assert_eq!(f.direction, FlowDirection::RightLeft);
    }

    #[test]
    fn parse_empty_returns_error() {
        assert!(matches!(parse(""), Err(AsciiRenderError::Empty)));
    }

    #[test]
    fn parse_missing_header_returns_error() {
        match parse("A --> B") {
            Err(AsciiRenderError::Parse { .. }) => {}
            other => panic!("expected Parse error, got {:?}", other),
        }
    }

    #[test]
    fn parse_comments_are_stripped() {
        let f = parse_ok("graph TD\n%% this is a comment\nA --> B");
        assert_eq!(f.edges.len(), 1);
    }

    #[test]
    fn parse_inline_comment_is_stripped() {
        let f = parse_ok("graph TD\nA --> B %% trailing\n");
        assert_eq!(f.edges.len(), 1);
        assert_eq!(f.edges[0].from, "A");
        assert_eq!(f.edges[0].to, "B");
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let f = parse_ok("graph TD\n\n\nA --> B\n\n");
        assert_eq!(f.edges.len(), 1);
    }

    // --- Node shapes ----------------------------------------------------

    #[test]
    fn shape_square_brackets() {
        let f = parse_ok("graph TD\nA[hello]");
        assert_eq!(f.nodes["A"].shape, NodeShape::Square);
        assert_eq!(f.nodes["A"].label.lines, vec!["hello".to_string()]);
    }

    #[test]
    fn shape_round_parens() {
        let f = parse_ok("graph TD\nA(hello)");
        assert_eq!(f.nodes["A"].shape, NodeShape::Round);
    }

    #[test]
    fn shape_stadium_brackets_around_parens() {
        let f = parse_ok("graph TD\nA([hello])");
        assert_eq!(f.nodes["A"].shape, NodeShape::Stadium);
    }

    #[test]
    fn shape_subroutine_double_brackets() {
        let f = parse_ok("graph TD\nA[[hello]]");
        assert_eq!(f.nodes["A"].shape, NodeShape::Subroutine);
    }

    #[test]
    fn shape_cylinder_bracket_paren() {
        let f = parse_ok("graph TD\nA[(db)]");
        assert_eq!(f.nodes["A"].shape, NodeShape::Cylinder);
        assert_eq!(f.nodes["A"].label.lines, vec!["db".to_string()]);
    }

    #[test]
    fn shape_circle_double_parens() {
        let f = parse_ok("graph TD\nA((c))");
        assert_eq!(f.nodes["A"].shape, NodeShape::Circle);
    }

    #[test]
    fn shape_asymmetric_gt() {
        let f = parse_ok("graph TD\nA>note]");
        assert_eq!(f.nodes["A"].shape, NodeShape::Asymmetric);
        assert_eq!(f.nodes["A"].label.lines, vec!["note".to_string()]);
    }

    #[test]
    fn shape_rhombus_braces() {
        let f = parse_ok("graph TD\nA{decide}");
        assert_eq!(f.nodes["A"].shape, NodeShape::Rhombus);
    }

    #[test]
    fn shape_hexagon_double_braces() {
        let f = parse_ok("graph TD\nA{{hex}}");
        assert_eq!(f.nodes["A"].shape, NodeShape::Hexagon);
    }

    #[test]
    fn shape_parallelogram_slash() {
        let f = parse_ok("graph TD\nA[/text/]");
        assert_eq!(f.nodes["A"].shape, NodeShape::Parallelogram);
    }

    #[test]
    fn shape_parallelogram_alt_backslash() {
        let f = parse_ok("graph TD\nA[\\text\\]");
        assert_eq!(f.nodes["A"].shape, NodeShape::ParallelogramAlt);
    }

    #[test]
    fn shape_trapezoid_slash_backslash() {
        let f = parse_ok("graph TD\nA[/text\\]");
        assert_eq!(f.nodes["A"].shape, NodeShape::Trapezoid);
    }

    #[test]
    fn shape_trapezoid_alt_backslash_slash() {
        let f = parse_ok("graph TD\nA[\\text/]");
        assert_eq!(f.nodes["A"].shape, NodeShape::TrapezoidAlt);
    }

    #[test]
    fn shape_default_when_no_brackets() {
        let f = parse_ok("graph TD\nA --> B");
        assert_eq!(f.nodes["A"].shape, NodeShape::Square);
        assert_eq!(f.nodes["A"].label.lines, vec!["A".to_string()]);
    }

    // --- Edges ----------------------------------------------------------

    #[test]
    fn edge_solid_arrow() {
        let f = parse_ok("graph TD\nA --> B");
        assert_eq!(f.edges[0].style, EdgeStyle::Solid);
        assert_eq!(f.edges[0].arrow, EdgeArrow::Closed);
    }

    #[test]
    fn edge_dotted_arrow() {
        let f = parse_ok("graph TD\nA -.-> B");
        assert_eq!(f.edges[0].style, EdgeStyle::Dotted);
        assert_eq!(f.edges[0].arrow, EdgeArrow::Closed);
    }

    #[test]
    fn edge_dotted_inline_label() {
        let f = parse_ok("graph TD\nA -.symlink.-> B");
        assert_eq!(f.edges[0].style, EdgeStyle::Dotted);
        assert_eq!(f.edges[0].arrow, EdgeArrow::Closed);
        assert_eq!(f.edges[0].label, "symlink");
    }

    #[test]
    fn edge_dotted_inline_label_spaced() {
        let f = parse_ok("graph TD\nA -. hello world .-> B");
        assert_eq!(f.edges[0].style, EdgeStyle::Dotted);
        assert_eq!(f.edges[0].label, "hello world");
    }

    #[test]
    fn edge_dotted_inline_label_with_dot() {
        let f = parse_ok("graph TD\nA -.foo.bar.-> B");
        assert_eq!(f.edges[0].style, EdgeStyle::Dotted);
        assert_eq!(f.edges[0].label, "foo.bar");
    }

    #[test]
    fn edge_dotted_inline_label_cross_marker() {
        let f = parse_ok("graph TD\nA -.nope.-x B");
        assert_eq!(f.edges[0].style, EdgeStyle::Dotted);
        assert_eq!(f.edges[0].arrow, EdgeArrow::Cross);
        assert_eq!(f.edges[0].label, "nope");
    }

    #[test]
    fn edge_thick_arrow() {
        let f = parse_ok("graph TD\nA ==> B");
        assert_eq!(f.edges[0].style, EdgeStyle::Thick);
        assert_eq!(f.edges[0].arrow, EdgeArrow::Closed);
    }

    #[test]
    fn edge_invisible() {
        let f = parse_ok("graph TD\nA ~~~ B");
        assert_eq!(f.edges[0].style, EdgeStyle::Invisible);
    }

    #[test]
    fn edge_with_pipe_label() {
        let f = parse_ok("graph TD\nA -->|yes| B");
        assert_eq!(f.edges[0].label, "yes");
    }

    #[test]
    fn edge_with_inline_label_after_dashes() {
        let f = parse_ok("graph TD\nA -->|hello world| B");
        assert_eq!(f.edges[0].label, "hello world");
    }

    #[test]
    fn edge_bidirectional() {
        let f = parse_ok("graph TD\nA <--> B");
        assert_eq!(f.edges[0].arrow, EdgeArrow::Bidirectional);
    }

    #[test]
    fn edge_open_arrow() {
        let f = parse_ok("graph TD\nA --- B");
        assert_eq!(f.edges[0].style, EdgeStyle::Solid);
        assert_eq!(f.edges[0].arrow, EdgeArrow::Open);
    }

    #[test]
    fn edge_circle_arrow_marker() {
        let f = parse_ok("graph TD\nA --o B");
        assert_eq!(f.edges[0].arrow, EdgeArrow::Circle);
    }

    #[test]
    fn edge_cross_arrow_marker() {
        let f = parse_ok("graph TD\nA --x B");
        assert_eq!(f.edges[0].arrow, EdgeArrow::Cross);
    }

    // --- Multi-node / chains --------------------------------------------

    #[test]
    fn chained_lhs_with_ampersand() {
        let f = parse_ok("graph TD\nA & B --> C");
        assert_eq!(f.edges.len(), 2);
        assert!(f.edges.iter().any(|e| e.from == "A" && e.to == "C"));
        assert!(f.edges.iter().any(|e| e.from == "B" && e.to == "C"));
    }

    #[test]
    fn chained_rhs_with_ampersand() {
        let f = parse_ok("graph TD\nA --> B & C");
        assert_eq!(f.edges.len(), 2);
        assert!(f.edges.iter().any(|e| e.from == "A" && e.to == "B"));
        assert!(f.edges.iter().any(|e| e.from == "A" && e.to == "C"));
    }

    #[test]
    fn chain_creates_cross_product_edges() {
        let f = parse_ok("graph TD\nA & B --> C & D");
        assert_eq!(f.edges.len(), 4);
    }

    #[test]
    fn sequential_chain_three_nodes() {
        let f = parse_ok("graph TD\nA --> B --> C");
        assert_eq!(f.edges.len(), 2);
        assert_eq!(f.edges[0].from, "A");
        assert_eq!(f.edges[0].to, "B");
        assert_eq!(f.edges[1].from, "B");
        assert_eq!(f.edges[1].to, "C");
    }

    // --- Subgraphs & classes --------------------------------------------

    #[test]
    fn subgraph_with_id_and_title() {
        let src = "graph TD\nsubgraph sg1 [My Title]\nA --> B\nend";
        let f = parse_ok(src);
        let sg = f.subgraphs.get("sg1").expect("sg1");
        assert_eq!(sg.title, "My Title");
        assert!(sg.node_ids.contains(&"A".to_string()));
        assert!(sg.node_ids.contains(&"B".to_string()));
    }

    #[test]
    fn subgraph_quoted_title() {
        let src = "graph TD\nsubgraph \"My Title\"\nA --> B\nend";
        let f = parse_ok(src);
        let (_id, sg) = f.subgraphs.iter().next().expect("subgraph");
        assert_eq!(sg.title, "My Title");
    }

    #[test]
    fn subgraph_nested() {
        let src = "graph TD\nsubgraph outer\nsubgraph inner\nA --> B\nend\nend";
        let f = parse_ok(src);
        assert_eq!(f.subgraphs.len(), 2);
        let outer = f.subgraphs.get("outer").expect("outer");
        assert_eq!(outer.child_subgraphs, vec!["inner".to_string()]);
        let inner = f.subgraphs.get("inner").expect("inner");
        assert_eq!(inner.parent.as_deref(), Some("outer"));
    }

    #[test]
    fn subgraph_end_keyword() {
        let src = "graph TD\nsubgraph s1\nA\nend\nB --> A";
        let f = parse_ok(src);
        let sg = f.subgraphs.get("s1").expect("s1");
        assert!(sg.node_ids.contains(&"A".to_string()));
        assert!(!sg.node_ids.contains(&"B".to_string()));
    }

    #[test]
    fn classdef_parsed() {
        let f = parse_ok("graph TD\nclassDef big fill:#f9f,stroke:#333\nA --> B");
        let cd = f.class_defs.get("big").expect("class big");
        assert_eq!(cd.styles.get("fill"), Some(&"#f9f".to_string()));
        assert_eq!(cd.styles.get("stroke"), Some(&"#333".to_string()));
    }

    #[test]
    fn node_with_class_via_triple_colon() {
        let f = parse_ok("graph TD\nA:::big --> B");
        assert_eq!(f.nodes["A"].class_name.as_deref(), Some("big"));
    }

    #[test]
    fn node_label_with_quoted_string() {
        let f = parse_ok("graph TD\nA[\"hello world\"]");
        assert_eq!(f.nodes["A"].label.lines, vec!["hello world".to_string()]);
    }

    #[test]
    fn node_label_with_html_br() {
        let f = parse_ok("graph TD\nA[line1<br>line2]");
        assert_eq!(
            f.nodes["A"].label.lines,
            vec!["line1".to_string(), "line2".to_string()]
        );
    }

    // --- Errors / edge cases --------------------------------------------

    #[test]
    fn lines_split_respects_brackets() {
        let src = "graph TD\nA[hello\nworld] --> B";
        let f = parse_ok(src);
        assert!(f.nodes.contains_key("A"));
        assert!(f.nodes.contains_key("B"));
    }

    #[test]
    fn lines_split_respects_quotes() {
        let src = "graph TD\nA[\"a\nb\"] --> B";
        let f = parse_ok(src);
        assert!(f.nodes.contains_key("A"));
        assert!(f.nodes.contains_key("B"));
    }

    #[test]
    fn lines_split_handles_escaped_newline() {
        let src = "graph TD\\nA --> B";
        let f = parse_ok(src);
        assert_eq!(f.edges.len(), 1);
    }
}
