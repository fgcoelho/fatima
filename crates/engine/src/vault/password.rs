use crate::Result;

use super::legacy::crypto::{derive_key, random_b64};
use super::domain::assert_non_empty_password;
use super::handle::FatimaVault;

impl FatimaVault {
    pub fn change_password(&mut self, password: &str) -> Result<()> {
        assert_non_empty_password(password)?;
        let unlocked = self.require_unlocked_mut()?;
        let salt = random_b64(16);
        unlocked.master_password_key = derive_key(password, &salt)?;
        unlocked.file.kdf.salt = salt;
        Ok(())
    }
}
