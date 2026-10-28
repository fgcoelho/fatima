use crate::Result;

use super::crypto::{derive_master_key, split_master_key};
use super::domain::assert_non_empty_password;
use super::handle::FatimaVault;
use super::legacy::crypto::random_bytes;

impl FatimaVault {
    pub fn change_password(&mut self, password: &str) -> Result<()> {
        assert_non_empty_password(password)?;
        let unlocked = self.require_unlocked_mut()?;
        let salt = random_bytes(16);
        let master_key = derive_master_key(password, &salt, &unlocked.kdf_params)?;
        unlocked.master_encryption_key = split_master_key(&master_key)?.encryption;
        unlocked.kdf_salt = salt;
        Ok(())
    }
}
