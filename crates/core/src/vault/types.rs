use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub(crate) const DEFAULT_STORE_PATH: &str = ".fatima";
pub(crate) const VAULT_FILE_NAME: &str = "vault.json";
pub(crate) const DEFAULT_ENVIRONMENTS: [&str; 3] = ["development", "staging", "production"];
pub(crate) const KEY_LENGTH: usize = 32;
pub(crate) const USER_KEY_PREFIX: &str = "fatima_user_";
pub(crate) const USER_KEY_ID_LENGTH: usize = 11;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultOptions {
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default, alias = "store-path")]
    pub store_path: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSecretManagerSettings {
    pub default_environment: String,
    pub environments: Vec<String>,
    pub store_path: String,
}

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
pub struct StoredVaultData {
    pub default_environment: String,
    pub environments: Vec<String>,
    pub secrets: Vec<SecretRecord>,
    #[serde(default)]
    pub user_secrets: Vec<UserSecretRecord>,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SecretRecord {
    pub id: String,
    pub key: String,
    #[serde(default)]
    pub values: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredProjectConfig {
    pub version: u8,
    pub mode: String,
    pub created_at: String,
    pub updated_at: String,
    pub default_environment: String,
    pub environments: Vec<String>,
    pub store: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct FatimaVaultSnapshot {
    pub project: ProjectSecretManagerSettings,
    pub config: StoredProjectConfig,
    pub secrets: Vec<SecretRecord>,
    pub user_keys: Vec<UserKeyRecord>,
    pub user_secrets: Vec<UserSecretRecord>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GeneratedUserKey {
    pub key: String,
    pub record: UserKeyRecord,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSecretRecord {
    pub id: String,
    pub owner_user_key_id: String,
    pub key: String,
    #[serde(default)]
    pub values: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub(crate) enum FatimaVaultSession {
    Password(Box<UnlockedVault>),
    UserKey {
        raw_key: String,
        user_key_id: String,
        environments: Vec<String>,
        user_environment_secrets: BTreeMap<String, Vec<u8>>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct UnlockedVault {
    pub project: ProjectSecretManagerSettings,
    pub file: StoredVaultFile,
    pub config: StoredProjectConfig,
    pub data: StoredVaultData,
    pub user_keys: Vec<UserKeyRecord>,
    pub user_secrets: Vec<UserSecretRecord>,
    pub environment_secrets: BTreeMap<String, Vec<u8>>,
    pub user_environment_secrets: BTreeMap<String, BTreeMap<String, Vec<u8>>>,
    pub master_password_key: Vec<u8>,
}

pub(crate) struct ProjectStore {
    pub project: ResolvedSettings,
    pub file: StoredVaultFile,
}

#[derive(Clone)]
pub(crate) struct ResolvedSettings {
    pub store_path: PathBuf,
}

pub(crate) struct StoreFiles {
    pub vault_path: PathBuf,
}

impl StoreFiles {
    pub fn new(project: &ResolvedSettings) -> Self {
        Self {
            vault_path: project.store_path.join(VAULT_FILE_NAME),
        }
    }
}
