use std::collections::BTreeMap;

use crate::env::Secrets;
use crate::{FatimaError, Result};

use super::account::unwrap_user_environment_key;
use super::associated_data::{self, Field};
use super::crypto::{
    decrypt, derive_user_key, encode_b64, encrypt, random_b64, random_key, to_key,
};
use super::domain::{assert_environment, to_environment_map};
use super::legacy;
use super::store::{decrypt_environment_value, read_store};
use super::types::{
    GeneratedUserKey, SecretRecord, StoredVaultFile, UserKeyRecord, UserKeyScheme, VaultFile,
    VaultOptions, WrappedKey, WrappedUserEnvironmentSecret, USER_KEY_ID_LENGTH, USER_KEY_PREFIX,
};

pub(crate) fn list_secrets_with_user_key(
    raw_key: &str,
    environment: Option<&str>,
    options: Option<&VaultOptions>,
) -> Result<Secrets> {
    let auth = authenticate_user_key(raw_key, options)?;
    let environment = environment.unwrap_or_else(|| auth.store.file.default_environment());
    if !auth
        .store
        .file
        .environment_names()
        .iter()
        .any(|value| value == environment)
    {
        return Err(FatimaError::message(format!(
            "Unknown Fatima environment: {environment}"
        )));
    }
    let Some(environment_secret) = auth.environment_secrets.get(environment) else {
        return Err(FatimaError::message(format!(
            "Fatima user key cannot access environment: {environment}"
        )));
    };
    let user = auth
        .user_environment_secrets
        .get(environment)
        .map(|secret| (auth.user_key_id.as_str(), secret));
    let (mut secrets, user_secrets) = match &auth.store.file {
        VaultFile::V1(file) => legacy::user_key_secrets(file, environment, environment_secret, user)?,
        VaultFile::V2(file) => v2_user_key_secrets(file, environment, environment_secret, user)?,
    };
    for secret in user_secrets {
        if secrets.iter().any(|shared| shared.key == secret.key) {
            return Err(FatimaError::message(format!(
                "Fatima user secret `{}` conflicts with a shared secret.",
                secret.key
            )));
        }
        secrets.push(secret);
    }
    Ok(to_environment_map(&secrets, environment))
}

fn v2_user_key_secrets(
    file: &StoredVaultFile,
    environment: &str,
    environment_secret: &[u8; 32],
    user: Option<(&str, &[u8; 32])>,
) -> Result<(Vec<SecretRecord>, Vec<SecretRecord>)> {
    let project_id = &file.project_id;
    let mut shared = Vec::new();
    for stored in &file.secrets {
        let Some(encrypted) = stored.values.get(environment) else {
            continue;
        };
        let encrypted_key = stored
            .keys
            .get(environment)
            .ok_or_else(|| FatimaError::message("Missing encrypted Fatima secret key."))?;
        shared.push(SecretRecord {
            id: stored.id.clone(),
            key: decrypt_environment_value(
                encrypted_key,
                environment_secret,
                &associated_data::secret_field(project_id, environment, &stored.id, Field::Name),
            )?,
            values: BTreeMap::from([(
                environment.to_string(),
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
            )]),
        });
    }
    let mut own = Vec::new();
    if let Some((user_key_id, user_environment_secret)) = user {
        for stored in &file.user_secrets {
            if stored.owner_user_key_id != user_key_id {
                continue;
            }
            let Some(encrypted) = stored.values.get(environment) else {
                continue;
            };
            let encrypted_key = stored
                .keys
                .get(environment)
                .ok_or_else(|| FatimaError::message("Missing encrypted Fatima user secret key."))?;
            own.push(SecretRecord {
                id: stored.id.clone(),
                key: decrypt_environment_value(
                    encrypted_key,
                    user_environment_secret,
                    &associated_data::user_secret_field(
                        project_id,
                        environment,
                        user_key_id,
                        &stored.id,
                        Field::Name,
                    ),
                )?,
                values: BTreeMap::from([(
                    environment.to_string(),
                    decrypt_environment_value(
                        encrypted,
                        user_environment_secret,
                        &associated_data::user_secret_field(
                            project_id,
                            environment,
                            user_key_id,
                            &stored.id,
                            Field::Value,
                        ),
                    )?,
                )]),
            });
        }
    }
    Ok((shared, own))
}

pub(crate) struct AuthenticatedUserKey {
    pub store: super::types::ProjectStore,
    pub user_key_id: String,
    pub environments: Vec<String>,
    pub environment_secrets: BTreeMap<String, [u8; 32]>,
    pub user_environment_secrets: BTreeMap<String, [u8; 32]>,
}

pub(crate) fn authenticate_user_key(
    raw_key: &str,
    options: Option<&VaultOptions>,
) -> Result<AuthenticatedUserKey> {
    let store = read_store(options)?;
    let (id, secret) = parse_raw_user_key(raw_key)?;
    let not_found = || FatimaError::message("Fatima user key not found.");
    let secrets = match &store.file {
        VaultFile::V1(file) => {
            let record = file
                .user_keys
                .iter()
                .find(|record| record.id == id)
                .ok_or_else(not_found)?;
            legacy::authenticate_user_key(
                &secret,
                &record.salt,
                &record.secret_hash,
                &record.wrapped_environment_secrets,
                record
                    .wrapped_user_environment_secrets
                    .iter()
                    .map(|(environment, wrapped)| (environment, &wrapped.by_user_key)),
            )?
        }
        VaultFile::V2(file) => {
            let record = file
                .user_keys
                .iter()
                .find(|record| record.id == id)
                .ok_or_else(not_found)?;
            match record.scheme {
                UserKeyScheme::V1 => authenticate_v1_scheme(record, &secret)?,
                UserKeyScheme::V2 => authenticate_v2_scheme(record, &file.project_id, &secret)?,
            }
        }
    };
    Ok(AuthenticatedUserKey {
        store,
        user_key_id: id,
        environments: secrets.environment_secrets.keys().cloned().collect(),
        environment_secrets: secrets.environment_secrets,
        user_environment_secrets: secrets.user_environment_secrets,
    })
}

