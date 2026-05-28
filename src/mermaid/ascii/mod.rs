pub mod astar;
pub mod canvas;
pub mod charset;
pub mod coord;
pub mod detect;
pub mod error;
pub mod label;

pub mod architecture;
pub mod block;
pub mod class;
pub mod er;
pub mod flowchart;
pub mod gantt;
pub mod gitgraph;
pub mod journey;
pub mod mindmap;
pub mod packet;
pub mod pie;
pub mod quadrant;
pub mod requirement;
pub mod sankey;
pub mod sequence;
pub mod state;
pub mod timeline;
pub mod xychart;

pub use canvas::{Canvas, StyledRow, StyledRun};
pub use charset::{Charset, CharsetKind};
pub use coord::{Coord, Direction};
pub use detect::{detect_kind, DiagramKind};
pub use error::AsciiRenderError;

use ratatui::style::Color;

pub fn render_mermaid_styled(source: &str, max_width: u16) -> Vec<StyledRow> {
    let kind = detect::detect_kind(source);
    let charset = CharsetKind::Unicode;
    let result: Result<Vec<StyledRow>, AsciiRenderError> = match kind {
        DiagramKind::Flowchart(_) => flowchart::render_styled(source, max_width, charset),
        DiagramKind::Sequence => sequence::render_styled(source, max_width, charset),
        DiagramKind::Class => class::render_styled(source, max_width, charset),
        DiagramKind::State => state::render_styled(source, max_width, charset),
        DiagramKind::ErDiagram => er::render_styled(source, max_width, charset),
        DiagramKind::Pie => pie::render_styled(source, max_width, charset),
        DiagramKind::Gantt => gantt::render_styled(source, max_width, charset),
        DiagramKind::Journey => journey::render_styled(source, max_width, charset),
        DiagramKind::Timeline => timeline::render_styled(source, max_width, charset),
        DiagramKind::Mindmap => mindmap::render_styled(source, max_width, charset),
        DiagramKind::GitGraph => gitgraph::render_styled(source, max_width, charset),
        DiagramKind::Quadrant => quadrant::render_styled(source, max_width, charset),
        DiagramKind::Requirement => requirement::render_styled(source, max_width, charset),
        DiagramKind::Sankey => sankey::render_styled(source, max_width, charset),
        DiagramKind::XYChart => xychart::render_styled(source, max_width, charset),
        DiagramKind::Block => block::render_styled(source, max_width, charset),
        DiagramKind::Architecture => architecture::render_styled(source, max_width, charset),
        DiagramKind::Packet => packet::render_styled(source, max_width, charset),
        DiagramKind::Unknown => Err(AsciiRenderError::Unsupported(format!("{kind:?}"))),
    };
    match result {
        Ok(rows) => rows,
        Err(err) => fallback_source_styled(source, &err.to_string()),
    }
}

pub fn render_mermaid(source: &str, max_width: u16) -> Vec<String> {
    render_mermaid_styled(source, max_width)
        .into_iter()
        .map(flatten_row)
        .collect()
}

fn flatten_row(row: StyledRow) -> String {
    row.into_iter()
        .map(|r| r.text)
        .collect::<String>()
        .trim_end()
        .to_string()
}

pub fn plain_row(text: impl Into<String>) -> StyledRow {
    let text = text.into();
    if text.is_empty() {
        return Vec::new();
    }
    vec![StyledRun { text, color: None }]
}

pub fn styled_row(text: impl Into<String>, color: Color) -> StyledRow {
    let text = text.into();
    if text.is_empty() {
        return Vec::new();
    }
    vec![StyledRun {
        text,
        color: Some(color),
    }]
}

pub fn lines_to_styled(lines: Vec<String>) -> Vec<StyledRow> {
    lines.into_iter().map(plain_row).collect()
}

fn fallback_source_styled(source: &str, reason: &str) -> Vec<StyledRow> {
    let mut out: Vec<StyledRow> = Vec::with_capacity(source.lines().count() + 3);
    out.push(styled_row(format!("// mermaid: {reason}"), Color::DarkGray));
    out.push(plain_row("```mermaid".to_string()));
    for line in source.lines() {
        out.push(plain_row(line.to_string()));
    }
    out.push(plain_row("```".to_string()));
    out
}
