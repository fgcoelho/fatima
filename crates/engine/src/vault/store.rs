use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{FatimaError, Result};

use super::legacy::crypto::{decrypt_bytes, derive_key, encrypt_bytes, random_b64, random_bytes};
use super::domain::assert_non_empty_password;
use super::legacy::{
    EnvironmentRecord, StoredSecretRecord, StoredUserSecretRecord, StoredVaultFile,
    VaultEncryption, VaultKdf,
};
use super::types::{
    ProjectSecretManagerSettings, ProjectStore, ResolvedSettings, SecretRecord, StoreFiles,
    StoredProjectConfig, StoredVaultData, UserSecretRecord, VaultOptions, DEFAULT_ENVIRONMENTS,
    DEFAULT_STORE_PATH, KEY_LENGTH,
};

pub(crate) fn has_store(options: Option<&VaultOptions>) -> bool {
    let project = resolve_settings(options);
    StoreFiles::new(&project).vault_path.exists()
}

pub(crate) fn get_settings(options: Option<&VaultOptions>) -> Result<ProjectSecretManagerSettings> {
    let store = read_store(options)?;
    assert_supported_store(&store)?;
    Ok(ProjectSecretManagerSettings {
        default_environment: store.file.default_environment,
        environments: store
            .file
            .environments
            .iter()
            .map(|environment| environment.name.clone())
            .collect(),
        store_path: resolve_settings(options).store_path.display().to_string(),
    })
}

pub(crate) fn initialize_store(
    password: &str,
    options: &VaultOptions,
) -> Result<super::types::UnlockedVault> {
    assert_non_empty_password(password)?;
    let project = resolve_settings(Some(options));
    if has_store(Some(options)) {
        return Err(FatimaError::message(format!(
            "Fatima store already exists at {}.",
            project.store_path.display()
        )));
    }

    fs::create_dir_all(&project.store_path).map_err(|source| FatimaError::WriteFile {
        path: project.store_path.display().to_string(),
        source,
    })?;
    let now = now()?;
    let environments = DEFAULT_ENVIRONMENTS
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    let data = StoredVaultData {
        default_environment: "development".to_string(),
        environments: environments.clone(),
        secrets: Vec::new(),
        user_secrets: Vec::new(),
    };
    let salt = random_b64(16);
    let master_password_key = derive_key(password, &salt)?;
    let environment_secrets = environments
        .iter()
        .map(|environment| (environment.clone(), random_bytes(KEY_LENGTH)))
        .collect::<BTreeMap<_, _>>();
    let file = StoredVaultFile {
        version: 1,
        mode: "local".to_string(),
        created_at: now.clone(),
        updated_at: now,
        default_environment: data.default_environment.clone(),
        environments: wrap_environment_secrets(
            &data.environments,
            &environment_secrets,
            &master_password_key,
        )?,
        user_keys: Vec::new(),
        kdf: VaultKdf {
            algorithm: "scrypt".to_string(),
            salt,
        },
        encryption: VaultEncryption {
            algorithm: "aes-256-gcm".to_string(),
        },
        secrets: Vec::new(),
        user_secrets: Vec::new(),
    };
    let files = StoreFiles::new(&project);
    write_json(&files.vault_path, &file)?;
    Ok(unlocked(
        project,
        file,
        data,
        environment_secrets,
        BTreeMap::new(),
        master_password_key,
    ))
}

pub(crate) fn resolve_settings(options: Option<&VaultOptions>) -> ResolvedSettings {
    let cwd = options
        .and_then(|options| options.cwd.as_ref())
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let store_path = options
        .and_then(|options| options.store_path.as_ref())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STORE_PATH));
    let store_path = if store_path.is_absolute() {
        store_path
    } else {
        cwd.join(store_path)
    };
    ResolvedSettings { store_path }
}

