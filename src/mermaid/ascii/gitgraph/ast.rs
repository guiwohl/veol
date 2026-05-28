#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitKind {
    Normal,
    Reverse,
    Highlight,
}

#[derive(Debug, Clone)]
pub enum GitOp {
    Commit {
        id: Option<String>,
        kind: CommitKind,
        tag: Option<String>,
    },
    Branch {
        name: String,
        from: Option<String>,
    },
    Checkout {
        branch: String,
    },
    Merge {
        branch: String,
        id: Option<String>,
        tag: Option<String>,
    },
    CherryPick {
        id: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct GitGraph {
    pub orientation: Orientation,
    pub ops: Vec<GitOp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    LeftRight,
    TopDown,
}
