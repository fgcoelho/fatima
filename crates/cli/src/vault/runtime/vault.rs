use std::fs;

use fatima_engine::env::SecretFormat;
use fatima_engine::vault::{
    FatimaVault, FatimaVaultSnapshot, GeneratedUserKey, ProjectSecretManagerSettings, VaultOptions,
};
use fatima_engine::{FatimaError, Result};

pub struct VaultRuntime {
    options: VaultOptions,
}

impl VaultRuntime {
    pub fn new(options: VaultOptions) -> Self {
        Self { options }
    }

    pub fn has_store(&self) -> bool {
        FatimaVault::has_store(Some(self.options.clone()))
    }

    pub fn get_project(&self) -> Result<ProjectSecretManagerSettings> {
        if self.has_store() {
            return FatimaVault::get_settings(Some(self.options.clone()));
        }

        Ok(ProjectSecretManagerSettings {
            default_environment: "development".to_string(),
            environments: vec![
                "development".to_string(),
                "staging".to_string(),
                "production".to_string(),
            ],
            store_path: self
                .options
                .store_path
                .clone()
                .unwrap_or_else(|| ".fatima".to_string()),
        })
    }

    pub fn initialize(&self, password: &str) -> Result<(FatimaVault, FatimaVaultSnapshot)> {
        let vault = FatimaVault::initialize(password, Some(self.options.clone()))?;
        let snapshot = vault.snapshot()?;
        Ok((vault, snapshot))
    }

    pub fn unlock(&self, password: &str) -> Result<(FatimaVault, FatimaVaultSnapshot)> {
        let vault = FatimaVault::unlock_with_password(password, Some(self.options.clone()))?;
        let snapshot = vault.snapshot()?;
        Ok((vault, snapshot))
    }

    pub fn save_snapshot(&self, vault: &mut FatimaVault) -> Result<FatimaVaultSnapshot> {
        vault.save()?;
        vault.snapshot()
    }

    pub fn set_secret(
        &self,
        vault: &mut FatimaVault,
        environment: &str,
        key: &str,
        value: &str,
        id: Option<&str>,
    ) -> Result<FatimaVaultSnapshot> {
        vault.set_secret(environment, key, value, id)?;
        self.save_snapshot(vault)
    }

    pub fn delete_secret(
        &self,
        vault: &mut FatimaVault,
        environment: &str,
        id: &str,
    ) -> Result<FatimaVaultSnapshot> {
        vault.delete_secret(environment, id)?;
        self.save_snapshot(vault)
    }

    pub fn import_env(
        &self,
        vault: &mut FatimaVault,
        environment: &str,
        path: &str,
    ) -> Result<(usize, FatimaVaultSnapshot)> {
        let count = vault.import_env(environment, path)?;
        Ok((count, self.save_snapshot(vault)?))
    }

    pub fn output_env(
        &self,
        vault: &FatimaVault,
        environment: &str,
        path: &str,
        format: SecretFormat,
    ) -> Result<usize> {
        let secrets = vault.get_secrets(Some(environment))?;
        let output = vault.format_secrets(Some(environment), format)?;
        fs::write(path, output).map_err(|source| FatimaError::WriteFile {
            path: path.to_string(),
            source,
        })?;
        Ok(secrets.len())
    }

    pub fn create_environment(
        &self,
        vault: &mut FatimaVault,
        environment: &str,
    ) -> Result<FatimaVaultSnapshot> {
        vault.create_environment(environment)?;
        self.save_snapshot(vault)
    }

    pub fn rename_environment(
        &self,
        vault: &mut FatimaVault,
        current: &str,
        next: &str,
    ) -> Result<FatimaVaultSnapshot> {
        vault.rename_environment(current, next)?;
        self.save_snapshot(vault)
    }

    pub fn delete_environment(
        &self,
        vault: &mut FatimaVault,
        environment: &str,
    ) -> Result<(String, FatimaVaultSnapshot)> {
        let selected = vault.delete_environment(environment)?;
        Ok((selected, self.save_snapshot(vault)?))
    }

    pub fn generate_user_key(
        &self,
        vault: &mut FatimaVault,
        name: &str,
        environments: Vec<String>,
    ) -> Result<(GeneratedUserKey, FatimaVaultSnapshot)> {
        let generated = vault.generate_user_key(name, environments)?;
        Ok((generated, self.save_snapshot(vault)?))
    }

    pub fn change_password(
        &self,
        vault: &mut FatimaVault,
        password: &str,
    ) -> Result<FatimaVaultSnapshot> {
        vault.change_password(password)?;
        self.save_snapshot(vault)
    }
}
