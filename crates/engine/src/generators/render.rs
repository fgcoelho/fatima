use super::js_ts::{render_javascript, render_typescript};
use super::paths::sibling_public_path;
use super::python::render_python;
use crate::env::Secrets;
use crate::{FatimaConfig, FatimaError, Result};

pub fn render_files(
    config: &FatimaConfig,
    file: &str,
    loaded: &Secrets,
) -> Result<Vec<(String, String)>> {
    match config.generator.as_str() {
        "typescript" => {
            let public_path = sibling_public_path(file);
            Ok(vec![
                (file.to_string(), render_typescript(config, loaded, false)?),
                (
                    public_path.clone(),
                    render_typescript(config, &public_keys(config, loaded), true)?,
                ),
            ])
        }
        "javascript" => {
            let public_path = sibling_public_path(file);
            Ok(vec![
                (file.to_string(), render_javascript(config, loaded, false)?),
                (
                    public_path.clone(),
                    render_javascript(config, &public_keys(config, loaded), true)?,
                ),
            ])
        }
        "python" => Ok(vec![(file.to_string(), render_python(config, loaded))]),
        other => Err(FatimaError::message(format!(
            "Unknown Fatima generator: {other}"
        ))),
    }
}

fn public_keys(config: &FatimaConfig, loaded: &Secrets) -> Secrets {
    let prefix = config.public_prefix.as_deref().unwrap_or("PUBLIC_");
    loaded
        .iter()
        .filter(|(key, _)| key.starts_with(prefix))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}
