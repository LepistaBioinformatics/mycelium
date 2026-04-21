use crate::{
    domain::utils::{derive_key_from_uuid, envelope::decrypt_with_dek},
    models::AccountLifeCycle,
};

use base64::{engine::general_purpose, Engine};
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use ring::{
    aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM},
    rand::{SecureRandom, SystemRandom},
};
use tracing::error;
use uuid::Uuid;

/// Encrypt a plain-text string with AES-256-GCM using the server's token
/// secret as key material (v1 legacy format, no DEK).
///
/// Returns `base64(nonce ‖ ciphertext ‖ tag)` suitable for opaque storage.
/// New code should prefer `envelope::encrypt_with_dek` instead.
#[tracing::instrument(name = "encrypt_string", skip_all)]
pub async fn encrypt_string(
    plain: &str,
    config: &AccountLifeCycle,
) -> Result<String, MappedErrors> {
    let key = build_aes_key(config).await?;

    let rand = SystemRandom::new();
    let mut nonce_bytes = [0u8; 12];

    rand.fill(&mut nonce_bytes).map_err(|err| {
        error!("Failed to generate nonce: {:?}", err);
        dto_err("failed_to_generate_nonce")
    })?;

    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = plain.as_bytes().to_vec();

    key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .map_err(|err| {
            error!("Failed to encrypt string: {:?}", err);
            dto_err("failed_to_encrypt_string")
        })?;

    let mut encrypted = nonce_bytes.to_vec();
    encrypted.extend_from_slice(&in_out);

    Ok(general_purpose::STANDARD.encode(encrypted))
}

/// Decrypt a value produced by `encrypt_string` (v1) or `encrypt_with_dek`
/// (v2).
///
/// If the ciphertext starts with `v2:` and a `dek` is supplied, the v2 path
/// is taken. Otherwise the legacy v1 path is used with the KEK from `config`.
/// Passing `dek = None` on a v2 ciphertext returns an error.
#[tracing::instrument(name = "decrypt_string", skip_all)]
pub async fn decrypt_string(
    encrypted_b64: &str,
    config: &AccountLifeCycle,
) -> Result<String, MappedErrors> {
    decrypt_string_with_optional_dek(encrypted_b64, config, None, &[]).await
}

/// Decrypt with an explicit DEK for the v2 path and AAD.
///
/// Falls back to v1 legacy decryption when the ciphertext has no `v2:` prefix.
#[tracing::instrument(name = "decrypt_string_with_dek", skip_all)]
pub async fn decrypt_string_with_dek(
    encrypted_b64: &str,
    config: &AccountLifeCycle,
    dek: &[u8; 32],
    aad: &[u8],
) -> Result<String, MappedErrors> {
    decrypt_string_with_optional_dek(encrypted_b64, config, Some(dek), aad)
        .await
}

async fn decrypt_string_with_optional_dek(
    encrypted_b64: &str,
    config: &AccountLifeCycle,
    dek: Option<&[u8; 32]>,
    aad: &[u8],
) -> Result<String, MappedErrors> {
    if encrypted_b64.starts_with("v2:") {
        let dek = dek.ok_or_else(|| dto_err("v2_ciphertext_requires_dek"))?;
        return decrypt_with_dek(encrypted_b64, dek, aad);
    }
    decrypt_string_v1(encrypted_b64, config).await
}

async fn decrypt_string_v1(
    encrypted_b64: &str,
    config: &AccountLifeCycle,
) -> Result<String, MappedErrors> {
    let key = build_aes_key(config).await?;

    let encrypted =
        general_purpose::STANDARD
            .decode(encrypted_b64)
            .map_err(|err| {
                error!("Failed to decode base64 encrypted data: {:?}", err);
                dto_err("failed_to_decode_encrypted_data")
            })?;

    if encrypted.len() < 12 {
        return dto_err("encrypted_data_too_short").as_error();
    }

    let (nonce_bytes, ciphertext) = encrypted.split_at(12);

    let nonce = Nonce::try_assume_unique_for_key(nonce_bytes)
        .map_err(|_| dto_err("invalid_nonce"))?;

    let mut in_out = ciphertext.to_vec();

    key.open_in_place(nonce, Aad::empty(), &mut in_out)
        .map_err(|err| {
            error!("Failed to decrypt string: {:?}", err);
            dto_err("failed_to_decrypt_string")
        })?;

    if in_out.len() >= 16 {
        in_out.truncate(in_out.len() - 16);
    }

    String::from_utf8(in_out)
        .map_err(|err| dto_err(format!("decrypted_data_not_valid_utf8: {err}")))
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

    UnboundKey::new(&AES_256_GCM, &key_bytes)
        .map(LessSafeKey::new)
        .map_err(|err| {
            error!("Failed to create AES key: {:?}", err);
            dto_err("failed_to_create_aes_key")
        })
}

// ? ---------------------------------------------------------------------------
// ? Tests
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::utils::envelope::{encrypt_with_dek, generate_dek};
    use myc_config::secret_resolver::SecretResolver;

    fn make_config() -> AccountLifeCycle {
        AccountLifeCycle {
            domain_name: SecretResolver::Value("example.com".to_string()),
            domain_url: None,
            locale: None,
            token_expiration: SecretResolver::Value(3600),
            noreply_name: None,
            noreply_email: SecretResolver::Value(
                "noreply@example.com".to_string(),
            ),
            support_name: None,
            support_email: SecretResolver::Value(
                "support@example.com".to_string(),
            ),
            token_secret: SecretResolver::Value(
                "550e8400-e29b-41d4-a716-446655440000".to_string(),
            ),
        }
    }

    #[tokio::test]
    async fn v1_round_trip() -> Result<(), MappedErrors> {
        let config = make_config();
        let plain = "hello-v1-world";
        let cipher = encrypt_string(plain, &config).await?;
        assert!(!cipher.starts_with("v2:"));
        let recovered = decrypt_string(&cipher, &config).await?;
        assert_eq!(plain, recovered);
        Ok(())
    }

    #[tokio::test]
    async fn v2_round_trip_via_decrypt_string_with_dek(
    ) -> Result<(), MappedErrors> {
        let config = make_config();
        let dek = generate_dek().unwrap();
        let aad = b"some-aad" as &[u8];
        let plain = "hello-v2-world";
        let cipher = encrypt_with_dek(plain, &dek, aad)?;
        assert!(cipher.starts_with("v2:"));
        let recovered =
            decrypt_string_with_dek(&cipher, &config, &dek, aad).await?;
        assert_eq!(plain, recovered);
        Ok(())
    }

    #[tokio::test]
    async fn v2_prefix_without_dek_fails() {
        let config = make_config();
        let dek = generate_dek().unwrap();
        let cipher = encrypt_with_dek("secret", &dek, b"aad").unwrap();
        let result = decrypt_string(&cipher, &config).await;
        assert!(result.is_err());
    }
}