pub(crate) fn read_store(options: Option<&VaultOptions>) -> Result<ProjectStore> {
    let project = resolve_settings(options);
    let files = StoreFiles::new(&project);
    if !project.store_path.exists() || !files.vault_path.exists() {
        return Err(FatimaError::message(format!(
            "Fatima secret manager is not initialized in {}. Open `fatima` to initialize it.",
            project.store_path.display()
        )));
    }
    Ok(ProjectStore {
        project,
        file: read_json(&files.vault_path, "Fatima vault")?,
    })
}

pub(crate) fn write_store(store: &ProjectStore) -> Result<()> {
    write_json(&StoreFiles::new(&store.project).vault_path, &store.file)
}

pub(crate) fn assert_supported_store(store: &ProjectStore) -> Result<()> {
    if store.file.version != 1 {
        return Err(FatimaError::message(
            "Fatima vault format is no longer supported. Delete and reinitialize the local vault.",
        ));
    }
    Ok(())
}

pub(crate) fn unlock_store(
    password: &str,
    options: Option<&VaultOptions>,
) -> Result<super::types::UnlockedVault> {
    let store = read_store(options)?;
    assert_supported_store(&store)?;
    let master_password_key = derive_key(password, &store.file.kdf.salt)?;
    let environment_secrets = unwrap_environment_secrets(&store.file, &master_password_key)?;
    let user_environment_secrets =
        unwrap_user_environment_secrets(&store.file.user_keys, &master_password_key)?;
    let data = decrypt_data(&store.file, &environment_secrets, &user_environment_secrets)?;
    Ok(unlocked(
        store.project,
        store.file,
        data,
        environment_secrets,
        user_environment_secrets,
        master_password_key,
    ))
}

pub(crate) fn save_unlocked(unlocked: &mut super::types::UnlockedVault) -> Result<()> {
    unlocked.file.updated_at = now()?;
    unlocked.file.default_environment = unlocked.data.default_environment.clone();
    unlocked.file.environments = wrap_environment_secrets(
        &unlocked.data.environments,
        &unlocked.environment_secrets,
        &unlocked.master_password_key,
    )?;
    unlocked.file.user_keys = unlocked.user_keys.clone();
    unlocked.config.updated_at = unlocked.file.updated_at.clone();
    unlocked.file.secrets = encrypt_secrets(&unlocked.data.secrets, &unlocked.environment_secrets)?;
    unlocked.file.user_secrets =
        encrypt_user_secrets(&unlocked.user_secrets, &unlocked.user_environment_secrets)?;
    let project = ResolvedSettings {
        store_path: PathBuf::from(&unlocked.project.store_path),
    };
    write_json(&StoreFiles::new(&project).vault_path, &unlocked.file)
}

fn unlocked(
    project: ResolvedSettings,
    file: StoredVaultFile,
    data: StoredVaultData,
    environment_secrets: BTreeMap<String, Vec<u8>>,
    user_environment_secrets: BTreeMap<String, BTreeMap<String, Vec<u8>>>,
    master_password_key: Vec<u8>,
) -> super::types::UnlockedVault {
    let store_name = project
        .store_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(DEFAULT_STORE_PATH)
        .to_string();
    let config = StoredProjectConfig {
        version: file.version,
        mode: file.mode.clone(),
        created_at: file.created_at.clone(),
        updated_at: file.updated_at.clone(),
        default_environment: data.default_environment.clone(),
        environments: data.environments.clone(),
        store: store_name,
    };
    let user_keys = file.user_keys.clone();
    let user_secrets = data.user_secrets.clone();
    super::types::UnlockedVault {
        project: ProjectSecretManagerSettings {
            default_environment: data.default_environment.clone(),
            environments: data.environments.clone(),
            store_path: project.store_path.display().to_string(),
        },
        file,
        config,
        data,
        user_keys,
        user_secrets,
        environment_secrets,
        user_environment_secrets,
        master_password_key,
    }
}

