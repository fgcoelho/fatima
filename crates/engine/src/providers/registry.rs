use std::path::Path;

use crate::env::Secrets;
use crate::{FatimaError, ProviderConfig, Result};

use super::types::{Provider, ProviderContext};
use super::{env, fatima, file, infisical, process_env, vercel};

pub async fn load_provider(
    cwd: &Path,
    environment: &str,
    provider: &ProviderConfig,
    env: &Secrets,
    host: &dyn crate::host::FatimaHost,
) -> Result<Secrets> {
    let context = ProviderContext {
        cwd,
        environment,
        env,
        options: &provider.options,
        host,
    };

    match provider.provider.as_str() {
        "file" | "local" => file::FileProvider.load(context).await,
        "process-env" => process_env::ProcessEnvProvider.load(context).await,
        "env" => env::EnvProvider.load(context).await,
        "fatima" | "fatima-vault" => fatima::FatimaProvider.load(context).await,
        "infisical" => infisical::InfisicalProvider.load(context).await,
        "vercel" => vercel::VercelProvider.load(context).await,
        name => Err(FatimaError::message(format!("unknown provider: {name}"))),
    }
}
