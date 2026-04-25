use base64::{engine::general_purpose, Engine};
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use ring::{
    aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM},
    rand::{SecureRandom, SystemRandom},
};
use tracing::error;
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? System constants
// ? ---------------------------------------------------------------------------

/// Sentinel tenant ID used for system-level (Staff, webhooks) DEK lookups.
pub const SYSTEM_TENANT_ID: Uuid = Uuid::nil();

/// Build the AAD (Additional Authenticated Data) bytes for a ciphertext.
///
/// AAD = tenant_id_bytes ++ field_tag prevents a ciphertext from being moved
/// across fields or tenants without detection. Pass `None` to use the system
/// tenant (Uuid::nil).
pub fn build_aad(tenant_id: Option<Uuid>, field: &[u8]) -> Vec<u8> {
    let tid = tenant_id.unwrap_or(SYSTEM_TENANT_ID);
    let mut aad = tid.as_bytes().to_vec();
    aad.extend_from_slice(field);
    aad
}

// ? ---------------------------------------------------------------------------
// ? AAD field-name constants
//
// Callers must construct AAD as:
//   tenant_id.as_bytes() ++ AAD_FIELD_*
// to prevent ciphertext from being moved across fields or tenants.
// ? ---------------------------------------------------------------------------

pub const AAD_FIELD_TOTP_SECRET: &[u8] = b"totp_secret";
pub const AAD_FIELD_TELEGRAM_BOT_TOKEN: &[u8] = b"telegram_bot_token";
pub const AAD_FIELD_TELEGRAM_WEBHOOK_SECRET: &[u8] = b"telegram_webhook_secret";
pub const AAD_FIELD_HTTP_SECRET: &[u8] = b"http_secret";

// ? ---------------------------------------------------------------------------
// ? Core functions
// ? ---------------------------------------------------------------------------

/// Generate a fresh 256-bit data-encryption key via CSPRNG.
pub fn generate_dek() -> Result<[u8; 32], MappedErrors> {
    let rand = SystemRandom::new();
    let mut dek = [0u8; 32];
    rand.fill(&mut dek).map_err(|err| {
        error!("Failed to generate DEK: {:?}", err);
        dto_err("failed_to_generate_dek")
    })?;
    Ok(dek)
}

/// Wrap (encrypt) a DEK with a KEK using AES-256-GCM.
///
/// Returns `v2:base64(nonce_12 || ciphertext || tag_16)`.
/// The `aad` should be `tenant_id.as_bytes()` to bind the wrapped key to its
/// tenant and detect any cross-tenant copy.
#[tracing::instrument(name = "wrap_dek", skip_all)]
pub fn wrap_dek(
    dek: &[u8; 32],
    kek: &[u8; 32],
    aad: &[u8],
) -> Result<String, MappedErrors> {
    let key = build_key(kek)?;
    let nonce_bytes = fresh_nonce()?;
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = dek.to_vec();
    key.seal_in_place_append_tag(nonce, Aad::from(aad), &mut in_out)
        .map_err(|err| {
            error!("Failed to wrap DEK: {:?}", err);
            dto_err("failed_to_wrap_dek")
        })?;
    let mut blob = nonce_bytes.to_vec();
    blob.extend_from_slice(&in_out);
    Ok(format!("v2:{}", general_purpose::STANDARD.encode(blob)))
}

/// Unwrap (decrypt) a wrapped DEK produced by `wrap_dek`.
///
/// Expects the `v2:` prefix; returns an error if the prefix is absent or the
/// ciphertext is tampered (including wrong `aad`).
#[tracing::instrument(name = "unwrap_dek", skip_all)]
pub fn unwrap_dek(
    encrypted_dek: &str,
    kek: &[u8; 32],
    aad: &[u8],
) -> Result<[u8; 32], MappedErrors> {
    let b64 = encrypted_dek
        .strip_prefix("v2:")
        .ok_or_else(|| dto_err("encrypted_dek_missing_v2_prefix"))?;
    let blob = general_purpose::STANDARD.decode(b64).map_err(|err| {
        error!("Failed to decode wrapped DEK: {:?}", err);
        dto_err("failed_to_decode_wrapped_dek")
    })?;
    let (nonce_bytes, ciphertext) = split_nonce(&blob)?;
    let nonce = Nonce::try_assume_unique_for_key(nonce_bytes)
        .map_err(|_| dto_err("invalid_nonce_in_wrapped_dek"))?;
    let key = build_key(kek)?;
    let mut in_out = ciphertext.to_vec();
    key.open_in_place(nonce, Aad::from(aad), &mut in_out)
        .map_err(|err| {
            error!("Failed to unwrap DEK: {:?}", err);
            dto_err("failed_to_unwrap_dek")
        })?;
    strip_tag(&mut in_out);
    in_out
        .try_into()
        .map_err(|_| dto_err("unwrapped_dek_wrong_length"))
}

/// Encrypt a plaintext string with a DEK using AES-256-GCM.
///
/// Returns `v2:base64(nonce_12 || ciphertext || tag_16)`.
#[tracing::instrument(name = "encrypt_with_dek", skip_all)]
pub fn encrypt_with_dek(
    plain: &str,
    dek: &[u8; 32],
    aad: &[u8],
) -> Result<String, MappedErrors> {
    let key = build_key(dek)?;
    let nonce_bytes = fresh_nonce()?;
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = plain.as_bytes().to_vec();
    key.seal_in_place_append_tag(nonce, Aad::from(aad), &mut in_out)
        .map_err(|err| {
            error!("Failed to encrypt with DEK: {:?}", err);
            dto_err("failed_to_encrypt_with_dek")
        })?;
    let mut blob = nonce_bytes.to_vec();
    blob.extend_from_slice(&in_out);
    Ok(format!("v2:{}", general_purpose::STANDARD.encode(blob)))
}

