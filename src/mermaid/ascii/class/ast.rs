use indexmap::IndexMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Package,
}

#[derive(Debug, Clone)]
pub struct Member {
    pub visibility: Option<Visibility>,
    pub name: String,
    pub type_hint: Option<String>,
    pub is_method: bool,
    pub is_static: bool,
    pub is_abstract: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Class {
    pub name: String,
    pub generic: Option<String>,
    pub annotation: Option<String>,
    pub attributes: Vec<Member>,
    pub methods: Vec<Member>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    Inheritance,
    Composition,
    Aggregation,
    Association,
    Dependency,
    Realization,
    Link,
}

#[derive(Debug, Clone)]
pub struct Relation {
    pub from: String,
    pub to: String,
    pub kind: RelationKind,
    pub label: String,
    pub from_card: String,
    pub to_card: String,
    pub dotted: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ClassDiagram {
    pub classes: IndexMap<String, Class>,
    pub relations: Vec<Relation>,
}
