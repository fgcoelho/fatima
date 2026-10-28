use async_trait::async_trait;

use crate::env::Secrets;
use crate::providers::{Provider, ProviderContext};
use crate::Result;

pub struct EnvProvider;

#[async_trait(?Send)]
impl Provider for EnvProvider {
    fn name(&self) -> &'static str {
        "env"
    }

    async fn load(&self, context: ProviderContext<'_>) -> Result<Secrets> {
        Ok(context.env.clone())
    }
}
