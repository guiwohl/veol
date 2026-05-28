use indexmap::IndexMap;

#[derive(Debug, Clone)]
pub struct ArchGroup {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub parent: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ArchService {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone)]
pub struct ArchEdge {
    pub from: String,
    pub from_side: Side,
    pub to: String,
    pub to_side: Side,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct ArchJunction {
    pub id: String,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ArchitectureDiagram {
    pub groups: IndexMap<String, ArchGroup>,
    pub services: IndexMap<String, ArchService>,
    pub junctions: IndexMap<String, ArchJunction>,
    pub edges: Vec<ArchEdge>,
    pub decl_order: Vec<String>,
}
