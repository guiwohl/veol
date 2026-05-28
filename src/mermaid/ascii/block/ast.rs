#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockShape {
    Square,
    Round,
    Stadium,
    Rhombus,
    Circle,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub id: String,
    pub label: String,
    pub shape: BlockShape,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct BlockEdge {
    pub from: String,
    pub to: String,
    pub label: String,
    pub arrow: bool,
}

#[derive(Debug, Clone, Default)]
pub struct BlockDiagram {
    pub columns: u32,
    pub blocks: Vec<Block>,
    pub edges: Vec<BlockEdge>,
}
