pub mod ast;
pub mod parser;
pub mod render;

use super::{AsciiRenderError, CharsetKind, StyledRow};

pub fn render(
    source: &str,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    let diag = parser::parse(source)?;
    render::render(&diag, max_width, charset)
}

pub fn render_styled(
    source: &str,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    let diag = parser::parse(source)?;
    render::render_styled(&diag, max_width, charset)
}