pub(crate) fn environment_names(file: &StoredVaultFile) -> Vec<String> {
    file.environments
        .iter()
        .map(|environment| environment.name.clone())
        .collect()
}

pub(crate) fn decrypt_environment_value(
    value: &super::legacy::EncryptedBlob,
    environment_secret: &[u8],
) -> Result<String> {
    let bytes = decrypt_bytes(value, environment_secret)?;
    String::from_utf8(bytes).map_err(|error| FatimaError::message(error.to_string()))
}

pub(crate) fn encrypt_environment_value(
    value: &str,
    environment_secret: &[u8],
) -> Result<super::legacy::EncryptedBlob> {
    encrypt_bytes(value.as_bytes(), environment_secret)
}

fn wrap_environment_secrets(
    environments: &[String],
    environment_secrets: &BTreeMap<String, Vec<u8>>,
    master_password_key: &[u8],
) -> Result<Vec<EnvironmentRecord>> {
    environments
        .iter()
        .map(|environment| {
            let secret = environment_secrets.get(environment).ok_or_else(|| {
                FatimaError::message(format!("Missing Fatima environment secret: {environment}"))
            })?;
            Ok(EnvironmentRecord {
                name: environment.clone(),
                wrapped_environment_secret: encrypt_bytes(secret, master_password_key)?,
            })
        })
        .collect()
}

pub(crate) fn unwrap_environment_secrets(
    file: &StoredVaultFile,
    master_password_key: &[u8],
) -> Result<BTreeMap<String, Vec<u8>>> {
    file.environments
        .iter()
        .map(|environment| {
            Ok((
                environment.name.clone(),
                decrypt_bytes(&environment.wrapped_environment_secret, master_password_key)?,
            ))
        })
        .collect()
}

fn decrypt_data(
    file: &StoredVaultFile,
    environment_secrets: &BTreeMap<String, Vec<u8>>,
    user_environment_secrets: &BTreeMap<String, BTreeMap<String, Vec<u8>>>,
) -> Result<StoredVaultData> {
    let mut secrets = Vec::new();
    for stored in &file.secrets {
        let mut values = BTreeMap::new();
        let mut key = None;
        for (environment, encrypted) in &stored.values {
            let environment_secret = environment_secrets.get(environment).ok_or_else(|| {
                FatimaError::message(format!("Missing Fatima environment secret: {environment}"))
            })?;
            if key.is_none() {
                if let Some(encrypted_key) = stored.keys.get(environment) {
                    key = Some(decrypt_environment_value(
                        encrypted_key,
                        environment_secret,
                    )?);
                }
            }
            values.insert(
                environment.clone(),
                decrypt_environment_value(encrypted, environment_secret)?,
            );
        }
        secrets.push(SecretRecord {
            id: stored.id.clone(),
            key: key.ok_or_else(|| FatimaError::message("Missing encrypted Fatima secret key."))?,
            values,
        });
    }
    let user_secrets = decrypt_user_secrets(&file.user_secrets, user_environment_secrets)?;
    Ok(StoredVaultData {
        default_environment: file.default_environment.clone(),
        environments: environment_names(file),
        secrets,
        user_secrets,
    })
}

