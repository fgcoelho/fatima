use std::fs;
use std::path::Path;

use crate::{FatimaError, Result};

use super::normalize::normalize_config;
use super::paths::resolve_config_path;
use super::schema::FatimaConfig;

impl FatimaConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = resolve_config_path(Some(path.as_ref()))?;
        let content = fs::read_to_string(&path).map_err(|source| FatimaError::ReadFile {
            path: path.display().to_string(),
            source,
        })?;

        let raw = serde_json::from_str(&content).map_err(|source| FatimaError::ParseJson {
            path: path.display().to_string(),
            source,
        })?;

        normalize_config(raw, &path)
    }

    pub fn load_default() -> Result<Self> {
        let path = resolve_config_path(None)?;
        Self::load(path)
    }
}
