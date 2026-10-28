use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{FatimaError, Result};

use super::account::{
    grant_environments, new_account_secrets, unwrap_account, unwrap_environment_keys,
    unwrap_user_environment_keys, wrap_account, AccountSecrets,
};
use super::associated_data::{self, Field};
use super::crypto::{
    decode_b64, decrypt, derive_encryption_key, derive_master_key, encode_b64, encrypt,
    hash_legacy_user_key, random_bytes, random_key, Ciphertext, KdfParams, KDF_SALT_LENGTH,
};
use super::domain::assert_non_empty_password;
use super::legacy;
use super::types::{
    ProjectSecretManagerSettings, ProjectStore, ResolvedSettings, SecretRecord, StoreFiles,
    StoredProjectConfig, StoredSecretRecord, StoredUserSecretRecord, StoredVaultData,
    StoredVaultFile, UserKeyRecord, UserKeyScheme, UserSecretRecord, VaultEncryption, VaultFile,
    VaultKdf, VaultOptions, WrappedKey, WrappedUserEnvironmentSecret, DEFAULT_ENVIRONMENTS,
    DEFAULT_STORE_PATH,
};

const VAULT_VERSION: u8 = 2;

pub(crate) fn has_store(options: Option<&VaultOptions>) -> bool {
    let project = resolve_settings(options);
    StoreFiles::new(&project).vault_path.exists()
}

