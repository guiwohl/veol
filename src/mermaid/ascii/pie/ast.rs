#[derive(Debug, Clone)]
pub struct Slice {
    pub label: String,
    pub value: f64,
}

#[derive(Debug, Clone, Default)]
pub struct PieChart {
    pub title: Option<String>,
    pub show_data: bool,
    pub slices: Vec<Slice>,
}
