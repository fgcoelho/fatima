mod crypto;
mod domain;
mod environments;
mod handle;
mod key_management;
mod password;
mod persistence;
mod secrets;
mod store;
mod types;
mod user_key;

pub use types::{
    EncryptedBlob, FatimaVaultSnapshot, GeneratedUserKey, ProjectSecretManagerSettings,
    SecretRecord, StoredProjectConfig, StoredVaultData, StoredVaultFile, UserKeyRecord,
    UserSecretRecord, VaultOptions,
};
pub use handle::FatimaVault;
