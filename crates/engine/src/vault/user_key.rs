use std::collections::BTreeMap;

use crate::env::Secrets;
use crate::{FatimaError, Result};

use super::account::unwrap_user_environment_key;
use super::associated_data::{self, Field};
use super::crypto::{decrypt, derive_user_key, encrypt, random_key, to_key};
use super::domain::{assert_environment, to_environment_map};
use super::legacy::crypto::{encode_b64, random_b64};
use super::store::{decrypt_environment_value, environment_names, read_store};
use super::types::{
    GeneratedUserKey, UserKeyRecord, VaultOptions, WrappedUserEnvironmentSecret,
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
    let project_id = &store.file.project_id;
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
        let value = decrypt_environment_value(
            encrypted,
            environment_secret,
            &associated_data::secret_field(project_id, environment, &stored.id, Field::Value),
        )?;
        secrets.push(super::types::SecretRecord {
            id: stored.id.clone(),
            key: decrypt_environment_value(
                encrypted_key,
                environment_secret,
                &associated_data::secret_field(project_id, environment, &stored.id, Field::Name),
            )?,
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
            let key = decrypt_environment_value(
                encrypted_key,
                user_environment_secret,
                &associated_data::user_secret_field(
                    project_id,
                    environment,
                    &auth.user_key_id,
                    &stored.id,
                    Field::Name,
                ),
            )?;
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
                    decrypt_environment_value(
                        encrypted,
                        user_environment_secret,
                        &associated_data::user_secret_field(
                            project_id,
                            environment,
                            &auth.user_key_id,
                            &stored.id,
                            Field::Value,
                        ),
                    )?,
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
    pub environment_secrets: BTreeMap<String, [u8; 32]>,
    pub user_environment_secrets: BTreeMap<String, [u8; 32]>,
}

pub(crate) fn authenticate_user_key(
    raw_key: &str,
    options: Option<&VaultOptions>,
) -> Result<AuthenticatedUserKey> {
    let store = read_store(options)?;
    let (id, secret) = parse_raw_user_key(raw_key)?;
    let record = store
        .file
        .user_keys
        .iter()
        .find(|record| record.id == id)
        .ok_or_else(|| FatimaError::message("Fatima user key not found."))?;
    let project_id = &store.file.project_id;
    let keys = derive_user_key(secret.as_bytes())?;
    if encode_b64(&keys.hash) != record.secret_hash {
        return Err(FatimaError::message("Invalid Fatima user key."));
    }
    let mut environment_secrets = BTreeMap::new();
    for (environment, wrapped) in &record.wrapped_environment_secrets {
        environment_secrets.insert(
            environment.clone(),
            to_key(&decrypt(
                wrapped,
                &keys.wrapping,
                &associated_data::user_key_environment(project_id, &id, environment),
            )?)?,
        );
    }
    let mut user_environment_secrets = BTreeMap::new();
    for (environment, wrapped) in &record.wrapped_user_environment_secrets {
        user_environment_secrets.insert(
            environment.clone(),
            unwrap_user_environment_key(
                &wrapped.by_user_key,
                &keys.wrapping,
                project_id,
                &id,
                environment,
            )?,
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
            encrypt(
                environment_secret,
                &keys.wrapping,
                &associated_data::user_key_environment(project_id, &id, environment),
            )?,
        );
        let user_environment_secret = random_key();
        let user_environment_data =
            associated_data::user_environment(project_id, &id, environment);
        wrapped_user_environment_secrets.insert(
            environment.clone(),
            WrappedUserEnvironmentSecret {
                by_account_key: encrypt(
                    &user_environment_secret,
                    account_key,
                    &user_environment_data,
                )?,
                by_user_key: encrypt(
                    &user_environment_secret,
                    &keys.wrapping,
                    &user_environment_data,
                )?,
            },
        );
    }
    let record = UserKeyRecord {
        id,
        name: name.to_string(),
        scheme: "v2".to_string(),
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
