use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{FatimaError, Result};

use super::crypto::{Ciphertext, KdfParams, KeyPair};
use super::legacy::{self, EncryptedBlob};

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
#[serde(rename_all = "camelCase")]
pub struct VaultKdf {
    pub algorithm: String,
    pub salt: String,
    #[serde(flatten)]
    pub params: KdfParams,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultEncryption {
    pub algorithm: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountRecord {
    pub wrapped_account_key: Ciphertext,
    pub public_key: String,
    pub wrapped_private_key: Ciphertext,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentGrant {
    pub public_key: String,
    pub sealed_key: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentRecord {
    pub name: String,
    pub grants: Vec<EnvironmentGrant>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredVaultFile {
    pub version: u8,
    pub project_id: String,
    pub mode: String,
    pub created_at: String,
    pub updated_at: String,
    pub default_environment: String,
    pub kdf: VaultKdf,
    pub encryption: VaultEncryption,
    pub account: AccountRecord,
    pub environments: Vec<EnvironmentRecord>,
    #[serde(default)]
    pub user_keys: Vec<UserKeyRecord>,
    #[serde(default)]
    pub secrets: Vec<StoredSecretRecord>,
    #[serde(default)]
    pub user_secrets: Vec<StoredUserSecretRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StoredSecretRecord {
    pub id: String,
    #[serde(default)]
    pub keys: BTreeMap<String, Ciphertext>,
    #[serde(default)]
    pub values: BTreeMap<String, Ciphertext>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredUserSecretRecord {
    pub id: String,
    pub owner_user_key_id: String,
    #[serde(default)]
    pub keys: BTreeMap<String, Ciphertext>,
    #[serde(default)]
    pub values: BTreeMap<String, Ciphertext>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UserKeyScheme {
    #[default]
    V1,
    V2,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum WrappedKey {
    V2(Ciphertext),
    V1(EncryptedBlob),
}

impl WrappedKey {
    pub(crate) fn v2(&self) -> Result<&Ciphertext> {
        match self {
            WrappedKey::V2(ciphertext) => Ok(ciphertext),
            WrappedKey::V1(_) => Err(FatimaError::Decrypt),
        }
    }

    pub(crate) fn v1(&self) -> Result<&EncryptedBlob> {
        match self {
            WrappedKey::V1(blob) => Ok(blob),
            WrappedKey::V2(_) => Err(FatimaError::Decrypt),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WrappedUserEnvironmentSecret {
    pub by_account_key: Ciphertext,
    pub by_user_key: WrappedKey,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserKeyRecord {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub scheme: UserKeyScheme,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub salt: Option<String>,
    pub secret_hash: String,
    #[serde(default)]
    pub wrapped_environment_secrets: BTreeMap<String, WrappedKey>,
    #[serde(default)]
    pub wrapped_user_environment_secrets: BTreeMap<String, WrappedUserEnvironmentSecret>,
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

#[derive(Clone, Serialize)]
pub struct GeneratedUserKey {
    pub key: String,
    pub record: UserKeyRecord,
}

impl fmt::Debug for GeneratedUserKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GeneratedUserKey")
            .field("record", &self.record)
            .finish_non_exhaustive()
    }
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

#[derive(Clone)]
pub(crate) enum FatimaVaultSession {
    Password(Box<UnlockedVault>),
    UserKey {
        raw_key: String,
        user_key_id: String,
        environments: Vec<String>,
        user_environment_secrets: BTreeMap<String, [u8; 32]>,
    },
}

#[derive(Clone)]
pub(crate) struct UnlockedVault {
    pub project: ProjectSecretManagerSettings,
    pub file: StoredVaultFile,
    pub config: StoredProjectConfig,
    pub data: StoredVaultData,
    pub user_keys: Vec<UserKeyRecord>,
    pub user_secrets: Vec<UserSecretRecord>,
    pub environment_secrets: BTreeMap<String, [u8; 32]>,
    pub user_environment_secrets: BTreeMap<String, BTreeMap<String, [u8; 32]>>,
    pub project_id: String,
    pub account_key: [u8; 32],
    pub key_pair: KeyPair,
    pub master_encryption_key: [u8; 32],
    pub kdf_salt: Vec<u8>,
    pub kdf_params: KdfParams,
}

impl fmt::Debug for FatimaVaultSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Password(vault) => formatter.debug_tuple("Password").field(vault).finish(),
            Self::UserKey { user_key_id, .. } => formatter
                .debug_struct("UserKey")
                .field("user_key_id", user_key_id)
                .finish_non_exhaustive(),
        }
    }
}

impl fmt::Debug for UnlockedVault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UnlockedVault")
            .field("project_id", &self.project_id)
            .finish_non_exhaustive()
    }
}

pub(crate) struct ProjectStore {
    pub project: ResolvedSettings,
    pub file: VaultFile,
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum VaultFile {
    V1(legacy::StoredVaultFile),
    V2(StoredVaultFile),
}

impl VaultFile {
    pub fn default_environment(&self) -> &str {
        match self {
            VaultFile::V1(file) => &file.default_environment,
            VaultFile::V2(file) => &file.default_environment,
        }
    }

    pub fn environment_names(&self) -> Vec<String> {
        match self {
            VaultFile::V1(file) => file
                .environments
                .iter()
                .map(|environment| environment.name.clone())
                .collect(),
            VaultFile::V2(file) => file
                .environments
                .iter()
                .map(|environment| environment.name.clone())
                .collect(),
        }
    }
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
