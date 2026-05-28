use crate::mermaid::ascii::detect::FlowDirection;
use crate::mermaid::ascii::label::Label;
use indexmap::IndexMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeShape {
    Square,
    Round,
    Stadium,
    Subroutine,
    Cylinder,
    Circle,
    Asymmetric,
    Rhombus,
    Hexagon,
    Parallelogram,
    ParallelogramAlt,
    Trapezoid,
    TrapezoidAlt,
    DoubleCircle,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub label: Label,
    pub shape: NodeShape,
    pub class_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeStyle {
    Solid,
    Dotted,
    Thick,
    Invisible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeArrow {
    Open,
    Closed,
    Circle,
    Cross,
    Bidirectional,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub label: String,
    pub style: EdgeStyle,
    pub arrow: EdgeArrow,
}

#[derive(Debug, Clone, Default)]
pub struct Subgraph {
    pub id: String,
    pub title: String,
    pub node_ids: Vec<String>,
    pub child_subgraphs: Vec<String>,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ClassDef {
    pub name: String,
    pub styles: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Flowchart {
    pub direction: FlowDirection,
    pub nodes: IndexMap<String, Node>,
    pub edges: Vec<Edge>,
    pub subgraphs: IndexMap<String, Subgraph>,
    pub class_defs: std::collections::HashMap<String, ClassDef>,
}

impl Default for Flowchart {
    fn default() -> Self {
        Self {
            direction: FlowDirection::TopDown,
            nodes: IndexMap::new(),
            edges: Vec::new(),
            subgraphs: IndexMap::new(),
            class_defs: std::collections::HashMap::new(),
        }
    }
}
