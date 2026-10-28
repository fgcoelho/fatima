use crate::{FatimaError, Result};

use super::handle::FatimaVault;
use super::store::save_unlocked;
use super::types::{FatimaVaultSession, FatimaVaultSnapshot};

impl FatimaVault {
    pub fn save(&mut self) -> Result<()> {
        let unlocked = self.require_unlocked_mut()?;
        save_unlocked(unlocked)
    }

    pub fn snapshot(&self) -> Result<FatimaVaultSnapshot> {
        let FatimaVaultSession::Password(unlocked) = self.require_session()? else {
            return Err(FatimaError::message(
                "Fatima vault snapshot requires password authentication.",
            ));
        };
        Ok(FatimaVaultSnapshot {
            project: unlocked.project.clone(),
            config: unlocked.config.clone(),
            secrets: unlocked.data.secrets.clone(),
            user_keys: unlocked.user_keys.clone(),
            user_secrets: unlocked.user_secrets.clone(),
        })
    }
}
