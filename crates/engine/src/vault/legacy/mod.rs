use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub(crate) mod crypto;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EncryptedBlob {
    pub iv: String,
    pub tag: String,
    pub ciphertext: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultKdf {
    pub algorithm: String,
    pub salt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultEncryption {
    pub algorithm: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredVaultFile {
    pub version: u8,
    pub mode: String,
    pub created_at: String,
    pub updated_at: String,
    pub default_environment: String,
    pub environments: Vec<EnvironmentRecord>,
    #[serde(default)]
    pub user_keys: Vec<UserKeyRecord>,
    pub kdf: VaultKdf,
    pub encryption: VaultEncryption,
    #[serde(default)]
    pub secrets: Vec<StoredSecretRecord>,
    #[serde(default)]
    pub user_secrets: Vec<StoredUserSecretRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentRecord {
    pub name: String,
    pub wrapped_environment_secret: EncryptedBlob,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StoredSecretRecord {
    pub id: String,
    #[serde(default)]
    pub keys: BTreeMap<String, EncryptedBlob>,
    #[serde(default)]
    pub values: BTreeMap<String, EncryptedBlob>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredUserSecretRecord {
    pub id: String,
    pub owner_user_key_id: String,
    #[serde(default)]
    pub keys: BTreeMap<String, EncryptedBlob>,
    #[serde(default)]
    pub values: BTreeMap<String, EncryptedBlob>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WrappedUserEnvironmentSecret {
    pub by_master_password: EncryptedBlob,
    pub by_user_key: EncryptedBlob,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserKeyRecord {
    pub id: String,
    pub name: String,
    pub salt: String,
    pub secret_hash: String,
    #[serde(default)]
    pub wrapped_environment_secrets: BTreeMap<String, EncryptedBlob>,
    #[serde(default)]
    pub wrapped_user_environment_secrets: BTreeMap<String, WrappedUserEnvironmentSecret>,
}
