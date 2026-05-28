#[derive(Debug, Clone)]
pub struct JourneyStep {
    pub label: String,
    pub score: u8,
    pub actors: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct JourneySection {
    pub name: String,
    pub steps: Vec<JourneyStep>,
}

#[derive(Debug, Clone, Default)]
pub struct UserJourney {
    pub title: Option<String>,
    pub sections: Vec<JourneySection>,
}
