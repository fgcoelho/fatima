use thiserror::Error;

pub type Result<T> = std::result::Result<T, FatimaError>;

#[derive(Debug, Error)]
pub enum FatimaError {
    #[error("{0}")]
    Message(String),
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to parse {path}: {source}")]
    ParseJson {
        path: String,
        source: serde_json::Error,
    },
    #[error("failed to write {path}: {source}")]
    WriteFile {
        path: String,
        source: std::io::Error,
    },
}

impl FatimaError {
    pub fn message(value: impl Into<String>) -> Self {
        Self::Message(value.into())
    }
}
