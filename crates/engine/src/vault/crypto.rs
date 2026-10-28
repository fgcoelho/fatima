use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::{rngs::OsRng, RngCore};
use scrypt::{scrypt, Params};

use crate::{FatimaError, Result};

use super::types::{EncryptedBlob, KEY_LENGTH};

pub(crate) fn encrypt_bytes(value: &[u8], key: &[u8]) -> Result<EncryptedBlob> {
    let cipher = cipher(key)?;
    let iv = random_bytes(12);
    let encrypted = cipher
        .encrypt(Nonce::from_slice(&iv), value)
        .map_err(|_| FatimaError::message("Failed to encrypt Fatima vault data."))?;
    let tag_start = encrypted.len().saturating_sub(16);
    Ok(EncryptedBlob {
        iv: encode_b64(&iv),
        tag: encode_b64(&encrypted[tag_start..]),
        ciphertext: encode_b64(&encrypted[..tag_start]),
    })
}

pub(crate) fn decrypt_bytes(value: &EncryptedBlob, key: &[u8]) -> Result<Vec<u8>> {
    let cipher = cipher(key)?;
    let mut encrypted = decode_b64(&value.ciphertext)?;
    encrypted.extend(decode_b64(&value.tag)?);
    cipher
        .decrypt(
            Nonce::from_slice(&decode_b64(&value.iv)?),
            encrypted.as_slice(),
        )
        .map_err(|_| FatimaError::message("Failed to decrypt Fatima vault data."))
}

pub(crate) fn derive_key(secret: &str, salt: &str) -> Result<Vec<u8>> {
    let params = Params::new(14, 8, 1, KEY_LENGTH)
        .map_err(|error| FatimaError::message(error.to_string()))?;
    let mut key = vec![0; KEY_LENGTH];
    scrypt(secret.as_bytes(), &decode_b64(salt)?, &params, &mut key)
        .map_err(|error| FatimaError::message(error.to_string()))?;
    Ok(key)
}

pub(crate) fn random_bytes(length: usize) -> Vec<u8> {
    let mut bytes = vec![0; length];
    OsRng.fill_bytes(&mut bytes);
    bytes
}

pub(crate) fn random_b64(length: usize) -> String {
    encode_b64(&random_bytes(length))
}

fn cipher(key: &[u8]) -> Result<Aes256Gcm> {
    Aes256Gcm::new_from_slice(key)
        .map_err(|_| FatimaError::message("Invalid Fatima vault encryption key."))
}

pub(crate) fn encode_b64(value: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(value)
}

pub(crate) fn decode_b64(value: &str) -> Result<Vec<u8>> {
    URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|error| FatimaError::message(error.to_string()))
}
