use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::vault::crypto::to_key;
use crate::vault::types::{SecretRecord, StoredVaultData, UserSecretRecord};
use crate::{FatimaError, Result};

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

pub(crate) struct Opened {
    pub environment_secrets: BTreeMap<String, [u8; 32]>,
    pub user_environment_secrets: BTreeMap<String, BTreeMap<String, [u8; 32]>>,
    pub data: StoredVaultData,
}

pub(crate) fn open(file: &StoredVaultFile, password: &str) -> Result<Opened> {
    let master_password_key = crypto::derive_key(password, &file.kdf.salt)?;
    let wrong_password = |error: FatimaError| match error {
        FatimaError::Decrypt => FatimaError::WrongMasterPassword,
        other => other,
    };
    if file.environments.is_empty() {
        return Err(FatimaError::message("Fatima vault has no environments."));
    }
    let mut environment_secrets = BTreeMap::new();
    for environment in &file.environments {
        environment_secrets.insert(
            environment.name.clone(),
            unwrap_key(&environment.wrapped_environment_secret, &master_password_key)
                .map_err(wrong_password)?,
        );
    }
    let mut user_environment_secrets = BTreeMap::new();
    for user_key in &file.user_keys {
        let mut by_environment = BTreeMap::new();
        for (environment, wrapped) in &user_key.wrapped_user_environment_secrets {
            by_environment.insert(
                environment.clone(),
                unwrap_key(&wrapped.by_master_password, &master_password_key)
                    .map_err(wrong_password)?,
            );
        }
        user_environment_secrets.insert(user_key.id.clone(), by_environment);
    }
    let mut secrets = Vec::new();
    for stored in &file.secrets {
        let (key, values) = decrypt_record(&stored.keys, &stored.values, |environment| {
            environment_secrets.get(environment).copied().ok_or_else(|| {
                FatimaError::message(format!("Missing Fatima environment secret: {environment}"))
            })
        })?;
        secrets.push(SecretRecord {
            id: stored.id.clone(),
            key,
            values,
        });
    }
    let mut user_secrets = Vec::new();
    for stored in &file.user_secrets {
        let (key, values) = decrypt_record(&stored.keys, &stored.values, |environment| {
            user_environment_secrets
                .get(&stored.owner_user_key_id)
                .and_then(|items| items.get(environment))
                .copied()
                .ok_or_else(|| FatimaError::message("Missing Fatima user environment secret."))
        })?;
        user_secrets.push(UserSecretRecord {
            id: stored.id.clone(),
            owner_user_key_id: stored.owner_user_key_id.clone(),
            key,
            values,
        });
    }
    let data = StoredVaultData {
        default_environment: file.default_environment.clone(),
        environments: file
            .environments
            .iter()
            .map(|environment| environment.name.clone())
            .collect(),
        secrets,
        user_secrets,
    };
    Ok(Opened {
        environment_secrets,
        user_environment_secrets,
        data,
    })
}

pub(crate) struct UserKeySecrets {
    pub environment_secrets: BTreeMap<String, [u8; 32]>,
    pub user_environment_secrets: BTreeMap<String, [u8; 32]>,
}

pub(crate) fn authenticate_user_key<'a>(
    secret: &str,
    salt: &str,
    secret_hash: &str,
    environments: impl IntoIterator<Item = (&'a String, &'a EncryptedBlob)>,
    user_environments: impl IntoIterator<Item = (&'a String, &'a EncryptedBlob)>,
) -> Result<UserKeySecrets> {
    let user_key = crypto::derive_key(secret, salt)?;
    if crate::vault::crypto::encode_b64(&user_key) != secret_hash {
        return Err(FatimaError::message("Invalid Fatima user key."));
    }
    Ok(UserKeySecrets {
        environment_secrets: unwrap_all(environments, &user_key)?,
        user_environment_secrets: unwrap_all(user_environments, &user_key)?,
    })
}

fn unwrap_all<'a>(
    wrapped: impl IntoIterator<Item = (&'a String, &'a EncryptedBlob)>,
    key: &[u8],
) -> Result<BTreeMap<String, [u8; 32]>> {
    wrapped
        .into_iter()
        .map(|(environment, blob)| Ok((environment.clone(), unwrap_key(blob, key)?)))
        .collect()
}

