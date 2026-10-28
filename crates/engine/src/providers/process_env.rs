use async_trait::async_trait;

use crate::env::Secrets;
use crate::providers::{Provider, ProviderContext};
use crate::Result;

pub struct ProcessEnvProvider;

#[async_trait(?Send)]
impl Provider for ProcessEnvProvider {
    fn name(&self) -> &'static str {
        "process-env"
    }

    async fn load(&self, context: ProviderContext<'_>) -> Result<Secrets> {
        context.host.env_vars().await
    }
}
