use std::collections::BTreeMap;

use crate::{FatimaError, Result};

use super::associated_data;
use super::crypto::{
    decrypt, encrypt, generate_key_pair, open_sealed, random_key, seal_to, derive_encryption_key,
    to_key, Ciphertext, KdfParams, KeyPair,
};
use super::legacy::crypto::{decode_b64, encode_b64};
use super::types::{AccountRecord, EnvironmentGrant, EnvironmentRecord, UserKeyRecord};

#[derive(Clone, Debug)]
pub(crate) struct AccountSecrets {
    pub account_key: [u8; 32],
    pub key_pair: KeyPair,
    pub master_encryption_key: [u8; 32],
    pub kdf_salt: Vec<u8>,
    pub kdf_params: KdfParams,
}

pub(crate) fn new_account_secrets(
    master_key: &[u8; 32],
    kdf_salt: Vec<u8>,
    kdf_params: KdfParams,
) -> Result<AccountSecrets> {
    Ok(AccountSecrets {
        account_key: random_key(),
        key_pair: generate_key_pair(),
        master_encryption_key: derive_encryption_key(master_key)?,
        kdf_salt,
        kdf_params,
    })
}

pub(crate) fn wrap_account(project_id: &str, secrets: &AccountSecrets) -> Result<AccountRecord> {
    Ok(AccountRecord {
        wrapped_account_key: encrypt(
            &secrets.account_key,
            &secrets.master_encryption_key,
            &associated_data::account_key(project_id),
        )?,
        public_key: encode_b64(&secrets.key_pair.public),
        wrapped_private_key: encrypt(
            &secrets.key_pair.secret,
            &secrets.account_key,
            &associated_data::private_key(project_id),
        )?,
    })
}

pub(crate) fn unwrap_account(
    record: &AccountRecord,
    project_id: &str,
    master_encryption_key: [u8; 32],
    kdf_salt: Vec<u8>,
    kdf_params: KdfParams,
) -> Result<AccountSecrets> {
    let account_key = to_key(
        &decrypt(
            &record.wrapped_account_key,
            &master_encryption_key,
            &associated_data::account_key(project_id),
        )
        .map_err(|error| match error {
            FatimaError::Decrypt => FatimaError::WrongMasterPassword,
            other => other,
        })?,
    )?;
    let secret = to_key(&decrypt(
        &record.wrapped_private_key,
        &account_key,
        &associated_data::private_key(project_id),
    )?)?;
    let public = to_key(&decode_b64(&record.public_key).map_err(|_| FatimaError::Decrypt)?)?;
    Ok(AccountSecrets {
        account_key,
        key_pair: KeyPair { public, secret },
        master_encryption_key,
        kdf_salt,
        kdf_params,
    })
}

fn grant_environment_key(
    environment_key: &[u8; 32],
    key_pair: &KeyPair,
) -> Result<EnvironmentGrant> {
    Ok(EnvironmentGrant {
        public_key: encode_b64(&key_pair.public),
        sealed_key: seal_to(environment_key, &key_pair.public)?,
    })
}

pub(crate) fn grant_environments(
    names: &[String],
    environment_keys: &BTreeMap<String, [u8; 32]>,
    existing: &[EnvironmentRecord],
    key_pair: &KeyPair,
) -> Result<Vec<EnvironmentRecord>> {
    names
        .iter()
        .map(|name| {
            if let Some(record) = existing
                .iter()
                .find(|record| &record.name == name && !record.grants.is_empty())
            {
                return Ok(record.clone());
            }
            let key = environment_keys.get(name).ok_or_else(|| {
                FatimaError::message(format!("Missing Fatima environment secret: {name}"))
            })?;
            Ok(EnvironmentRecord {
                name: name.clone(),
                grants: vec![grant_environment_key(key, key_pair)?],
            })
        })
        .collect()
}

pub(crate) fn unwrap_environment_keys(
    environments: &[EnvironmentRecord],
    key_pair: &KeyPair,
) -> Result<BTreeMap<String, [u8; 32]>> {
    let public_key = encode_b64(&key_pair.public);
    environments
        .iter()
        .map(|environment| {
            let grant = environment
                .grants
                .iter()
                .find(|grant| grant.public_key == public_key)
                .ok_or(FatimaError::Decrypt)?;
            Ok((
                environment.name.clone(),
                to_key(&open_sealed(&grant.sealed_key, key_pair)?)?,
            ))
        })
        .collect()
}

pub(crate) fn unwrap_user_environment_keys(
    user_keys: &[UserKeyRecord],
    project_id: &str,
    account_key: &[u8; 32],
) -> Result<BTreeMap<String, BTreeMap<String, [u8; 32]>>> {
    let mut result = BTreeMap::new();
    for user_key in user_keys {
        let mut by_environment = BTreeMap::new();
        for (environment, wrapped) in &user_key.wrapped_user_environment_secrets {
            by_environment.insert(
                environment.clone(),
                unwrap_user_environment_key(
                    &wrapped.by_account_key,
                    account_key,
                    project_id,
                    &user_key.id,
                )?,
            );
        }
        result.insert(user_key.id.clone(), by_environment);
    }
    Ok(result)
}

pub(crate) fn unwrap_user_environment_key(
    wrapped: &Ciphertext,
    key: &[u8; 32],
    project_id: &str,
    user_key_id: &str,
) -> Result<[u8; 32]> {
    to_key(&decrypt(
        wrapped,
        key,
        &associated_data::user_environment(project_id, user_key_id),
    )?)
}