pub(crate) fn user_key_secrets(
    file: &StoredVaultFile,
    environment: &str,
    environment_secret: &[u8; 32],
    user: Option<(&str, &[u8; 32])>,
) -> Result<(Vec<SecretRecord>, Vec<SecretRecord>)> {
    let mut shared = Vec::new();
    for stored in &file.secrets {
        if let Some(record) = read_one(
            &stored.id,
            &stored.keys,
            &stored.values,
            environment,
            environment_secret,
        )? {
            shared.push(record);
        }
    }
    let mut own = Vec::new();
    if let Some((user_key_id, user_environment_secret)) = user {
        for stored in &file.user_secrets {
            if stored.owner_user_key_id != user_key_id {
                continue;
            }
            if let Some(record) = read_one(
                &stored.id,
                &stored.keys,
                &stored.values,
                environment,
                user_environment_secret,
            )? {
                own.push(record);
            }
        }
    }
    Ok((shared, own))
}

pub(crate) fn set_user_secret(
    file: &mut StoredVaultFile,
    user_key_id: &str,
    user_environment_secret: &[u8; 32],
    environment: &str,
    key: &str,
    value: &str,
) -> Result<()> {
    let mut existing = None;
    for (index, secret) in file.user_secrets.iter().enumerate() {
        if secret.owner_user_key_id != user_key_id {
            continue;
        }
        let Some(encrypted_name) = secret.keys.get(environment) else {
            continue;
        };
        if decrypt_value(encrypted_name, user_environment_secret)? == key {
            existing = Some(index);
            break;
        }
    }
    let encrypted_name = crypto::encrypt_bytes(key.as_bytes(), user_environment_secret)?;
    let encrypted_value = crypto::encrypt_bytes(value.as_bytes(), user_environment_secret)?;
    match existing {
        Some(index) => {
            let secret = &mut file.user_secrets[index];
            secret.keys.insert(environment.to_string(), encrypted_name);
            secret
                .values
                .insert(environment.to_string(), encrypted_value);
        }
        None => file.user_secrets.push(StoredUserSecretRecord {
            id: uuid::Uuid::new_v4().to_string(),
            owner_user_key_id: user_key_id.to_string(),
            keys: BTreeMap::from([(environment.to_string(), encrypted_name)]),
            values: BTreeMap::from([(environment.to_string(), encrypted_value)]),
        }),
    }
    Ok(())
}

fn read_one(
    id: &str,
    keys: &BTreeMap<String, EncryptedBlob>,
    values: &BTreeMap<String, EncryptedBlob>,
    environment: &str,
    secret: &[u8; 32],
) -> Result<Option<SecretRecord>> {
    let Some(encrypted) = values.get(environment) else {
        return Ok(None);
    };
    let encrypted_name = keys
        .get(environment)
        .ok_or_else(|| FatimaError::message("Missing encrypted Fatima secret key."))?;
    Ok(Some(SecretRecord {
        id: id.to_string(),
        key: decrypt_value(encrypted_name, secret)?,
        values: BTreeMap::from([(environment.to_string(), decrypt_value(encrypted, secret)?)]),
    }))
}

fn decrypt_record(
    keys: &BTreeMap<String, EncryptedBlob>,
    values: &BTreeMap<String, EncryptedBlob>,
    secret_for: impl Fn(&str) -> Result<[u8; 32]>,
) -> Result<(String, BTreeMap<String, String>)> {
    let mut name = None;
    let mut decrypted = BTreeMap::new();
    for (environment, encrypted) in values {
        let secret = secret_for(environment)?;
        if name.is_none() {
            if let Some(encrypted_name) = keys.get(environment) {
                name = Some(decrypt_value(encrypted_name, &secret)?);
            }
        }
        decrypted.insert(environment.clone(), decrypt_value(encrypted, &secret)?);
    }
    let name = name.ok_or_else(|| FatimaError::message("Missing encrypted Fatima secret key."))?;
    Ok((name, decrypted))
}

fn decrypt_value(value: &EncryptedBlob, secret: &[u8; 32]) -> Result<String> {
    String::from_utf8(crypto::decrypt_bytes(value, secret)?).map_err(|_| FatimaError::Decrypt)
}

fn unwrap_key(value: &EncryptedBlob, key: &[u8]) -> Result<[u8; 32]> {
    to_key(&crypto::decrypt_bytes(value, key)?)
}
