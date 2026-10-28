use crate::{FatimaError, Result};

use super::legacy::crypto::random_bytes;
use super::domain::assert_environment;
use super::types::KEY_LENGTH;
use super::handle::FatimaVault;

impl FatimaVault {
    pub fn create_environment(&mut self, environment: &str) -> Result<()> {
        let unlocked = self.require_unlocked_mut()?;
        let environment = environment.trim();
        if environment.is_empty() {
            return Err(FatimaError::message("Environment name cannot be empty."));
        }
        if unlocked
            .data
            .environments
            .iter()
            .any(|value| value == environment)
        {
            return Err(FatimaError::message(format!(
                "Fatima environment already exists: {environment}"
            )));
        }
        unlocked.data.environments.push(environment.to_string());
        unlocked
            .environment_secrets
            .insert(environment.to_string(), random_bytes(KEY_LENGTH));
        sync_project(unlocked);
        Ok(())
    }

    pub fn rename_environment(&mut self, current: &str, next: &str) -> Result<()> {
        let unlocked = self.require_unlocked_mut()?;
        let current = current.trim();
        let next = next.trim();
        assert_environment(&unlocked.data.environments, current)?;
        if next.is_empty() {
            return Err(FatimaError::message("Environment name cannot be empty."));
        }
        if current == next {
            return Ok(());
        }
        if unlocked.data.environments.iter().any(|value| value == next) {
            return Err(FatimaError::message(format!(
                "Fatima environment already exists: {next}"
            )));
        }
        for environment in &mut unlocked.data.environments {
            if environment == current {
                *environment = next.to_string();
            }
        }
        for secret in &mut unlocked.data.secrets {
            if let Some(value) = secret.values.remove(current) {
                secret.values.insert(next.to_string(), value);
            }
        }
        if let Some(secret) = unlocked.environment_secrets.remove(current) {
            unlocked
                .environment_secrets
                .insert(next.to_string(), secret);
        }
        for record in &mut unlocked.user_keys {
            if let Some(secret) = record.wrapped_environment_secrets.remove(current) {
                record
                    .wrapped_environment_secrets
                    .insert(next.to_string(), secret);
            }
            if let Some(secret) = record.wrapped_user_environment_secrets.remove(current) {
                record
                    .wrapped_user_environment_secrets
                    .insert(next.to_string(), secret);
            }
        }
        for user_secrets in unlocked.user_environment_secrets.values_mut() {
            if let Some(secret) = user_secrets.remove(current) {
                user_secrets.insert(next.to_string(), secret);
            }
        }
        for secret in &mut unlocked.user_secrets {
            if let Some(value) = secret.values.remove(current) {
                secret.values.insert(next.to_string(), value);
            }
        }
        if unlocked.data.default_environment == current {
            unlocked.data.default_environment = next.to_string();
        }
        sync_project(unlocked);
        Ok(())
    }

    pub fn delete_environment(&mut self, environment: &str) -> Result<String> {
        let unlocked = self.require_unlocked_mut()?;
        let environment = environment.trim();
        assert_environment(&unlocked.data.environments, environment)?;
        if unlocked.data.environments.len() <= 1 {
            return Err(FatimaError::message(
                "Fatima vault must keep at least one environment.",
            ));
        }
        unlocked
            .data
            .environments
            .retain(|value| value != environment);
        for secret in &mut unlocked.data.secrets {
            secret.values.remove(environment);
        }
        unlocked.environment_secrets.remove(environment);
        unlocked
            .data
            .secrets
            .retain(|secret| !secret.values.is_empty());
        for record in &mut unlocked.user_keys {
            record.wrapped_environment_secrets.remove(environment);
            record.wrapped_user_environment_secrets.remove(environment);
        }
        for user_secrets in unlocked.user_environment_secrets.values_mut() {
            user_secrets.remove(environment);
        }
        for secret in &mut unlocked.user_secrets {
            secret.values.remove(environment);
        }
        unlocked
            .user_secrets
            .retain(|secret| !secret.values.is_empty());
        unlocked
            .user_keys
            .retain(|record| !record.wrapped_environment_secrets.is_empty());
        if unlocked.data.default_environment == environment {
            unlocked.data.default_environment = unlocked.data.environments[0].clone();
        }
        sync_project(unlocked);
        Ok(unlocked.data.default_environment.clone())
    }
}

fn sync_project(unlocked: &mut super::types::UnlockedVault) {
    unlocked.project.environments = unlocked.data.environments.clone();
    unlocked.project.default_environment = unlocked.data.default_environment.clone();
    unlocked.config.environments = unlocked.data.environments.clone();
    unlocked.config.default_environment = unlocked.data.default_environment.clone();
}
