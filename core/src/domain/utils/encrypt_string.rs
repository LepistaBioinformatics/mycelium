use crate::{domain::utils::derive_key_from_uuid, models::AccountLifeCycle};

use base64::{engine::general_purpose, Engine};
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use ring::{
    aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM},
    rand::{SecureRandom, SystemRandom},
};
use tracing::error;
use uuid::Uuid;

/// Encrypt a plain-text string with AES-256-GCM using the server's token
/// secret as key material.
///
/// Returns `base64(nonce ‖ ciphertext ‖ tag)` suitable for opaque storage
/// (e.g. tenant or account meta JSONB columns). Decryptable only with the
/// same `AccountLifeCycle::token_secret` that was active at write time.
#[tracing::instrument(name = "encrypt_string", skip_all)]
pub async fn encrypt_string(
    plain: &str,
    config: &AccountLifeCycle,
) -> Result<String, MappedErrors> {
    let key = build_aes_key(config).await?;

    let rand = SystemRandom::new();
    let mut nonce_bytes = [0u8; 12];

    match rand.fill(&mut nonce_bytes) {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to generate nonce: {:?}", err);
            return dto_err("failed_to_generate_nonce").as_error();
        }
    };

    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = plain.as_bytes().to_vec();

    match key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out) {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to encrypt string: {:?}", err);
            return dto_err("failed_to_encrypt_string").as_error();
        }
    };

    let mut encrypted = nonce_bytes.to_vec();
    encrypted.extend_from_slice(&in_out);

    Ok(general_purpose::STANDARD.encode(encrypted))
}

/// Decrypt a value previously produced by `encrypt_string`.
///
/// Expects `base64(nonce ‖ ciphertext ‖ tag)`. Returns the original
/// plain-text string on success.
#[tracing::instrument(name = "decrypt_string", skip_all)]
pub async fn decrypt_string(
    encrypted_b64: &str,
    config: &AccountLifeCycle,
) -> Result<String, MappedErrors> {
    let key = build_aes_key(config).await?;

    let encrypted = match general_purpose::STANDARD.decode(encrypted_b64) {
        Ok(v) => v,
        Err(err) => {
            error!("Failed to decode base64 encrypted data: {:?}", err);
            return dto_err("failed_to_decode_encrypted_data").as_error();
        }
    };

    if encrypted.len() < 12 {
        return dto_err("encrypted_data_too_short").as_error();
    }

    let (nonce_bytes, ciphertext) = encrypted.split_at(12);

    let nonce = match Nonce::try_assume_unique_for_key(nonce_bytes) {
        Ok(n) => n,
        Err(_) => return dto_err("invalid_nonce").as_error(),
    };

    let mut in_out = ciphertext.to_vec();

    match key.open_in_place(nonce, Aad::empty(), &mut in_out) {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to decrypt string: {:?}", err);
            return dto_err("failed_to_decrypt_string").as_error();
        }
    };

    if in_out.len() >= 16 {
        in_out.truncate(in_out.len() - 16);
    }

    match String::from_utf8(in_out) {
        Ok(s) => Ok(s),
        Err(err) => {
            dto_err(format!("decrypted_data_not_valid_utf8: {err}")).as_error()
        }
    }
}

async fn build_aes_key(
    config: &AccountLifeCycle,
) -> Result<LessSafeKey, MappedErrors> {
    let token_secret = config.token_secret.async_get_or_error().await?;

    let key_uuid = match Uuid::parse_str(&token_secret) {
        Ok(u) => u,
        Err(err) => {
            error!("Failed to parse token_secret as UUID: {:?}", err);
            return dto_err("failed_to_parse_encryption_key").as_error();
        }
    };

    let key_bytes = derive_key_from_uuid(&key_uuid);

    match UnboundKey::new(&AES_256_GCM, &key_bytes) {
        Ok(k) => Ok(LessSafeKey::new(k)),
        Err(err) => {
            error!("Failed to create AES key: {:?}", err);
            dto_err("failed_to_create_aes_key").as_error()
        }
    }
}
