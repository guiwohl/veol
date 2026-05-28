use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum AsciiRenderError {
    #[error("parse error at line {line}: {msg}")]
    Parse { line: usize, msg: String },

    #[error("unsupported diagram type: {0}")]
    Unsupported(String),

    #[error("layout error: {0}")]
    Layout(String),

    #[error("empty diagram")]
    Empty,
}
