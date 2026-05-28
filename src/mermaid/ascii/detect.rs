#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowDirection {
    TopDown,
    LeftRight,
    BottomTop,
    RightLeft,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagramKind {
    Flowchart(FlowDirection),
    Sequence,
    Class,
    State,
    ErDiagram,
    Pie,
    Gantt,
    Journey,
    Timeline,
    Mindmap,
    GitGraph,
    Quadrant,
    Requirement,
    Sankey,
    XYChart,
    Block,
    Architecture,
    Packet,
    Unknown,
}

pub fn detect_kind(source: &str) -> DiagramKind {
    let mut in_frontmatter = false;
    let mut frontmatter_started = false;

    for raw in source.lines() {
        let line = raw.trim();

        if in_frontmatter {
            if line == "---" {
                in_frontmatter = false;
            }
            continue;
        }

        if line.is_empty() {
            continue;
        }

        if !frontmatter_started && line == "---" {
            in_frontmatter = true;
            frontmatter_started = true;
            continue;
        }
        frontmatter_started = true;

        if line.starts_with("%%") {
            continue;
        }

        return classify(line);
    }

    DiagramKind::Unknown
}

fn classify(line: &str) -> DiagramKind {
    let lower = line.to_ascii_lowercase();

    if let Some(rest) = strip_keyword(&lower, "flowchart") {
        return DiagramKind::Flowchart(parse_direction(rest));
    }
    if let Some(rest) = strip_keyword(&lower, "graph") {
        return DiagramKind::Flowchart(parse_direction(rest));
    }
    if has_keyword(&lower, "sequencediagram") {
        return DiagramKind::Sequence;
    }
    if has_keyword(&lower, "classdiagram") {
        return DiagramKind::Class;
    }
    if has_keyword(&lower, "statediagram-v2") || has_keyword(&lower, "statediagram") {
        return DiagramKind::State;
    }
    if has_keyword(&lower, "erdiagram") {
        return DiagramKind::ErDiagram;
    }
    if has_keyword(&lower, "gantt") {
        return DiagramKind::Gantt;
    }
    if has_keyword(&lower, "journey") {
        return DiagramKind::Journey;
    }
    if has_keyword(&lower, "timeline") {
        return DiagramKind::Timeline;
    }
    if has_keyword(&lower, "mindmap") {
        return DiagramKind::Mindmap;
    }
    if has_keyword(&lower, "gitgraph") {
        return DiagramKind::GitGraph;
    }
    if has_keyword(&lower, "quadrantchart") {
        return DiagramKind::Quadrant;
    }
    if has_keyword(&lower, "requirementdiagram") {
        return DiagramKind::Requirement;
    }
    if has_keyword(&lower, "sankey-beta") {
        return DiagramKind::Sankey;
    }
    if has_keyword(&lower, "xychart-beta") {
        return DiagramKind::XYChart;
    }
    if has_keyword(&lower, "block-beta") {
        return DiagramKind::Block;
    }
    if has_keyword(&lower, "architecture-beta") {
        return DiagramKind::Architecture;
    }
    if has_keyword(&lower, "packet-beta") {
        return DiagramKind::Packet;
    }
    if has_keyword(&lower, "pie") {
        return DiagramKind::Pie;
    }

    DiagramKind::Unknown
}

fn has_keyword(lower_line: &str, keyword: &str) -> bool {
    if !lower_line.starts_with(keyword) {
        return false;
    }
    match lower_line.as_bytes().get(keyword.len()) {
        None => true,
        Some(b) => !is_keyword_byte(*b),
    }
}

fn strip_keyword<'a>(lower_line: &'a str, keyword: &str) -> Option<&'a str> {
    if !lower_line.starts_with(keyword) {
        return None;
    }
    let rest = &lower_line[keyword.len()..];
    match rest.as_bytes().first() {
        None => Some(""),
        Some(b) if !is_keyword_byte(*b) => Some(rest),
        _ => None,
    }
}

