#[derive(Debug, Clone)]
pub struct TimelineEvent {
    pub period: String,
    pub events: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TimelineSection {
    pub name: String,
    pub entries: Vec<TimelineEvent>,
}

#[derive(Debug, Clone, Default)]
pub struct Timeline {
    pub title: Option<String>,
    pub sections: Vec<TimelineSection>,
}
