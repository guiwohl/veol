#[derive(Debug, Clone)]
pub enum Series {
    Line(Vec<f64>),
    Bar(Vec<f64>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Default)]
pub struct XyChart {
    pub title: Option<String>,
    pub orientation: Option<Orientation>,
    pub x_categories: Vec<String>,
    pub x_range: Option<(f64, f64)>,
    pub y_range: Option<(f64, f64)>,
    pub x_axis_title: Option<String>,
    pub y_axis_title: Option<String>,
    pub series: Vec<Series>,
}
