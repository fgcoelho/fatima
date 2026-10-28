use std::collections::BTreeMap;

use crate::env::Secrets;
use crate::{FatimaError, Result};

use super::crypto::{decrypt_bytes, derive_key, encode_b64, encrypt_bytes, random_b64};
use super::domain::{assert_environment, to_environment_map};
use super::store::{
    assert_supported_store, decrypt_environment_value, environment_names, read_store,
};
use super::types::{
    GeneratedUserKey, UserKeyRecord, VaultOptions, WrappedUserEnvironmentSecret, KEY_LENGTH,
    USER_KEY_ID_LENGTH, USER_KEY_PREFIX,
};

pub(crate) fn list_secrets_with_user_key(
    raw_key: &str,
    environment: Option<&str>,
    options: Option<&VaultOptions>,
) -> Result<Secrets> {
    let auth = authenticate_user_key(raw_key, options)?;
    let store = auth.store;
    let environment_secrets = auth.environment_secrets;
    let environment = environment.unwrap_or(&store.file.default_environment);
    let all_environments = environment_names(&store.file);
    if !all_environments.iter().any(|value| value == environment) {
        return Err(FatimaError::message(format!(
            "Unknown Fatima environment: {environment}"
        )));
    }
    let Some(environment_secret) = environment_secrets.get(environment) else {
        return Err(FatimaError::message(format!(
            "Fatima user key cannot access environment: {environment}"
        )));
    };
    let mut secrets = Vec::new();
    for stored in &store.file.secrets {
        let Some(encrypted) = stored.values.get(environment) else {
            continue;
        };
        let encrypted_key = stored
            .keys
            .get(environment)
            .ok_or_else(|| FatimaError::message("Missing encrypted Fatima secret key."))?;
        let value = decrypt_environment_value(encrypted, environment_secret)?;
        secrets.push(super::types::SecretRecord {
            id: stored.id.clone(),
            key: decrypt_environment_value(encrypted_key, environment_secret)?,
            values: BTreeMap::from([(environment.to_string(), value)]),
        });
    }
    if let Some(user_environment_secret) = auth.user_environment_secrets.get(environment) {
        for stored in &store.file.user_secrets {
            if stored.owner_user_key_id != auth.user_key_id {
                continue;
            }
            let Some(encrypted) = stored.values.get(environment) else {
                continue;
            };
            let encrypted_key = stored
                .keys
                .get(environment)
                .ok_or_else(|| FatimaError::message("Missing encrypted Fatima user secret key."))?;
            let key = decrypt_environment_value(encrypted_key, user_environment_secret)?;
            if secrets.iter().any(|secret| secret.key == key) {
                return Err(FatimaError::message(format!(
                    "Fatima user secret `{key}` conflicts with a shared secret."
                )));
            }
            secrets.push(super::types::SecretRecord {
                id: stored.id.clone(),
                key,
                values: BTreeMap::from([(
                    environment.to_string(),
                    decrypt_environment_value(encrypted, user_environment_secret)?,
                )]),
            });
        }
    }
    Ok(to_environment_map(&secrets, environment))
}

pub(crate) struct AuthenticatedUserKey {
    pub store: super::types::ProjectStore,
    pub user_key_id: String,
    pub environments: Vec<String>,
    pub environment_secrets: BTreeMap<String, Vec<u8>>,
    pub user_environment_secrets: BTreeMap<String, Vec<u8>>,
}

pub(crate) fn authenticate_user_key(
    raw_key: &str,
    options: Option<&VaultOptions>,
) -> Result<AuthenticatedUserKey> {
    let store = read_store(options)?;
    assert_supported_store(&store)?;
    let (id, secret) = parse_raw_user_key(raw_key)?;
    let record = store
        .file
        .user_keys
        .iter()
        .find(|record| record.id == id)
        .ok_or_else(|| FatimaError::message("Fatima user key not found."))?;
    let user_key = derive_key(&secret, &record.salt)?;
    let derived = encode_b64(&user_key);
    if derived != record.secret_hash {
        return Err(FatimaError::message("Invalid Fatima user key."));
    }
    let mut environment_secrets = BTreeMap::new();
    for (environment, wrapped) in &record.wrapped_environment_secrets {
        environment_secrets.insert(environment.clone(), decrypt_bytes(wrapped, &user_key)?);
    }
    let mut user_environment_secrets = BTreeMap::new();
    for (environment, wrapped) in &record.wrapped_user_environment_secrets {
        user_environment_secrets.insert(
            environment.clone(),
            decrypt_bytes(&wrapped.by_user_key, &user_key)?,
        );
    }
    Ok(AuthenticatedUserKey {
        store,
        user_key_id: id,
        environments: environment_secrets.keys().cloned().collect(),
        environment_secrets,
        user_environment_secrets,
    })
}

pub(crate) fn create_user_key_record(
    name: &str,
    environments: &[String],
    environment_secrets: &BTreeMap<String, Vec<u8>>,
    master_password_key: &[u8],
) -> Result<GeneratedUserKey> {
    let (id, secret, raw_key) = create_raw_user_key();
    let salt = random_b64(16);
    let user_key = derive_key(&secret, &salt)?;
    let secret_hash = encode_b64(&user_key);
    let mut wrapped_environment_secrets = BTreeMap::new();
    let mut wrapped_user_environment_secrets = BTreeMap::new();
    for environment in environments {
        let environment_secret = environment_secrets.get(environment).ok_or_else(|| {
            FatimaError::message(format!("Missing Fatima environment secret: {environment}"))
        })?;
        wrapped_environment_secrets.insert(
            environment.clone(),
            encrypt_bytes(environment_secret, &user_key)?,
        );
        let user_environment_secret = super::crypto::random_bytes(KEY_LENGTH);
        wrapped_user_environment_secrets.insert(
            environment.clone(),
            WrappedUserEnvironmentSecret {
                by_master_password: encrypt_bytes(&user_environment_secret, master_password_key)?,
                by_user_key: encrypt_bytes(&user_environment_secret, &user_key)?,
            },
        );
    }
    let record = UserKeyRecord {
        id,
        name: name.to_string(),
        salt,
        secret_hash,
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
    let secret = random_b64(24);
    let raw_key = format!("{USER_KEY_PREFIX}{id}_{secret}");
    (id, secret, raw_key)
}

fn is_base64urlish(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}