pub(crate) fn get_settings(options: Option<&VaultOptions>) -> Result<ProjectSecretManagerSettings> {
    let store = read_store(options)?;
    Ok(ProjectSecretManagerSettings {
        default_environment: store.file.default_environment().to_string(),
        environments: store.file.environment_names(),
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
    let kdf_salt = random_bytes(KDF_SALT_LENGTH);
    let kdf_params = KdfParams::default();
    let master_key = derive_master_key(password, &kdf_salt, &kdf_params)?;
    let account = new_account_secrets(&master_key, kdf_salt, kdf_params)?;
    let project_id = uuid::Uuid::new_v4().to_string();
    let environment_secrets = environments
        .iter()
        .map(|environment| (environment.clone(), random_key()))
        .collect::<BTreeMap<_, _>>();
    let file = StoredVaultFile {
        version: VAULT_VERSION,
        project_id: project_id.clone(),
        mode: "local".to_string(),
        created_at: now.clone(),
        updated_at: now,
        default_environment: data.default_environment.clone(),
        kdf: stored_kdf(&account),
        encryption: VaultEncryption {
            algorithm: "xchacha20-poly1305".to_string(),
        },
        account: wrap_account(&project_id, &account)?,
        environments: grant_environments(
            &data.environments,
            &environment_secrets,
            &[],
            &account.key_pair,
        )?,
        user_keys: Vec::new(),
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
        account,
    ))
}

fn stored_kdf(account: &AccountSecrets) -> VaultKdf {
    VaultKdf {
        algorithm: "argon2id".to_string(),
        salt: encode_b64(&account.kdf_salt),
        params: account.kdf_params.clone(),
    }
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
    let content =
        fs::read_to_string(&files.vault_path).map_err(|source| FatimaError::ReadFile {
            path: files.vault_path.display().to_string(),
            source,
        })?;
    let probe: VersionProbe = parse_json(&content, "Fatima vault")?;
    let file = match probe.version {
        1 => VaultFile::V1(parse_json(&content, "Fatima vault")?),
        VAULT_VERSION => VaultFile::V2(parse_json(&content, "Fatima vault")?),
        version => return Err(FatimaError::UnsupportedVaultVersion { version }),
    };
    Ok(ProjectStore { project, file })
}

pub(crate) fn write_store(store: &ProjectStore) -> Result<()> {
    write_json(&StoreFiles::new(&store.project).vault_path, &store.file)
}

pub(crate) fn unlock_store(
    password: &str,
    options: Option<&VaultOptions>,
) -> Result<super::types::UnlockedVault> {
    let store = read_store(options)?;
    match store.file {
        VaultFile::V1(file) => upgrade_v1(store.project, file, password),
        VaultFile::V2(file) => unlock_v2(store.project, file, password),
    }
}

fn unlock_v2(
    project: ResolvedSettings,
    file: StoredVaultFile,
    password: &str,
) -> Result<super::types::UnlockedVault> {
    let kdf_salt = decode_b64(&file.kdf.salt)?;
    let kdf_params = file.kdf.params.clone();
    let master_key = derive_master_key(password, &kdf_salt, &kdf_params)?;
    let account = unwrap_account(
        &file.account,
        &file.project_id,
        derive_encryption_key(&master_key)?,
        kdf_salt,
        kdf_params,
    )?;
    let environment_secrets = unwrap_environment_keys(&file.environments, &account.key_pair)?;
    let user_environment_secrets =
        unwrap_user_environment_keys(&file.user_keys, &file.project_id, &account.account_key)?;
    let data = decrypt_data(&file, &environment_secrets, &user_environment_secrets)?;
    Ok(unlocked(
        project,
        file,
        data,
        environment_secrets,
        user_environment_secrets,
        account,
    ))
}

fn upgrade_v1(
    project: ResolvedSettings,
    file: legacy::StoredVaultFile,
    password: &str,
) -> Result<super::types::UnlockedVault> {
    let opened = legacy::open(&file, password)?;
    let kdf_salt = random_bytes(KDF_SALT_LENGTH);
    let kdf_params = KdfParams::default();
    let master_key = derive_master_key(password, &kdf_salt, &kdf_params)?;
    let account = new_account_secrets(&master_key, kdf_salt, kdf_params)?;
    let project_id = uuid::Uuid::new_v4().to_string();
    let user_keys = file
        .user_keys
        .iter()
        .map(|record| {
            upgrade_user_key(
                record,
                &project_id,
                &account.account_key,
                &opened.user_environment_secrets,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let upgraded = StoredVaultFile {
        version: VAULT_VERSION,
        project_id: project_id.clone(),
        mode: file.mode,
        created_at: file.created_at,
        updated_at: file.updated_at,
        default_environment: file.default_environment,
        kdf: stored_kdf(&account),
        encryption: VaultEncryption {
            algorithm: "xchacha20-poly1305".to_string(),
        },
        account: wrap_account(&project_id, &account)?,
        environments: grant_environments(
            &opened.data.environments,
            &opened.environment_secrets,
            &[],
            &account.key_pair,
        )?,
        user_keys,
        secrets: Vec::new(),
        user_secrets: Vec::new(),
    };
    Ok(unlocked(
        project,
        upgraded,
        opened.data,
        opened.environment_secrets,
        opened.user_environment_secrets,
        account,
    ))
}

fn upgrade_user_key(
    record: &legacy::UserKeyRecord,
    project_id: &str,
    account_key: &[u8; 32],
    user_environment_secrets: &BTreeMap<String, BTreeMap<String, [u8; 32]>>,
) -> Result<UserKeyRecord> {
    let mut wrapped_user_environment_secrets = BTreeMap::new();
    for (environment, wrapped) in &record.wrapped_user_environment_secrets {
        let secret = user_environment_secrets
            .get(&record.id)
            .and_then(|items| items.get(environment))
            .ok_or(FatimaError::Decrypt)?;
        wrapped_user_environment_secrets.insert(
            environment.clone(),
            WrappedUserEnvironmentSecret {
                by_account_key: encrypt(
                    secret,
                    account_key,
                    &associated_data::user_environment(project_id, &record.id),
                )?,
                by_user_key: WrappedKey::V1(wrapped.by_user_key.clone()),
            },
        );
    }
    Ok(UserKeyRecord {
        id: record.id.clone(),
        name: record.name.clone(),
        scheme: UserKeyScheme::V1,
        salt: Some(record.salt.clone()),
        secret_hash: hash_legacy_user_key(&decode_b64(&record.secret_hash)?)?,
        wrapped_environment_secrets: record
            .wrapped_environment_secrets
            .iter()
            .map(|(environment, blob)| (environment.clone(), WrappedKey::V1(blob.clone())))
            .collect(),
        wrapped_user_environment_secrets,
    })
}

pub(crate) fn save_unlocked(unlocked: &mut super::types::UnlockedVault) -> Result<()> {
    unlocked.file.updated_at = now()?;
    unlocked.file.default_environment = unlocked.data.default_environment.clone();
    let account = AccountSecrets {
        account_key: unlocked.account_key,
        key_pair: unlocked.key_pair.clone(),
        master_encryption_key: unlocked.master_encryption_key,
        kdf_salt: unlocked.kdf_salt.clone(),
        kdf_params: unlocked.kdf_params.clone(),
    };
    unlocked.file.kdf = stored_kdf(&account);
    unlocked.file.account = wrap_account(&unlocked.project_id, &account)?;
    unlocked.file.environments = grant_environments(
        &unlocked.data.environments,
        &unlocked.environment_secrets,
        &unlocked.file.environments,
        &unlocked.key_pair,
    )?;
    unlocked.file.user_keys = unlocked.user_keys.clone();
    unlocked.config.updated_at = unlocked.file.updated_at.clone();
    unlocked.file.secrets = encrypt_secrets(
        &unlocked.project_id,
        &unlocked.data.secrets,
        &unlocked.environment_secrets,
    )?;
    unlocked.file.user_secrets = encrypt_user_secrets(
        &unlocked.project_id,
        &unlocked.user_secrets,
        &unlocked.user_environment_secrets,
    )?;
    let project = ResolvedSettings {
        store_path: PathBuf::from(&unlocked.project.store_path),
    };
    write_json(&StoreFiles::new(&project).vault_path, &unlocked.file)
}

fn unlocked(
    project: ResolvedSettings,
    file: StoredVaultFile,
    data: StoredVaultData,
    environment_secrets: BTreeMap<String, [u8; 32]>,
    user_environment_secrets: BTreeMap<String, BTreeMap<String, [u8; 32]>>,
    account: AccountSecrets,
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
        project_id: file.project_id.clone(),
        file,
        config,
        data,
        user_keys,
        user_secrets,
        environment_secrets,
        user_environment_secrets,
        account_key: account.account_key,
        key_pair: account.key_pair,
        master_encryption_key: account.master_encryption_key,
        kdf_salt: account.kdf_salt,
        kdf_params: account.kdf_params,
    }
}

pub(crate) fn environment_names(file: &StoredVaultFile) -> Vec<String> {
    file.environments
        .iter()
        .map(|environment| environment.name.clone())
        .collect()
}

pub(crate) fn decrypt_environment_value(
    value: &Ciphertext,
    environment_secret: &[u8; 32],
    associated_data: &[u8],
) -> Result<String> {
    let bytes = decrypt(value, environment_secret, associated_data)?;
    String::from_utf8(bytes).map_err(|_| FatimaError::Decrypt)
}

pub(crate) fn encrypt_environment_value(
    value: &str,
    environment_secret: &[u8; 32],
    associated_data: &[u8],
) -> Result<Ciphertext> {
    encrypt(value.as_bytes(), environment_secret, associated_data)
}

fn decrypt_data(
    file: &StoredVaultFile,
    environment_secrets: &BTreeMap<String, [u8; 32]>,
    user_environment_secrets: &BTreeMap<String, BTreeMap<String, [u8; 32]>>,
) -> Result<StoredVaultData> {
    let project_id = &file.project_id;
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
                        &associated_data::secret_field(
                            project_id,
                            environment,
                            &stored.id,
                            Field::Name,
                        ),
                    )?);
                }
            }
            values.insert(
                environment.clone(),
                decrypt_environment_value(
                    encrypted,
                    environment_secret,
                    &associated_data::secret_field(
                        project_id,
                        environment,
                        &stored.id,
                        Field::Value,
                    ),
                )?,
            );
        }
        secrets.push(SecretRecord {
            id: stored.id.clone(),
            key: key.ok_or_else(|| FatimaError::message("Missing encrypted Fatima secret key."))?,
            values,
        });
    }
    let user_secrets =
        decrypt_user_secrets(project_id, &file.user_secrets, user_environment_secrets)?;
    Ok(StoredVaultData {
        default_environment: file.default_environment.clone(),
        environments: environment_names(file),
        secrets,
        user_secrets,
    })
}

