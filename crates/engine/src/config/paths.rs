use std::path::{Path, PathBuf};

use crate::{FatimaError, Result};

pub fn resolve_config_path(config_path: Option<&Path>) -> Result<PathBuf> {
    let cwd = std::env::current_dir().map_err(|error| FatimaError::message(error.to_string()))?;
    let path = config_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("fatima.json"));

    if config_path.is_some() {
        match path.extension().and_then(|value| value.to_str()) {
            Some("json") => {}
            Some(ext) => {
                return Err(FatimaError::message(format!(
                    "Invalid given config file extension: .{ext}"
                )))
            }
            None => {
                return Err(FatimaError::message(format!(
                    "No extension found in given config file path: {}",
                    path.display()
                )))
            }
        }
    }

    let resolved = if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    };

    if !resolved.exists() {
        return Err(FatimaError::message(format!(
            "Config file not found: {}\n\nFatima requires a fatima.json file. Run `fatima init` in your project root before using Fatima.",
            resolved.display()
        )));
    }

    Ok(resolved)
}
