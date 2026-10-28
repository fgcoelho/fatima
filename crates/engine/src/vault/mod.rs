mod account;
mod associated_data;
mod crypto;
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

pub use crypto::Ciphertext;
pub use handle::FatimaVault;
pub use types::{
    FatimaVaultSnapshot, GeneratedUserKey, ProjectSecretManagerSettings, SecretRecord,
    StoredProjectConfig, StoredVaultData, StoredVaultFile, UserKeyRecord, UserSecretRecord,
    VaultOptions,
};
