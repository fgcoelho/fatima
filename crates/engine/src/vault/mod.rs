mod domain;
mod environments;
mod handle;
mod key_management;
mod legacy;
mod password;
mod persistence;
mod secrets;
mod store;
mod types;
mod user_key;

pub use legacy::{EncryptedBlob, StoredVaultFile, UserKeyRecord};
pub use types::{
    FatimaVaultSnapshot, GeneratedUserKey, ProjectSecretManagerSettings, SecretRecord,
    StoredProjectConfig, StoredVaultData, UserSecretRecord, VaultOptions,
};
pub use handle::FatimaVault;
