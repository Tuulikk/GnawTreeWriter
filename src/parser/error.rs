use serde::{Serialize, Deserialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, Error)]
#[error("Syntax error at line {line}, col {column}: {message}")]
pub struct SyntaxError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub expected: Option<String>,
}

pub type ParseResult<T> = std::result::Result<T, SyntaxError>;

impl From<anyhow::Error> for SyntaxError {
    fn from(err: anyhow::Error) -> Self {
        SyntaxError {
            message: err.to_string(),
            line: 0,
            column: 0,
            expected: None,
        }
    }
}

impl From<tree_sitter::LanguageError> for SyntaxError {
    fn from(err: tree_sitter::LanguageError) -> Self {
        SyntaxError {
            message: err.to_string(),
            line: 0,
            column: 0,
            expected: None,
        }
    }
}