fn encrypt_secrets(
    project_id: &str,
    secrets: &[SecretRecord],
    environment_secrets: &BTreeMap<String, [u8; 32]>,
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
                encrypt_environment_value(
                    &secret.key,
                    environment_secret,
                    &associated_data::secret_field(
                        project_id,
                        environment,
                        &secret.id,
                        Field::Name,
                    ),
                )?,
            );
            values.insert(
                environment.clone(),
                encrypt_environment_value(
                    value,
                    environment_secret,
                    &associated_data::secret_field(
                        project_id,
                        environment,
                        &secret.id,
                        Field::Value,
                    ),
                )?,
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

fn decrypt_user_secrets(
    project_id: &str,
    stored_secrets: &[StoredUserSecretRecord],
    user_environment_secrets: &BTreeMap<String, BTreeMap<String, [u8; 32]>>,
) -> Result<Vec<UserSecretRecord>> {
    let mut secrets = Vec::new();
    for stored in stored_secrets {
        let owner = &stored.owner_user_key_id;
        let mut values = BTreeMap::new();
        let mut key = None;
        for (environment, encrypted) in &stored.values {
            let secret = user_environment_secrets
                .get(owner)
                .and_then(|items| items.get(environment))
                .ok_or_else(|| FatimaError::message("Missing Fatima user environment secret."))?;
            if key.is_none() {
                if let Some(encrypted_key) = stored.keys.get(environment) {
                    key = Some(decrypt_environment_value(
                        encrypted_key,
                        secret,
                        &associated_data::user_secret_field(
                            project_id,
                            environment,
                            owner,
                            &stored.id,
                            Field::Name,
                        ),
                    )?);
                }
            }
            values.insert(
                environment.clone(),
                decrypt_environment_value(
                    encrypted,
                    secret,
                    &associated_data::user_secret_field(
                        project_id,
                        environment,
                        owner,
                        &stored.id,
                        Field::Value,
                    ),
                )?,
            );
        }
        secrets.push(UserSecretRecord {
            id: stored.id.clone(),
            owner_user_key_id: owner.clone(),
            key: key
                .ok_or_else(|| FatimaError::message("Missing encrypted Fatima user secret key."))?,
            values,
        });
    }
    Ok(secrets)
}

fn encrypt_user_secrets(
    project_id: &str,
    secrets: &[UserSecretRecord],
    user_environment_secrets: &BTreeMap<String, BTreeMap<String, [u8; 32]>>,
) -> Result<Vec<StoredUserSecretRecord>> {
    let mut stored = Vec::new();
    for secret in secrets {
        let owner = &secret.owner_user_key_id;
        let mut keys = BTreeMap::new();
        let mut values = BTreeMap::new();
        for (environment, value) in &secret.values {
            let user_environment_secret = user_environment_secrets
                .get(owner)
                .and_then(|items| items.get(environment))
                .ok_or_else(|| FatimaError::message("Missing Fatima user environment secret."))?;
            keys.insert(
                environment.clone(),
                encrypt_environment_value(
                    &secret.key,
                    user_environment_secret,
                    &associated_data::user_secret_field(
                        project_id,
                        environment,
                        owner,
                        &secret.id,
                        Field::Name,
                    ),
                )?,
            );
            values.insert(
                environment.clone(),
                encrypt_environment_value(
                    value,
                    user_environment_secret,
                    &associated_data::user_secret_field(
                        project_id,
                        environment,
                        owner,
                        &secret.id,
                        Field::Value,
                    ),
                )?,
            );
        }
        stored.push(StoredUserSecretRecord {
            id: secret.id.clone(),
            owner_user_key_id: owner.clone(),
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

#[derive(Deserialize)]
struct VersionProbe {
    version: u8,
}

fn parse_json<T: for<'de> Deserialize<'de>>(content: &str, label: &str) -> Result<T> {
    serde_json::from_str(content)
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
