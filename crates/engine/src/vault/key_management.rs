use crate::{FatimaError, Result};

use super::domain::dedupe;
use super::types::GeneratedUserKey;
use super::user_key::{assert_user_key_environments, create_user_key_record};
use super::handle::FatimaVault;

impl FatimaVault {
    pub fn generate_user_key(
        &mut self,
        name: &str,
        environments: Vec<String>,
    ) -> Result<GeneratedUserKey> {
        let unlocked = self.require_unlocked_mut()?;
        if name.trim().is_empty() {
            return Err(FatimaError::message(
                "Fatima user key name cannot be empty.",
            ));
        }
        let environments = dedupe(environments);
        if environments.is_empty() {
            return Err(FatimaError::message(
                "Select at least one environment for the Fatima user key.",
            ));
        }
        assert_user_key_environments(&unlocked.data.environments, &environments)?;
        let generated = create_user_key_record(
            name,
            &environments,
            &unlocked.environment_secrets,
            &unlocked.master_password_key,
        )?;
        for (environment, wrapped) in &generated.record.wrapped_user_environment_secrets {
            let secret = super::crypto::decrypt_bytes(
                &wrapped.by_master_password,
                &unlocked.master_password_key,
            )?;
            unlocked
                .user_environment_secrets
                .entry(generated.record.id.clone())
                .or_default()
                .insert(environment.clone(), secret);
        }
        unlocked.user_keys.push(generated.record.clone());
        Ok(generated)
    }
}
