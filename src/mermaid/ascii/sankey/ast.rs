#[derive(Debug, Clone)]
pub struct SankeyFlow {
    pub source: String,
    pub target: String,
    pub value: f64,
}

#[derive(Debug, Clone, Default)]
pub struct SankeyDiagram {
    pub flows: Vec<SankeyFlow>,
}
