use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::defaults::{default_generator, default_generator_file};

pub type ModelConfig = BTreeMap<String, Value>;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FatimaConfig {
    #[serde(default = "default_generator")]
    pub generator: String,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub formatter: Option<String>,
    #[serde(default)]
    pub environment: Option<String>,
    #[serde(default)]
    pub providers: BTreeMap<String, Vec<ProviderConfig>>,
    #[serde(default)]
    pub model: ModelConfig,
    #[serde(default, alias = "public-prefix")]
    pub public_prefix: Option<String>,
    #[serde(default)]
    pub plugins: Vec<String>,
    #[serde(default)]
    pub config_file: ConfigFile,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProviderConfig {
    pub provider: String,
    #[serde(flatten)]
    pub options: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ConfigFile {
    pub path: String,
    pub folder_path: String,
}

impl ProviderConfig {
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            options: BTreeMap::new(),
        }
    }
}

impl FatimaConfig {
    pub fn normalized(mut self) -> Self {
        if self.generator.is_empty() {
            self.generator = default_generator();
        }
        if self.file.is_none() {
            self.file = Some(default_generator_file(&self.generator).to_string());
        }
        if self.environment.is_none() {
            self.environment = Some("'development'".to_string());
        }
        self
    }
}
