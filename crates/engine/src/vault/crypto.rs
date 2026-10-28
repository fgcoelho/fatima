use std::fmt;

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use crypto_box::{PublicKey, SecretKey};
use hkdf::Hkdf;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::{FatimaError, Result};

use super::legacy::crypto::{decode_b64, encode_b64};

const NONCE_LENGTH: usize = 24;
const ENCRYPTION_INFO: &[u8] = b"fatima/v2/encryption";
const LOGIN_INFO: &[u8] = b"fatima/v2/login";
const USER_KEY_WRAP_INFO: &[u8] = b"fatima/v2/user-key/wrap";
const USER_KEY_HASH_INFO: &[u8] = b"fatima/v2/user-key/hash";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KdfParams {
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            memory_kib: 65536,
            iterations: 3,
            parallelism: 4,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ciphertext {
    pub nonce: String,
    pub ciphertext: String,
}

pub(crate) struct MasterKeys {
    pub encryption: [u8; 32],
    pub login: [u8; 32],
}

pub(crate) struct UserKeyKeys {
    pub wrapping: [u8; 32],
    pub hash: [u8; 32],
}

#[derive(Clone)]
pub(crate) struct KeyPair {
    pub public: [u8; 32],
    pub secret: [u8; 32],
}

impl fmt::Debug for KeyPair {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("KeyPair").finish_non_exhaustive()
    }
}

pub(crate) fn derive_master_key(
    password: &str,
    salt: &[u8],
    params: &KdfParams,
) -> Result<[u8; 32]> {
    let params = Params::new(
        params.memory_kib,
        params.iterations,
        params.parallelism,
        Some(32),
    )
    .map_err(|error| FatimaError::message(error.to_string()))?;
    let mut key = [0u8; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|error| FatimaError::message(error.to_string()))?;
    Ok(key)
}

pub(crate) fn split_master_key(master: &[u8; 32]) -> Result<MasterKeys> {
    let hkdf = Hkdf::<Sha256>::new(None, master);
    Ok(MasterKeys {
        encryption: expand(&hkdf, ENCRYPTION_INFO)?,
        login: expand(&hkdf, LOGIN_INFO)?,
    })
}

pub(crate) fn derive_user_key(secret: &[u8]) -> Result<UserKeyKeys> {
    let hkdf = Hkdf::<Sha256>::new(None, secret);
    Ok(UserKeyKeys {
        wrapping: expand(&hkdf, USER_KEY_WRAP_INFO)?,
        hash: expand(&hkdf, USER_KEY_HASH_INFO)?,
    })
}

fn expand(hkdf: &Hkdf<Sha256>, info: &[u8]) -> Result<[u8; 32]> {
    let mut key = [0u8; 32];
    hkdf.expand(info, &mut key)
        .map_err(|_| FatimaError::message("Failed to derive Fatima vault key."))?;
    Ok(key)
}

pub(crate) fn encrypt(
    plaintext: &[u8],
    key: &[u8; 32],
    associated_data: &[u8],
) -> Result<Ciphertext> {
    let mut nonce = [0u8; NONCE_LENGTH];
    OsRng.fill_bytes(&mut nonce);
    let ciphertext = XChaCha20Poly1305::new(key.into())
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad: associated_data,
            },
        )
        .map_err(|_| FatimaError::message("Failed to encrypt Fatima vault data."))?;
    Ok(Ciphertext {
        nonce: encode_b64(&nonce),
        ciphertext: encode_b64(&ciphertext),
    })
}

pub(crate) fn decrypt(
    value: &Ciphertext,
    key: &[u8; 32],
    associated_data: &[u8],
) -> Result<Vec<u8>> {
    let nonce = decode_b64(&value.nonce).map_err(|_| FatimaError::Decrypt)?;
    if nonce.len() != NONCE_LENGTH {
        return Err(FatimaError::Decrypt);
    }
    let ciphertext = decode_b64(&value.ciphertext).map_err(|_| FatimaError::Decrypt)?;
    XChaCha20Poly1305::new(key.into())
        .decrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad: associated_data,
            },
        )
        .map_err(|_| FatimaError::Decrypt)
}

pub(crate) fn generate_key_pair() -> KeyPair {
    let secret = SecretKey::generate(&mut OsRng);
    KeyPair {
        public: secret.public_key().to_bytes(),
        secret: secret.to_bytes(),
    }
}

pub(crate) fn seal_to(plaintext: &[u8], public: &[u8; 32]) -> Result<String> {
    let sealed = PublicKey::from(*public)
        .seal(&mut OsRng, plaintext)
        .map_err(|_| FatimaError::message("Failed to seal Fatima vault key."))?;
    Ok(encode_b64(&sealed))
}

pub(crate) fn open_sealed(sealed: &str, key_pair: &KeyPair) -> Result<Vec<u8>> {
    SecretKey::from_bytes(key_pair.secret)
        .unseal(&decode_b64(sealed).map_err(|_| FatimaError::Decrypt)?)
        .map_err(|_| FatimaError::Decrypt)
}

pub(crate) fn random_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    key
}

pub(crate) fn to_key(bytes: &[u8]) -> Result<[u8; 32]> {
    bytes.try_into().map_err(|_| FatimaError::Decrypt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_then_decrypt_round_trips() {
        let key = random_key();
        let sealed = encrypt(b"plaintext", &key, b"context").unwrap();
        assert_eq!(decrypt(&sealed, &key, b"context").unwrap(), b"plaintext");
    }

    #[test]
    fn decrypt_with_other_associated_data_fails() {
        let key = random_key();
        let sealed = encrypt(b"plaintext", &key, b"context").unwrap();
        assert!(matches!(
            decrypt(&sealed, &key, b"other"),
            Err(FatimaError::Decrypt)
        ));
    }

    #[test]
    fn decrypt_with_other_key_fails() {
        let sealed = encrypt(b"plaintext", &random_key(), b"context").unwrap();
        assert!(matches!(
            decrypt(&sealed, &random_key(), b"context"),
            Err(FatimaError::Decrypt)
        ));
    }

    #[test]
    fn sealed_box_opens_only_with_its_key_pair() {
        let owner = generate_key_pair();
        let stranger = generate_key_pair();
        let sealed = seal_to(b"environment key", &owner.public).unwrap();
        assert_eq!(open_sealed(&sealed, &owner).unwrap(), b"environment key");
        assert!(matches!(
            open_sealed(&sealed, &stranger),
            Err(FatimaError::Decrypt)
        ));
    }

    #[test]
    fn split_master_key_gives_distinct_keys() {
        let master = random_key();
        let keys = split_master_key(&master).unwrap();
        assert_ne!(keys.encryption, keys.login);
        assert_ne!(keys.encryption, master);
        assert_ne!(keys.login, master);
    }

    #[test]
    fn derive_master_key_is_deterministic_for_salt_and_params() {
        let params = KdfParams {
            memory_kib: 1024,
            iterations: 1,
            parallelism: 1,
        };
        let first = derive_master_key("password", b"0123456789abcdef", &params).unwrap();
        let second = derive_master_key("password", b"0123456789abcdef", &params).unwrap();
        let other_salt = derive_master_key("password", b"fedcba9876543210", &params).unwrap();
        let other_password = derive_master_key("other", b"0123456789abcdef", &params).unwrap();
        assert_eq!(first, second);
        assert_ne!(first, other_salt);
        assert_ne!(first, other_password);
    }
}
