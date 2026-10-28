use async_trait::async_trait;

use crate::env::Secrets;
use crate::providers::options::{option_env_string, option_string};
use crate::providers::{Provider, ProviderContext};
use crate::vault::{FatimaVault, VaultOptions};
use crate::{FatimaError, Result};

pub struct FatimaProvider;

#[async_trait(?Send)]
impl Provider for FatimaProvider {
    fn name(&self) -> &'static str {
        "fatima"
    }

    async fn load(&self, context: ProviderContext<'_>) -> Result<Secrets> {
        let source = option_string(context.options, "source").unwrap_or_else(|| "e2e".to_string());
        match source.as_str() {
            "e2e" => load_e2e(context),
            "cloud" => Err(FatimaError::message(
                "fatima provider source `cloud` is not implemented yet.",
            )),
            value => Err(FatimaError::message(format!(
                "invalid fatima provider source `{value}`, expected cloud or e2e"
            ))),
        }
    }
}

fn load_e2e(context: ProviderContext<'_>) -> Result<Secrets> {
    let key = option_env_string(context.options, context.env, "key", "FATIMA_USER_KEY")
        .ok_or_else(|| FatimaError::message("fatima provider source `e2e` requires a user key. Set `key`, export FATIMA_USER_KEY, or add a file provider before fatima to load it from .env."))?;
    let environment = option_string(context.options, "environment")
        .unwrap_or_else(|| context.environment.to_string());
    let vault_cwd =
        option_string(context.options, "cwd").unwrap_or_else(|| context.cwd.display().to_string());
    let options = VaultOptions {
        cwd: Some(vault_cwd),
        store_path: option_string(context.options, "store-path")
            .or_else(|| option_string(context.options, "storePath")),
    };
    FatimaVault::authenticate_with_user_key(&key, Some(options))?.get_secrets(Some(&environment))
}
