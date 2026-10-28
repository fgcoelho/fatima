use async_trait::async_trait;
use serde::Deserialize;

use crate::env::Secrets;
use crate::host::{FatimaHost, HttpRequest};
use crate::providers::options::{option_env_string, option_string};
use crate::providers::{Provider, ProviderContext};
use crate::{FatimaError, Result};

pub struct InfisicalProvider;

#[async_trait(?Send)]
impl Provider for InfisicalProvider {
    fn name(&self) -> &'static str {
        "infisical"
    }

    async fn load(&self, context: ProviderContext<'_>) -> Result<Secrets> {
        let client_id = required(
            option_env_string(
                context.options,
                context.env,
                "client-id",
                "INFISICAL_CLIENT_ID",
            ),
            "INFISICAL_CLIENT_ID",
        )?;
        let client_secret = required(
            option_env_string(
                context.options,
                context.env,
                "client-secret",
                "INFISICAL_CLIENT_SECRET",
            ),
            "INFISICAL_CLIENT_SECRET",
        )?;
        let project_id = required(
            option_env_string(
                context.options,
                context.env,
                "project-id",
                "INFISICAL_PROJECT_ID",
            ),
            "INFISICAL_PROJECT_ID",
        )?;
        let site_url = option_env_string(
            context.options,
            context.env,
            "site-url",
            "INFISICAL_SITE_URL",
        )
        .unwrap_or_else(|| "https://app.infisical.com".to_string());

        let environment = option_string(context.options, "environment")
            .or_else(|| context.env.get("INFISICAL_ENVIRONMENT").cloned())
            .or(context.host.env_var("INFISICAL_ENVIRONMENT").await?)
            .unwrap_or_else(|| "dev".to_string());

        load_from_api(
            context.host,
            &site_url,
            &client_id,
            &client_secret,
            &project_id,
            &environment,
        )
        .await
    }
}

async fn load_from_api(
    host: &dyn FatimaHost,
    site_url: &str,
    client_id: &str,
    client_secret: &str,
    project_id: &str,
    environment: &str,
) -> Result<Secrets> {
    let base = site_url.trim_end_matches('/');
    let auth: LoginResponse = serde_json::from_value(
        host.http_json(HttpRequest {
            method: "POST".to_string(),
            url: format!("{base}/api/v1/auth/universal-auth/login"),
            headers: Default::default(),
            query: Vec::new(),
            body: Some(serde_json::json!({ "clientId": client_id, "clientSecret": client_secret })),
            bearer: None,
        })
        .await?,
    )
    .map_err(|error| {
        FatimaError::message(format!("failed to parse Infisical auth response: {error}"))
    })?;

    let secrets: SecretsResponse = serde_json::from_value(
        host.http_json(HttpRequest {
            method: "GET".to_string(),
            url: format!("{base}/api/v3/secrets/raw"),
            headers: Default::default(),
            query: vec![
                ("workspaceId".to_string(), project_id.to_string()),
                ("environment".to_string(), environment.to_string()),
                ("secretPath".to_string(), "/".to_string()),
            ],
            body: None,
            bearer: Some(auth.access_token),
        })
        .await?,
    )
    .map_err(|error| {
        FatimaError::message(format!(
            "failed to parse Infisical secrets response: {error}"
        ))
    })?;

    Ok(secrets
        .secrets
        .into_iter()
        .map(|secret| (secret.secret_key, secret.secret_value))
        .collect())
}

fn required(value: Option<String>, name: &str) -> Result<String> {
    value.ok_or_else(|| FatimaError::message(format!("Missing configuration: {name}")))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct SecretsResponse {
    secrets: Vec<InfisicalSecret>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InfisicalSecret {
    secret_key: String,
    secret_value: String,
}
