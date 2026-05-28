#[derive(Debug, Clone)]
pub struct QuadrantPoint {
    pub label: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Default)]
pub struct QuadrantChart {
    pub title: Option<String>,
    pub x_axis_left: Option<String>,
    pub x_axis_right: Option<String>,
    pub y_axis_bottom: Option<String>,
    pub y_axis_top: Option<String>,
    pub q1_label: Option<String>,
    pub q2_label: Option<String>,
    pub q3_label: Option<String>,
    pub q4_label: Option<String>,
    pub points: Vec<QuadrantPoint>,
}