fn encrypt_secrets(
    secrets: &[SecretRecord],
    environment_secrets: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<StoredSecretRecord>> {
    let mut stored_secrets = Vec::new();
    for secret in secrets {
        let mut keys = BTreeMap::new();
        let mut values = BTreeMap::new();
        for (environment, value) in &secret.values {
            let environment_secret = environment_secrets.get(environment).ok_or_else(|| {
                FatimaError::message(format!("Missing Fatima environment secret: {environment}"))
            })?;
            keys.insert(
                environment.clone(),
                encrypt_environment_value(&secret.key, environment_secret)?,
            );
            values.insert(
                environment.clone(),
                encrypt_environment_value(value, environment_secret)?,
            );
        }
        stored_secrets.push(StoredSecretRecord {
            id: secret.id.clone(),
            keys,
            values,
        });
    }
    Ok(stored_secrets)
}

fn unwrap_user_environment_secrets(
    user_keys: &[super::legacy::UserKeyRecord],
    master_password_key: &[u8],
) -> Result<BTreeMap<String, BTreeMap<String, Vec<u8>>>> {
    if master_password_key.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut result = BTreeMap::new();
    for user_key in user_keys {
        let mut by_environment = BTreeMap::new();
        for (environment, wrapped) in &user_key.wrapped_user_environment_secrets {
            by_environment.insert(
                environment.clone(),
                decrypt_bytes(&wrapped.by_master_password, master_password_key)?,
            );
        }
        result.insert(user_key.id.clone(), by_environment);
    }
    Ok(result)
}

fn decrypt_user_secrets(
    stored_secrets: &[StoredUserSecretRecord],
    user_environment_secrets: &BTreeMap<String, BTreeMap<String, Vec<u8>>>,
) -> Result<Vec<UserSecretRecord>> {
    let mut secrets = Vec::new();
    for stored in stored_secrets {
        let mut values = BTreeMap::new();
        let mut key = None;
        for (environment, encrypted) in &stored.values {
            let secret = user_environment_secrets
                .get(&stored.owner_user_key_id)
                .and_then(|items| items.get(environment))
                .ok_or_else(|| FatimaError::message("Missing Fatima user environment secret."))?;
            if key.is_none() {
                if let Some(encrypted_key) = stored.keys.get(environment) {
                    key = Some(decrypt_environment_value(encrypted_key, secret)?);
                }
            }
            values.insert(
                environment.clone(),
                decrypt_environment_value(encrypted, secret)?,
            );
        }
        secrets.push(UserSecretRecord {
            id: stored.id.clone(),
            owner_user_key_id: stored.owner_user_key_id.clone(),
            key: key
                .ok_or_else(|| FatimaError::message("Missing encrypted Fatima user secret key."))?,
            values,
        });
    }
    Ok(secrets)
}

fn encrypt_user_secrets(
    secrets: &[UserSecretRecord],
    user_environment_secrets: &BTreeMap<String, BTreeMap<String, Vec<u8>>>,
) -> Result<Vec<StoredUserSecretRecord>> {
    let mut stored = Vec::new();
    for secret in secrets {
        let mut keys = BTreeMap::new();
        let mut values = BTreeMap::new();
        for (environment, value) in &secret.values {
            let user_environment_secret = user_environment_secrets
                .get(&secret.owner_user_key_id)
                .and_then(|items| items.get(environment))
                .ok_or_else(|| FatimaError::message("Missing Fatima user environment secret."))?;
            keys.insert(
                environment.clone(),
                encrypt_environment_value(&secret.key, user_environment_secret)?,
            );
            values.insert(
                environment.clone(),
                encrypt_environment_value(value, user_environment_secret)?,
            );
        }
        stored.push(StoredUserSecretRecord {
            id: secret.id.clone(),
            owner_user_key_id: secret.owner_user_key_id.clone(),
            keys,
            values,
        });
    }
    Ok(stored)
}

fn now() -> Result<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| FatimaError::message(error.to_string()))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &std::path::Path, label: &str) -> Result<T> {
    let content = fs::read_to_string(path).map_err(|source| FatimaError::ReadFile {
        path: path.display().to_string(),
        source,
    })?;
    serde_json::from_str(&content)
        .map_err(|error| FatimaError::message(format!("Invalid {label}: {error}")))
}

fn write_json<T: Serialize>(path: &std::path::Path, value: &T) -> Result<()> {
    let content = serde_json::to_string_pretty(value)
        .map_err(|error| FatimaError::message(error.to_string()))?;
    fs::write(path, format!("{content}\n")).map_err(|source| FatimaError::WriteFile {
        path: path.display().to_string(),
        source,
    })
}
