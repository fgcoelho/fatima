use async_trait::async_trait;

use crate::env::{parse_env, Secrets};
use crate::host::CommandRequest;
use crate::providers::options::{option_env_string, option_string};
use crate::providers::{Provider, ProviderContext};
use crate::{FatimaError, Result};

pub struct VercelProvider;

#[async_trait(?Send)]
impl Provider for VercelProvider {
    fn name(&self) -> &'static str {
        "vercel"
    }

    async fn load(&self, context: ProviderContext<'_>) -> Result<Secrets> {
        let org_id = option_env_string(
            context.options,
            context.env,
            "vercel-org-id",
            "VERCEL_ORG_ID",
        );
        let project_id = option_env_string(
            context.options,
            context.env,
            "vercel-project-id",
            "VERCEL_PROJECT_ID",
        );
        let token = option_env_string(context.options, context.env, "vercel-token", "VERCEL_TOKEN");
        let environment = option_string(context.options, "environment")
            .or_else(|| context.env.get("VERCEL_ENVIRONMENT").cloned())
            .or(context.host.env_var("VERCEL_ENVIRONMENT").await?)
            .unwrap_or_else(|| "development".to_string());

        let has_vercel_project = context.host.exists(&context.cwd.join(".vercel")).await?;
        if !has_vercel_project {
            require(&org_id, "VERCEL_ORG_ID")?;
            require(&project_id, "VERCEL_PROJECT_ID")?;
            require(&token, "VERCEL_TOKEN")?;
        }

        let filename = format!(
            ".tmp.fatima.vercel.{}.env",
            context.host.process_id().await?
        );
        let output_path = context.cwd.join(&filename);
        let mut args = vec![
            "env".to_string(),
            "pull".to_string(),
            filename,
            format!("--environment={environment}"),
        ];
        if !has_vercel_project {
            if let Some(token) = &token {
                args.push(format!("--token={token}"));
            }
        }

        let mut env = context.env.clone();
        if let Some(value) = &org_id {
            env.insert("VERCEL_ORG_ID".to_string(), value.clone());
        }
        if let Some(value) = &project_id {
            env.insert("VERCEL_PROJECT_ID".to_string(), value.clone());
        }
        if let Some(value) = &token {
            env.insert("VERCEL_TOKEN".to_string(), value.clone());
        }

        let output = context
            .host
            .run_command(CommandRequest {
                command: "vercel".to_string(),
                args,
                cwd: context.cwd.to_path_buf(),
                env,
            })
            .await?;
        if !output.success {
            let _ = context.host.remove_file(&output_path).await;
            return Err(FatimaError::message(
                "Failed to pull environment variables from Vercel.",
            ));
        }

        let content = context.host.read_to_string(&output_path).await?;
        let _ = context.host.remove_file(&output_path).await;
        Ok(parse_env(&content))
    }
}

fn require(value: &Option<String>, name: &str) -> Result<()> {
    if value.as_deref().unwrap_or_default().trim().is_empty() {
        return Err(FatimaError::message(format!(
            "Missing configuration: {name}"
        )));
    }
    Ok(())
}
