#[derive(Debug, Clone)]
pub struct PacketField {
    pub start_bit: u32,
    pub end_bit: u32,
    pub label: String,
}

#[derive(Debug, Clone, Default)]
pub struct PacketDiagram {
    pub title: Option<String>,
    pub bits_per_row: u32,
    pub fields: Vec<PacketField>,
}
