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
    #[error("wrong master password")]
    WrongMasterPassword,
    #[error("failed to decrypt Fatima vault data")]
    Decrypt,
    #[error("Fatima vault key derivation parameters are out of range")]
    InvalidKdfParams,
    #[error("Fatima vault version {version} is not supported")]
    UnsupportedVaultVersion { version: u8 },
}

impl FatimaError {
    pub fn message(value: impl Into<String>) -> Self {
        Self::Message(value.into())
    }
}
