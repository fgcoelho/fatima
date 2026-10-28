use std::path::{Path, PathBuf};

use crate::Result;

use super::schema::{ConfigFile, FatimaConfig};

pub fn normalize_config(config: FatimaConfig, path: &Path) -> Result<FatimaConfig> {
    let mut config = config.normalized();
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let folder_path = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    config.config_file = ConfigFile {
        path: path.display().to_string(),
        folder_path: folder_path.display().to_string(),
    };
    Ok(config)
}
