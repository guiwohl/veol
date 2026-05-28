pub mod ast;
pub mod layout;
pub mod parser;
pub mod render;

use super::{AsciiRenderError, CharsetKind, StyledRow};

pub fn render(
    source: &str,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    let flow = parser::parse(source)?;
    let laid = layout::layout(&flow)?;
    render::render(&flow, &laid, max_width, charset)
}

pub fn render_styled(
    source: &str,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    let flow = parser::parse(source)?;
    let laid = layout::layout(&flow)?;
    render::render_styled(&flow, &laid, max_width, charset)
}
