use super::Secrets;
use crate::{FatimaError, Result};

#[derive(Clone, Copy, Debug)]
pub enum SecretFormat {
    Dotenv,
    Json,
    Yml,
    BashExport,
}

impl SecretFormat {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "dotenv" => Ok(Self::Dotenv),
            "json" => Ok(Self::Json),
            "yml" => Ok(Self::Yml),
            "bash-export" => Ok(Self::BashExport),
            _ => Err(FatimaError::message(
                "invalid format, expected dotenv, json, yml, or bash-export",
            )),
        }
    }
}

pub fn format_secrets(secrets: &Secrets, format: SecretFormat) -> Result<String> {
    match format {
        SecretFormat::Dotenv => Ok(secrets
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("\n")),
        SecretFormat::Json => serde_json::to_string_pretty(secrets)
            .map_err(|error| FatimaError::message(error.to_string())),
        SecretFormat::Yml => {
            serde_yaml::to_string(secrets).map_err(|error| FatimaError::message(error.to_string()))
        }
        SecretFormat::BashExport => Ok(secrets
            .iter()
            .map(|(key, value)| format!("export {key}={}", shell_quote(value)))
            .collect::<Vec<_>>()
            .join("\n")),
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
