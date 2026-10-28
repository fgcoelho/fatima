use std::fmt;

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use crypto_box::{PublicKey, SecretKey};
use hkdf::Hkdf;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

use crate::{FatimaError, Result};

const MAX_MEMORY_KIB: u32 = 1_048_576;
const MAX_ITERATIONS: u32 = 16;
const MAX_PARALLELISM: u32 = 16;
const NONCE_LENGTH: usize = 24;
const ENCRYPTION_INFO: &[u8] = b"fatima/v2/encryption";
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
    if !(1..=MAX_MEMORY_KIB).contains(&params.memory_kib)
        || !(1..=MAX_ITERATIONS).contains(&params.iterations)
        || !(1..=MAX_PARALLELISM).contains(&params.parallelism)
    {
        return Err(FatimaError::InvalidKdfParams);
    }
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

pub(crate) fn derive_encryption_key(master: &[u8; 32]) -> Result<[u8; 32]> {
    expand(&Hkdf::<Sha256>::new(None, master), ENCRYPTION_INFO)
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

pub(crate) fn random_bytes(length: usize) -> Vec<u8> {
    let mut bytes = vec![0; length];
    OsRng.fill_bytes(&mut bytes);
    bytes
}

pub(crate) fn random_b64(length: usize) -> String {
    encode_b64(&random_bytes(length))
}

pub(crate) fn encode_b64(value: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(value)
}

pub(crate) fn decode_b64(value: &str) -> Result<Vec<u8>> {
    URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|error| FatimaError::message(error.to_string()))
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
    fn derive_encryption_key_differs_from_master_and_is_deterministic() {
        let master = random_key();
        let key = derive_encryption_key(&master).unwrap();
        assert_ne!(key, master);
        assert_eq!(key, derive_encryption_key(&master).unwrap());
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

    #[test]
    fn derive_master_key_rejects_out_of_range_params() {
        let valid = KdfParams {
            memory_kib: 1024,
            iterations: 1,
            parallelism: 1,
        };
        let invalid = [
            KdfParams {
                memory_kib: 0,
                ..valid.clone()
            },
            KdfParams {
                memory_kib: 1_048_577,
                ..valid.clone()
            },
            KdfParams {
                iterations: 0,
                ..valid.clone()
            },
            KdfParams {
                iterations: 17,
                ..valid.clone()
            },
            KdfParams {
                parallelism: 0,
                ..valid.clone()
            },
            KdfParams {
                parallelism: 17,
                ..valid.clone()
            },
        ];
        for params in invalid {
            assert!(matches!(
                derive_master_key("password", b"0123456789abcdef", &params),
                Err(FatimaError::InvalidKdfParams)
            ));
        }
    }
}
