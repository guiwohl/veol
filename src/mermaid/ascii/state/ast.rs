use indexmap::IndexMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StateKind {
    #[default]
    Normal,
    Start,
    End,
    Choice,
    Fork,
    Join,
    Composite,
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub id: String,
    pub label: String,
    pub kind: StateKind,
    pub children: Vec<String>,
    pub parent: Option<String>,
    pub description: Option<String>,
    pub note: Option<String>,
    pub concurrent_regions: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StateDir {
    #[default]
    TopDown,
    LeftRight,
}

#[derive(Debug, Clone, Default)]
pub struct StateDiagram {
    pub states: IndexMap<String, State>,
    pub transitions: Vec<Transition>,
    pub direction: StateDir,
}