fn authenticate_v1_scheme(record: &UserKeyRecord, secret: &str) -> Result<legacy::UserKeySecrets> {
    let salt = record.salt.as_deref().ok_or(FatimaError::Decrypt)?;
    let environments = record
        .wrapped_environment_secrets
        .iter()
        .map(|(environment, wrapped)| -> Result<_> { Ok((environment, wrapped.v1()?)) })
        .collect::<Result<Vec<_>>>()?;
    let user_environments = record
        .wrapped_user_environment_secrets
        .iter()
        .map(|(environment, wrapped)| -> Result<_> { Ok((environment, wrapped.by_user_key.v1()?)) })
        .collect::<Result<Vec<_>>>()?;
    legacy::authenticate_user_key(
        secret,
        salt,
        &record.secret_hash,
        environments,
        user_environments,
    )
}

fn authenticate_v2_scheme(
    record: &UserKeyRecord,
    project_id: &str,
    secret: &str,
) -> Result<legacy::UserKeySecrets> {
    let keys = derive_user_key(secret.as_bytes())?;
    if encode_b64(&keys.hash) != record.secret_hash {
        return Err(FatimaError::message("Invalid Fatima user key."));
    }
    let mut environment_secrets = BTreeMap::new();
    for (environment, wrapped) in &record.wrapped_environment_secrets {
        environment_secrets.insert(
            environment.clone(),
            to_key(&decrypt(
                wrapped.v2()?,
                &keys.wrapping,
                &associated_data::user_key_environment(project_id, &record.id),
            )?)?,
        );
    }
    let mut user_environment_secrets = BTreeMap::new();
    for (environment, wrapped) in &record.wrapped_user_environment_secrets {
        user_environment_secrets.insert(
            environment.clone(),
            unwrap_user_environment_key(
                wrapped.by_user_key.v2()?,
                &keys.wrapping,
                project_id,
                &record.id,
            )?,
        );
    }
    Ok(legacy::UserKeySecrets {
        environment_secrets,
        user_environment_secrets,
    })
}

pub(crate) fn create_user_key_record(
    project_id: &str,
    name: &str,
    environments: &[String],
    environment_secrets: &BTreeMap<String, [u8; 32]>,
    account_key: &[u8; 32],
) -> Result<GeneratedUserKey> {
    let (id, secret, raw_key) = create_raw_user_key();
    let keys = derive_user_key(secret.as_bytes())?;
    let mut wrapped_environment_secrets = BTreeMap::new();
    let mut wrapped_user_environment_secrets = BTreeMap::new();
    for environment in environments {
        let environment_secret = environment_secrets.get(environment).ok_or_else(|| {
            FatimaError::message(format!("Missing Fatima environment secret: {environment}"))
        })?;
        wrapped_environment_secrets.insert(
            environment.clone(),
            WrappedKey::V2(encrypt(
                environment_secret,
                &keys.wrapping,
                &associated_data::user_key_environment(project_id, &id),
            )?),
        );
        let user_environment_secret = random_key();
        let user_environment_data = associated_data::user_environment(project_id, &id);
        wrapped_user_environment_secrets.insert(
            environment.clone(),
            WrappedUserEnvironmentSecret {
                by_account_key: encrypt(
                    &user_environment_secret,
                    account_key,
                    &user_environment_data,
                )?,
                by_user_key: WrappedKey::V2(encrypt(
                    &user_environment_secret,
                    &keys.wrapping,
                    &user_environment_data,
                )?),
            },
        );
    }
    let record = UserKeyRecord {
        id,
        name: name.to_string(),
        scheme: UserKeyScheme::V2,
        salt: None,
        secret_hash: encode_b64(&keys.hash),
        wrapped_environment_secrets,
        wrapped_user_environment_secrets,
    };
    Ok(GeneratedUserKey {
        key: raw_key,
        record,
    })
}

pub(crate) fn assert_user_key_environments(
    all_environments: &[String],
    environments: &[String],
) -> Result<()> {
    for environment in environments {
        assert_environment(all_environments, environment)?;
    }
    Ok(())
}

fn parse_raw_user_key(raw_key: &str) -> Result<(String, String)> {
    let value = raw_key.trim();
    if !value.starts_with(USER_KEY_PREFIX) {
        return Err(FatimaError::message("Invalid Fatima user key format."));
    }
    let payload = &value[USER_KEY_PREFIX.len()..];
    if payload.len() <= USER_KEY_ID_LENGTH || payload.as_bytes()[USER_KEY_ID_LENGTH] != b'_' {
        return Err(FatimaError::message("Invalid Fatima user key format."));
    }
    let id = &payload[..USER_KEY_ID_LENGTH];
    let secret = &payload[USER_KEY_ID_LENGTH + 1..];
    if !is_base64urlish(id) || !is_base64urlish(secret) {
        return Err(FatimaError::message("Invalid Fatima user key format."));
    }
    Ok((id.to_string(), secret.to_string()))
}

fn create_raw_user_key() -> (String, String, String) {
    let id = random_b64(8);
    let secret = random_b64(32);
    let raw_key = format!("{USER_KEY_PREFIX}{id}_{secret}");
    (id, secret, raw_key)
}

fn is_base64urlish(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}
