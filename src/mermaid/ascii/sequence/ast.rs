#[derive(Debug, Clone)]
pub struct Participant {
    pub id: String,
    pub label: String,
    pub index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowKind {
    Solid,
    Dotted,
    AsyncSolid,
    AsyncDotted,
    SolidCross,
    DottedCross,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub from: usize,
    pub to: usize,
    pub label: String,
    pub arrow: ArrowKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotePosition {
    LeftOf,
    RightOf,
    Over,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub position: NotePosition,
    pub participants: Vec<usize>,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Loop,
    Alt,
    Opt,
    Par,
    Critical,
    Break,
}

#[derive(Debug, Clone)]
pub struct BlockBranch {
    pub label: String,
    pub items: Vec<DiagramItem>,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub kind: BlockKind,
    pub label: String,
    pub branches: Vec<BlockBranch>,
}

#[derive(Debug, Clone)]
pub enum DiagramItem {
    Message(Message),
    Note(Note),
    Activate(usize),
    Deactivate(usize),
    Block(Block),
}

#[derive(Debug, Clone, Default)]
pub struct SequenceDiagram {
    pub participants: Vec<Participant>,
    pub messages: Vec<Message>,
    pub items: Vec<DiagramItem>,
    pub autonumber: bool,
}
