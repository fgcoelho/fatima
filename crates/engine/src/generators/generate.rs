use std::path::PathBuf;

use serde::Serialize;

use super::render::render_files;
use crate::env::Secrets;
use crate::host::FatimaHost;
#[cfg(feature = "native")]
use crate::host::NativeHost;
use crate::{Fatima, FatimaConfig, Result};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateOutput {
    pub output_path: String,
    pub environment: String,
    pub loaded_env: Secrets,
}

#[cfg(feature = "native")]
pub fn generate(
    config: FatimaConfig,
    environment: Option<&str>,
    strict: bool,
) -> Result<GenerateOutput> {
    futures::executor::block_on(generate_with_host(config, environment, strict, &NativeHost))
}

pub async fn generate_with_host(
    config: FatimaConfig,
    environment: Option<&str>,
    strict: bool,
    host: &dyn FatimaHost,
) -> Result<GenerateOutput> {
    let fatima = Fatima::from_config(config.clone()).cwd(host.current_dir().await?);
    let loaded = fatima
        .load_environment_with_host(environment, false, host)
        .await?;

    if strict {
        fatima.validate_with_host(&loaded.environment, host).await?;
    }

    let folder = PathBuf::from(&config.config_file.folder_path);
    let file = config
        .file
        .clone()
        .unwrap_or_else(|| default_file(&config.generator).to_string());
    let files = render_files(&config, &file, &loaded.loaded_env)?;
    let first = files.first().map(|(path, _)| path.clone()).unwrap_or(file);

    for (path, content) in files {
        let output = folder.join(path);
        if let Some(parent) = output.parent() {
            host.create_dir_all(parent).await?;
        }
        host.write_string(&output, &content).await?;
    }

    Ok(GenerateOutput {
        output_path: folder.join(first).display().to_string(),
        environment: loaded.environment,
        loaded_env: loaded.loaded_env,
    })
}

fn default_file(generator: &str) -> &'static str {
    match generator {
        "typescript" => "env.ts",
        "javascript" => "env.js",
        "python" => "env.py",
        _ => "env",
    }
}
