use std::path::PathBuf;

use serde::Serialize;
use serde_json::Value;

use crate::config::{evaluate_environment_expression, interpolate_value};
use crate::env::Secrets;
use crate::host::FatimaHost;
#[cfg(feature = "native")]
use crate::host::NativeHost;
use crate::providers::load_provider;
use crate::{models, FatimaConfig, FatimaError, ModelConfig, ProviderConfig, Result};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedEnvironment {
    pub env: Secrets,
    pub environment: String,
    pub loaded_env: Secrets,
    pub providers_used: usize,
}

#[derive(Clone, Debug)]
pub struct Fatima {
    cwd: PathBuf,
    config: FatimaConfig,
}

impl Default for Fatima {
    fn default() -> Self {
        Self::new()
    }
}

impl Fatima {
    pub fn new() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            config: FatimaConfig::default(),
        }
    }

    pub fn from_config(config: FatimaConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = cwd.into();
        self
    }

    pub fn add_provider(
        mut self,
        environment: impl Into<String>,
        provider: ProviderConfig,
    ) -> Self {
        self.config
            .providers
            .entry(environment.into())
            .or_default()
            .push(provider);
        self
    }

    pub fn set_model(mut self, model: ModelConfig) -> Self {
        self.config.model = model;
        self
    }

    pub fn set_public_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.config.public_prefix = Some(prefix.into());
        self
    }

    #[cfg(feature = "native")]
    pub fn secrets(&self, environment: impl AsRef<str>) -> Result<Secrets> {
        futures::executor::block_on(self.secrets_with_host(environment, &NativeHost))
    }

    pub async fn secrets_with_host(
        &self,
        environment: impl AsRef<str>,
        host: &dyn FatimaHost,
    ) -> Result<Secrets> {
        Ok(self
            .load_environment_with_host(Some(environment.as_ref()), false, host)
            .await?
            .loaded_env)
    }

    #[cfg(feature = "native")]
    pub fn validate(&self, environment: impl AsRef<str>) -> Result<()> {
        futures::executor::block_on(self.validate_with_host(environment, &NativeHost))
    }

    pub async fn validate_with_host(
        &self,
        environment: impl AsRef<str>,
        host: &dyn FatimaHost,
    ) -> Result<()> {
        let secrets = self
            .load_environment_with_host(Some(environment.as_ref()), false, host)
            .await?
            .env;
        models::validate_environment(&self.config.model, &secrets)
    }

    #[cfg(feature = "native")]
    pub fn load_environment(
        &self,
        environment: Option<&str>,
        use_process_env: bool,
    ) -> Result<LoadedEnvironment> {
        futures::executor::block_on(self.load_environment_with_host(
            environment,
            use_process_env,
            &NativeHost,
        ))
    }

    pub async fn load_environment_with_host(
        &self,
        environment: Option<&str>,
        use_process_env: bool,
        host: &dyn FatimaHost,
    ) -> Result<LoadedEnvironment> {
        let base_env: Secrets = host.env_vars().await?;
        let environment = match environment {
            Some(value) => value.to_string(),
            None => evaluate_environment_expression(
                self.config
                    .environment
                    .as_deref()
                    .unwrap_or("'development'"),
                &base_env,
            )?,
        };

        if environment.is_empty() {
            return Err(FatimaError::message(
                "fatima.json `environment` resolved to an empty value. Return a concrete environment name.",
            ));
        }

        if use_process_env {
            return Ok(LoadedEnvironment {
                env: base_env.clone(),
                environment,
                loaded_env: base_env,
                providers_used: 0,
            });
        }

        let Some(providers) = self.config.providers.get(&environment) else {
            return Ok(LoadedEnvironment {
                env: base_env.clone(),
                environment,
                loaded_env: base_env,
                providers_used: 0,
            });
        };

        if providers.is_empty() {
            return Ok(LoadedEnvironment {
                env: base_env.clone(),
                environment,
                loaded_env: base_env,
                providers_used: 0,
            });
        }

        let cwd = if self.config.config_file.folder_path.is_empty() {
            self.cwd.clone()
        } else {
            PathBuf::from(&self.config.config_file.folder_path)
        };
        let mut current_env = base_env;
        let mut loaded_env = Secrets::new();

        for provider in providers {
            let provider = interpolate_provider(provider, &current_env)?;
            let next_env = load_provider(&cwd, &environment, &provider, &current_env, host).await?;
            current_env.extend(next_env.clone());
            loaded_env.extend(next_env);
        }

        let resolved_environment = evaluate_environment_expression(
            self.config
                .environment
                .as_deref()
                .unwrap_or("'development'"),
            &current_env,
        )?;

        if resolved_environment != environment {
            return Err(FatimaError::message(format!(
                "Environment changed while loading from providers: started with {environment} and ended as {resolved_environment}."
            )));
        }

        Ok(LoadedEnvironment {
            env: current_env,
            environment,
            loaded_env,
            providers_used: providers.len(),
        })
    }
}

fn interpolate_provider(provider: &ProviderConfig, env: &Secrets) -> Result<ProviderConfig> {
    let value = Value::Object(
        provider
            .options
            .clone()
            .into_iter()
            .collect::<serde_json::Map<_, _>>(),
    );
    let Value::Object(options) = interpolate_value(value, env)? else {
        unreachable!("provider options should remain an object");
    };
    Ok(ProviderConfig {
        provider: provider.provider.clone(),
        options: options.into_iter().collect(),
    })
}
