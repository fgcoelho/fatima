use std::collections::BTreeMap;
use std::path::Path;

use async_trait::async_trait;
use serde_json::Value;

use crate::env::Secrets;
use crate::host::FatimaHost;
use crate::Result;

pub struct ProviderContext<'a> {
    pub cwd: &'a Path,
    pub environment: &'a str,
    pub env: &'a Secrets,
    pub options: &'a BTreeMap<String, Value>,
    pub host: &'a dyn FatimaHost,
}

#[async_trait(?Send)]
pub trait Provider {
    fn name(&self) -> &'static str;
    async fn load(&self, context: ProviderContext<'_>) -> Result<Secrets>;
}
