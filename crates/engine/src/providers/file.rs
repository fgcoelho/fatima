use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::env::{parse_env, Secrets};
use crate::providers::options::{option_string, option_string_vec};
use crate::providers::{Provider, ProviderContext};
use crate::{FatimaError, Result};

pub struct FileProvider;

#[async_trait(?Send)]
impl Provider for FileProvider {
    fn name(&self) -> &'static str {
        "file"
    }

    async fn load(&self, context: ProviderContext<'_>) -> Result<Secrets> {
        let files = option_string_vec(context.options, "files")
            .or_else(|| option_string_vec(context.options, "file"))
            .unwrap_or_else(|| vec![".env".to_string()]);
        let format = option_string(context.options, "format");

        let mut env = Secrets::new();
        for file in files {
            let path = resolve_path(context.cwd, &file);
            let format = format
                .as_deref()
                .map(ToOwned::to_owned)
                .or_else(|| infer_format(&path))
                .unwrap_or_else(|| "env".to_string());

            if format != "env" {
                return Err(FatimaError::message(format!(
                    "file provider format `{format}` is not supported yet. Supported formats: env."
                )));
            }

            let content = context.host.read_to_string(&path).await?;
            env.extend(parse_env(&content));
        }
        Ok(env)
    }
}

fn infer_format(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_str()?;
    if file_name == ".env" || file_name.starts_with(".env.") {
        return Some("env".to_string());
    }
    match path.extension()?.to_str()? {
        "env" => Some("env".to_string()),
        "json" => Some("json".to_string()),
        "yaml" | "yml" => Some("yaml".to_string()),
        _ => None,
    }
}

fn resolve_path(cwd: &Path, file: &str) -> PathBuf {
    let path = PathBuf::from(file);
    if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    }
}
