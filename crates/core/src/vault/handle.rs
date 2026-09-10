use crate::{FatimaError, Result};

use super::store::{get_settings, has_store, initialize_store, read_store, unlock_store};
use super::types::{FatimaVaultSession, ProjectSecretManagerSettings, VaultOptions};
use super::user_key::authenticate_user_key;

#[derive(Clone, Debug)]
pub struct FatimaVault {
    pub(crate) options: VaultOptions,
    pub(crate) session: Option<FatimaVaultSession>,
}

impl Default for FatimaVault {
    fn default() -> Self {
        Self::new()
    }
}

impl FatimaVault {
    pub fn new() -> Self {
        Self {
            options: VaultOptions::default(),
            session: None,
        }
    }

    pub fn with_options(options: VaultOptions) -> Self {
        Self {
            options,
            session: None,
        }
    }

    pub fn store_path(mut self, store_path: impl Into<String>) -> Self {
        self.options.store_path = Some(store_path.into());
        self
    }

    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.options.cwd = Some(cwd.into());
        self
    }

    pub fn has_store(options: Option<VaultOptions>) -> bool {
        has_store(options.as_ref())
    }

    pub fn get_settings(options: Option<VaultOptions>) -> Result<ProjectSecretManagerSettings> {
        get_settings(options.as_ref())
    }

    pub fn initialize(password: &str, options: Option<VaultOptions>) -> Result<Self> {
        let options = options.unwrap_or_default();
        let unlocked = initialize_store(password, &options)?;
        Ok(Self {
            options,
            session: Some(FatimaVaultSession::Password(Box::new(unlocked))),
        })
    }

    pub fn unlock_with_password(password: &str, options: Option<VaultOptions>) -> Result<Self> {
        let options = options.unwrap_or_default();
        let unlocked = unlock_store(password, Some(&options))?;
        Ok(Self {
            options,
            session: Some(FatimaVaultSession::Password(Box::new(unlocked))),
        })
    }

    pub fn authenticate_with_user_key(key: &str, options: Option<VaultOptions>) -> Result<Self> {
        let options = options.unwrap_or_default();
        let authenticated = authenticate_user_key(key, Some(&options))?;
        Ok(Self {
            options,
            session: Some(FatimaVaultSession::UserKey {
                raw_key: key.to_string(),
                user_key_id: authenticated.user_key_id,
                environments: authenticated.environments,
                user_environment_secrets: authenticated.user_environment_secrets,
            }),
        })
    }

    pub fn resolve_default_user_key(provided_key: Option<&str>) -> Option<String> {
        provided_key
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().to_string())
            .or_else(|| std::env::var("FATIMA_USER_KEY").ok())
            .filter(|value| !value.trim().is_empty())
    }

    pub fn resolve_default_password(provided_password: Option<&str>) -> Option<String> {
        provided_password
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().to_string())
            .or_else(|| std::env::var("FATIMA_VAULT_PASSWORD").ok())
            .filter(|value| !value.trim().is_empty())
    }

    pub fn is_authenticated(&self) -> bool {
        self.session.is_some()
    }

    pub fn list_environments(&self) -> Result<Vec<String>> {
        match self.require_session()? {
            FatimaVaultSession::Password(unlocked) => Ok(unlocked.data.environments.clone()),
            FatimaVaultSession::UserKey { environments, .. } => Ok(environments.clone()),
        }
    }

    pub fn get_default_environment(&self) -> Result<String> {
        match self.require_session()? {
            FatimaVaultSession::Password(unlocked) => Ok(unlocked.data.default_environment.clone()),
            FatimaVaultSession::UserKey { .. } => {
                Ok(read_store(Some(&self.options))?.file.default_environment)
            }
        }
    }

    pub(crate) fn require_session(&self) -> Result<&FatimaVaultSession> {
        self.session
            .as_ref()
            .ok_or_else(|| FatimaError::message("Fatima vault is not authenticated."))
    }

    pub(crate) fn require_unlocked_mut(&mut self) -> Result<&mut super::types::UnlockedVault> {
        match self.session.as_mut() {
            Some(FatimaVaultSession::Password(unlocked)) => Ok(unlocked.as_mut()),
            Some(FatimaVaultSession::UserKey { .. }) => Err(FatimaError::message(
                "Fatima vault mutation requires password authentication.",
            )),
            None => Err(FatimaError::message("Fatima vault is not authenticated.")),
        }
    }
}
