mod api;
pub mod config;
pub mod env;
mod error;
pub mod generators;
pub mod host;
pub mod models;
pub mod providers;
mod runtime;
pub mod vault;

#[cfg(feature = "native")]
pub use api::handle_api_request;
pub use api::{handle_api_request_with_host, ApiRequest, ApiResponse};
pub use config::{FatimaConfig, ModelConfig, ProviderConfig};
pub use env::{format_secrets, SecretFormat, Secrets};
pub use error::{FatimaError, Result};
#[cfg(feature = "native")]
pub use generators::generate;
pub use generators::{generate_with_host, GenerateOutput};
#[cfg(feature = "native")]
pub use host::NativeHost;
pub use host::{CommandOutput, CommandRequest, FatimaHost, HttpRequest};
pub use runtime::{Fatima, LoadedEnvironment};
pub use vault::FatimaVault;
