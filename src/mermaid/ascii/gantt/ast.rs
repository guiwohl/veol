#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Default,
    Active,
    Done,
    Crit,
    Milestone,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub label: String,
    pub status: TaskStatus,
    pub start_day: i64,
    pub duration_days: i64,
    pub depends_on: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Section {
    pub name: String,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Default)]
pub struct GanttChart {
    pub title: Option<String>,
    pub date_format: Option<String>,
    pub axis_format: Option<String>,
    pub sections: Vec<Section>,
}
