use indexmap::IndexMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReqType {
    Requirement,
    FunctionalRequirement,
    InterfaceRequirement,
    PerformanceRequirement,
    PhysicalRequirement,
    DesignConstraint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyMethod {
    Analysis,
    Inspection,
    Test,
    Demonstration,
}

#[derive(Debug, Clone, Default)]
pub struct Requirement {
    pub kind: Option<ReqType>,
    pub name: String,
    pub id: Option<String>,
    pub text: Option<String>,
    pub risk: Option<RiskLevel>,
    pub verify: Option<VerifyMethod>,
}

#[derive(Debug, Clone, Default)]
pub struct Element {
    pub name: String,
    pub element_type: Option<String>,
    pub docref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReqRelationKind {
    Contains,
    Copies,
    Derives,
    Satisfies,
    Verifies,
    Refines,
    Traces,
}

#[derive(Debug, Clone)]
pub struct ReqRelation {
    pub from: String,
    pub to: String,
    pub kind: ReqRelationKind,
}

#[derive(Debug, Clone, Default)]
pub struct RequirementDiagram {
    pub requirements: IndexMap<String, Requirement>,
    pub elements: IndexMap<String, Element>,
    pub relations: Vec<ReqRelation>,
}
