use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
#[cfg(feature = "native")]
use std::process::{Command, Stdio};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::env::Secrets;
#[cfg(feature = "native")]
use crate::FatimaError;
use crate::Result;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub query: Vec<(String, String)>,
    pub body: Option<Value>,
    pub bearer: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommandRequest {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Secrets,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

#[async_trait(?Send)]
pub trait FatimaHost {
    async fn env_vars(&self) -> Result<Secrets>;
    async fn env_var(&self, key: &str) -> Result<Option<String>>;
    async fn current_dir(&self) -> Result<PathBuf>;
    async fn exists(&self, path: &Path) -> Result<bool>;
    async fn read_to_string(&self, path: &Path) -> Result<String>;
    async fn write_string(&self, path: &Path, content: &str) -> Result<()>;
    async fn create_dir_all(&self, path: &Path) -> Result<()>;
    async fn remove_file(&self, path: &Path) -> Result<()>;
    async fn process_id(&self) -> Result<u32>;
    async fn run_command(&self, request: CommandRequest) -> Result<CommandOutput>;
    async fn http_json(&self, request: HttpRequest) -> Result<Value>;
}

#[cfg(feature = "native")]
#[derive(Clone, Debug, Default)]
pub struct NativeHost;

#[cfg(feature = "native")]
#[async_trait(?Send)]
impl FatimaHost for NativeHost {
    async fn env_vars(&self) -> Result<Secrets> {
        Ok(std::env::vars().collect())
    }

    async fn env_var(&self, key: &str) -> Result<Option<String>> {
        Ok(std::env::var(key).ok())
    }

    async fn current_dir(&self) -> Result<PathBuf> {
        std::env::current_dir().map_err(|error| FatimaError::message(error.to_string()))
    }

    async fn exists(&self, path: &Path) -> Result<bool> {
        Ok(path.exists())
    }

    async fn read_to_string(&self, path: &Path) -> Result<String> {
        std::fs::read_to_string(path).map_err(|source| FatimaError::ReadFile {
            path: path.display().to_string(),
            source,
        })
    }

    async fn write_string(&self, path: &Path, content: &str) -> Result<()> {
        std::fs::write(path, content).map_err(|source| FatimaError::WriteFile {
            path: path.display().to_string(),
            source,
        })
    }

    async fn create_dir_all(&self, path: &Path) -> Result<()> {
        std::fs::create_dir_all(path).map_err(|source| FatimaError::WriteFile {
            path: path.display().to_string(),
            source,
        })
    }

    async fn remove_file(&self, path: &Path) -> Result<()> {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(FatimaError::WriteFile {
                path: path.display().to_string(),
                source,
            }),
        }
    }

    async fn process_id(&self) -> Result<u32> {
        Ok(std::process::id())
    }

    async fn run_command(&self, request: CommandRequest) -> Result<CommandOutput> {
        let output = Command::new(&request.command)
            .current_dir(&request.cwd)
            .stdin(Stdio::null())
            .args(&request.args)
            .envs(&request.env)
            .output()
            .map_err(|error| {
                FatimaError::message(format!("failed to run {}: {error}", request.command))
            })?;
        Ok(CommandOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    async fn http_json(&self, request: HttpRequest) -> Result<Value> {
        let client = reqwest::Client::new();
        let method = request
            .method
            .parse()
            .map_err(|error| FatimaError::message(format!("invalid HTTP method: {error}")))?;
        let mut builder = client.request(method, request.url).query(&request.query);
        for (key, value) in request.headers {
            builder = builder.header(key, value);
        }
        if let Some(token) = request.bearer {
            builder = builder.bearer_auth(token);
        }
        if let Some(body) = request.body {
            builder = builder.json(&body);
        }
        builder
            .send()
            .await
            .and_then(|response| response.error_for_status())
            .map_err(|error| FatimaError::message(format!("HTTP request failed: {error}")))?
            .json()
            .await
            .map_err(|error| {
                FatimaError::message(format!("failed to parse HTTP response: {error}"))
            })
    }
}
