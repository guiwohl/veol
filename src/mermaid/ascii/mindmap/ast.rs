#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeShape {
    Default,
    Square,
    Round,
    Circle,
    Cloud,
    Bang,
    Hexagon,
}

#[derive(Debug, Clone)]
pub struct MindNode {
    pub label: String,
    pub shape: NodeShape,
    pub icon: Option<String>,
    pub children: Vec<MindNode>,
}

#[derive(Debug, Clone, Default)]
pub struct Mindmap {
    pub root: Option<MindNode>,
}