/// Decrypt a ciphertext produced by `encrypt_with_dek`.
///
/// Expects the `v2:` prefix; returns an error if absent or on auth failure.
#[tracing::instrument(name = "decrypt_with_dek", skip_all)]
pub fn decrypt_with_dek(
    cipher: &str,
    dek: &[u8; 32],
    aad: &[u8],
) -> Result<String, MappedErrors> {
    let b64 = cipher
        .strip_prefix("v2:")
        .ok_or_else(|| dto_err("ciphertext_missing_v2_prefix"))?;
    let blob = general_purpose::STANDARD.decode(b64).map_err(|err| {
        error!("Failed to decode v2 ciphertext: {:?}", err);
        dto_err("failed_to_decode_v2_ciphertext")
    })?;
    let (nonce_bytes, ciphertext) = split_nonce(&blob)?;
    let nonce = Nonce::try_assume_unique_for_key(nonce_bytes)
        .map_err(|_| dto_err("invalid_nonce_in_v2_ciphertext"))?;
    let key = build_key(dek)?;
    let mut in_out = ciphertext.to_vec();
    key.open_in_place(nonce, Aad::from(aad), &mut in_out)
        .map_err(|err| {
            error!("Failed to decrypt with DEK: {:?}", err);
            dto_err("failed_to_decrypt_with_dek")
        })?;
    strip_tag(&mut in_out);
    String::from_utf8(in_out)
        .map_err(|err| dto_err(format!("decrypted_data_not_valid_utf8: {err}")))
}

// ? ---------------------------------------------------------------------------
// ? Private helpers
// ? ---------------------------------------------------------------------------

fn build_key(key_bytes: &[u8; 32]) -> Result<LessSafeKey, MappedErrors> {
    let unbound = UnboundKey::new(&AES_256_GCM, key_bytes).map_err(|err| {
        error!("Failed to build AES key: {:?}", err);
        dto_err("failed_to_build_aes_key")
    })?;
    Ok(LessSafeKey::new(unbound))
}

fn fresh_nonce() -> Result<[u8; 12], MappedErrors> {
    let rand = SystemRandom::new();
    let mut nonce = [0u8; 12];
    rand.fill(&mut nonce).map_err(|err| {
        error!("Failed to generate nonce: {:?}", err);
        dto_err("failed_to_generate_nonce")
    })?;
    Ok(nonce)
}

fn split_nonce(blob: &[u8]) -> Result<(&[u8], &[u8]), MappedErrors> {
    if blob.len() < 12 {
        return dto_err("blob_too_short_for_nonce").as_error();
    }
    Ok(blob.split_at(12))
}

/// Remove the 16-byte GCM tag appended by `seal_in_place_append_tag`.
fn strip_tag(buf: &mut Vec<u8>) {
    let len = buf.len();
    if len >= 16 {
        buf.truncate(len - 16);
    }
}

// ? ---------------------------------------------------------------------------
// ? Tests
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_kek() -> [u8; 32] {
        [0x42u8; 32]
    }

    fn make_aad() -> Vec<u8> {
        b"tenant-aad-bytes".to_vec()
    }

    #[test]
    fn dek_round_trip() {
        let dek = generate_dek().unwrap();
        let kek = make_kek();
        let aad = make_aad();

        let wrapped = wrap_dek(&dek, &kek, &aad).unwrap();
        assert!(wrapped.starts_with("v2:"));

        let recovered = unwrap_dek(&wrapped, &kek, &aad).unwrap();
        assert_eq!(dek, recovered);
    }

    #[test]
    fn dek_tampered_aad_fails() {
        let dek = generate_dek().unwrap();
        let kek = make_kek();

        let wrapped = wrap_dek(&dek, &kek, b"original-aad").unwrap();
        let result = unwrap_dek(&wrapped, &kek, b"different-aad");
        assert!(result.is_err());
    }

    #[test]
    fn payload_round_trip() {
        let dek = generate_dek().unwrap();
        let aad = make_aad();
        let plain = "my-secret-totp-value";

        let cipher = encrypt_with_dek(plain, &dek, &aad).unwrap();
        assert!(cipher.starts_with("v2:"));

        let recovered = decrypt_with_dek(&cipher, &dek, &aad).unwrap();
        assert_eq!(plain, recovered);
    }

    #[test]
    fn payload_tampered_aad_fails() {
        let dek = generate_dek().unwrap();
        let cipher = encrypt_with_dek("secret", &dek, b"aad-a").unwrap();
        let result = decrypt_with_dek(&cipher, &dek, b"aad-b");
        assert!(result.is_err());
    }

    #[test]
    fn missing_v2_prefix_fails_unwrap() {
        let kek = make_kek();
        let result = unwrap_dek("no-prefix-here", &kek, b"aad");
        assert!(result.is_err());
    }

    #[test]
    fn missing_v2_prefix_fails_decrypt() {
        let dek = generate_dek().unwrap();
        let result = decrypt_with_dek("no-prefix", &dek, b"aad");
        assert!(result.is_err());
    }
}
