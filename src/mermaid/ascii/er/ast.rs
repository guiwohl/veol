use indexmap::IndexMap;

#[derive(Debug, Clone)]
pub struct Attribute {
    pub type_name: String,
    pub name: String,
    pub key: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Entity {
    pub name: String,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    ZeroOrOne,
    ExactlyOne,
    ZeroOrMany,
    OneOrMany,
}

#[derive(Debug, Clone)]
pub struct Relationship {
    pub left: String,
    pub right: String,
    pub left_card: Cardinality,
    pub right_card: Cardinality,
    pub identifying: bool,
    pub label: String,
}

#[derive(Debug, Clone, Default)]
pub struct ErDiagram {
    pub entities: IndexMap<String, Entity>,
    pub relationships: Vec<Relationship>,
}