fn is_keyword_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

fn parse_direction(rest: &str) -> FlowDirection {
    let trimmed = rest.trim();
    let token: String = trimmed
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    match token.as_str() {
        "td" | "tb" => FlowDirection::TopDown,
        "lr" => FlowDirection::LeftRight,
        "bt" => FlowDirection::BottomTop,
        "rl" => FlowDirection::RightLeft,
        _ => FlowDirection::TopDown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_graph_td() {
        assert_eq!(
            detect_kind("graph TD\nA --> B"),
            DiagramKind::Flowchart(FlowDirection::TopDown)
        );
    }

    #[test]
    fn detects_graph_tb_same_as_td() {
        assert_eq!(
            detect_kind("graph TB\nA --> B"),
            DiagramKind::Flowchart(FlowDirection::TopDown)
        );
    }

    #[test]
    fn detects_graph_lr() {
        assert_eq!(
            detect_kind("graph LR\nA --> B"),
            DiagramKind::Flowchart(FlowDirection::LeftRight)
        );
    }

    #[test]
    fn detects_graph_bt() {
        assert_eq!(
            detect_kind("graph BT\nA --> B"),
            DiagramKind::Flowchart(FlowDirection::BottomTop)
        );
    }

    #[test]
    fn detects_graph_rl() {
        assert_eq!(
            detect_kind("graph RL\nA --> B"),
            DiagramKind::Flowchart(FlowDirection::RightLeft)
        );
    }

    #[test]
    fn detects_flowchart_td() {
        assert_eq!(
            detect_kind("flowchart TD\nA --> B"),
            DiagramKind::Flowchart(FlowDirection::TopDown)
        );
    }

    #[test]
    fn detects_flowchart_lr() {
        assert_eq!(
            detect_kind("flowchart LR\nA --> B"),
            DiagramKind::Flowchart(FlowDirection::LeftRight)
        );
    }

    #[test]
    fn detects_flowchart_tb() {
        assert_eq!(
            detect_kind("flowchart TB\nA --> B"),
            DiagramKind::Flowchart(FlowDirection::TopDown)
        );
    }

    #[test]
    fn detects_flowchart_bt() {
        assert_eq!(
            detect_kind("flowchart BT\nA --> B"),
            DiagramKind::Flowchart(FlowDirection::BottomTop)
        );
    }

    #[test]
    fn detects_flowchart_rl() {
        assert_eq!(
            detect_kind("flowchart RL\nA --> B"),
            DiagramKind::Flowchart(FlowDirection::RightLeft)
        );
    }

    #[test]
    fn detects_sequence_diagram() {
        assert_eq!(
            detect_kind("sequenceDiagram\nAlice->>Bob: hi"),
            DiagramKind::Sequence
        );
    }

    #[test]
    fn detects_class_diagram() {
        assert_eq!(detect_kind("classDiagram\nclass Foo"), DiagramKind::Class);
    }

    #[test]
    fn detects_state_diagram_v1() {
        assert_eq!(detect_kind("stateDiagram\n[*] --> A"), DiagramKind::State);
    }

    #[test]
    fn detects_state_diagram_v2() {
        assert_eq!(
            detect_kind("stateDiagram-v2\n[*] --> A"),
            DiagramKind::State
        );
    }

    #[test]
    fn detects_er_diagram() {
        assert_eq!(
            detect_kind("erDiagram\nCUSTOMER ||--o{ ORDER : places"),
            DiagramKind::ErDiagram
        );
    }

    #[test]
    fn detects_pie() {
        assert_eq!(detect_kind("pie\n\"A\" : 50"), DiagramKind::Pie);
    }

    #[test]
    fn detects_pie_with_title() {
        assert_eq!(
            detect_kind("pie title my chart\n\"A\" : 50"),
            DiagramKind::Pie
        );
    }

    #[test]
    fn detects_gantt() {
        assert_eq!(detect_kind("gantt\ntitle planning"), DiagramKind::Gantt);
    }

    #[test]
    fn detects_journey() {
        assert_eq!(detect_kind("journey\ntitle My day"), DiagramKind::Journey);
    }

    #[test]
    fn detects_timeline() {
        assert_eq!(
            detect_kind("timeline\ntitle history"),
            DiagramKind::Timeline
        );
    }

    #[test]
    fn detects_mindmap() {
        assert_eq!(detect_kind("mindmap\nroot"), DiagramKind::Mindmap);
    }

    #[test]
    fn detects_gitgraph() {
        assert_eq!(detect_kind("gitGraph\ncommit"), DiagramKind::GitGraph);
        assert_eq!(detect_kind("gitgraph\ncommit"), DiagramKind::GitGraph);
    }

    #[test]
    fn detects_quadrant_chart() {
        assert_eq!(detect_kind("quadrantChart\ntitle x"), DiagramKind::Quadrant);
    }

    #[test]
    fn detects_requirement_diagram() {
        assert_eq!(
            detect_kind("requirementDiagram\nrequirement r {"),
            DiagramKind::Requirement
        );
    }

    #[test]
    fn detects_sankey_beta() {
        assert_eq!(detect_kind("sankey-beta\nA,B,10"), DiagramKind::Sankey);
    }

    #[test]
    fn detects_xychart_beta() {
        assert_eq!(detect_kind("xychart-beta\ntitle x"), DiagramKind::XYChart);
    }

    #[test]
    fn detects_block_beta() {
        assert_eq!(detect_kind("block-beta\ncolumns 3"), DiagramKind::Block);
    }

    #[test]
    fn detects_architecture_beta() {
        assert_eq!(
            detect_kind("architecture-beta\ngroup foo"),
            DiagramKind::Architecture
        );
    }

    #[test]
    fn detects_packet_beta() {
        assert_eq!(detect_kind("packet-beta\n0-15: hdr"), DiagramKind::Packet);
    }

    #[test]
    fn skips_yaml_frontmatter() {
        let src = "---\ntitle: hello\nconfig:\n  theme: dark\n---\ngraph LR\nA --> B";
        assert_eq!(
            detect_kind(src),
            DiagramKind::Flowchart(FlowDirection::LeftRight)
        );
    }

    #[test]
    fn skips_init_directive() {
        let src = "%%{init: {\"theme\":\"dark\"}}%%\nsequenceDiagram\nA->>B: hi";
        assert_eq!(detect_kind(src), DiagramKind::Sequence);
    }

    #[test]
    fn skips_leading_blank_lines() {
        let src = "\n\n   \n\ngraph TD\nA --> B";
        assert_eq!(
            detect_kind(src),
            DiagramKind::Flowchart(FlowDirection::TopDown)
        );
    }

    #[test]
    fn skips_comment_lines() {
        let src = "%% this is a comment\n%% another\ngraph LR\nA --> B";
        assert_eq!(
            detect_kind(src),
            DiagramKind::Flowchart(FlowDirection::LeftRight)
        );
    }

    #[test]
    fn case_insensitive_keywords() {
        assert_eq!(
            detect_kind("GRAPH lr\nA --> B"),
            DiagramKind::Flowchart(FlowDirection::LeftRight)
        );
        assert_eq!(
            detect_kind("SequenceDiagram\nA->>B: hi"),
            DiagramKind::Sequence
        );
        assert_eq!(detect_kind("CLASSDIAGRAM\nclass Foo"), DiagramKind::Class);
    }

    #[test]
    fn unknown_for_garbage() {
        assert_eq!(
            detect_kind("hello world\nthis is not mermaid"),
            DiagramKind::Unknown
        );
    }

    #[test]
    fn unknown_for_empty() {
        assert_eq!(detect_kind(""), DiagramKind::Unknown);
    }

    #[test]
    fn whitespace_only_returns_unknown() {
        assert_eq!(detect_kind("   \n\t\n  \n"), DiagramKind::Unknown);
    }
}
