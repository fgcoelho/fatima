use crate::env::Secrets;
use crate::{FatimaError, Result};

use super::types::SecretRecord;

pub(crate) fn to_environment_map(secrets: &[SecretRecord], environment: &str) -> Secrets {
    secrets
        .iter()
        .filter_map(|secret| {
            secret
                .values
                .get(environment)
                .map(|value| (secret.key.clone(), value.clone()))
        })
        .collect()
}

pub(crate) fn assert_environment(environments: &[String], environment: &str) -> Result<()> {
    if !environments.iter().any(|value| value == environment) {
        return Err(FatimaError::message(format!(
            "Unknown Fatima environment: {environment}"
        )));
    }
    Ok(())
}

pub(crate) fn assert_non_empty_password(password: &str) -> Result<()> {
    if password.trim().is_empty() {
        return Err(FatimaError::message(
            "Fatima secret manager password cannot be empty.",
        ));
    }
    Ok(())
}

pub(crate) fn dedupe(values: Vec<String>) -> Vec<String> {
    values.into_iter().fold(Vec::new(), |mut output, value| {
        if !output.contains(&value) {
            output.push(value);
        }
        output
    })
}
