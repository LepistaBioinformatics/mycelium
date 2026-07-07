mod derive_local_wrapping_key;
mod encrypted_file_store;
mod keyring_store;

use encrypted_file_store::{
    persist_secret_to_encrypted_file, read_secret_from_encrypted_file,
};
use keyring_store::{persist_secret_to_keyring, read_secret_from_keyring};

use mycelium_base::utils::errors::MappedErrors;
use std::path::Path;

/// Resolves a named standalone secret (SM-R9, DEC-2), called only when no
/// explicit secret was configured (env/config `Value` -- `SecretResolver`
/// already covers that path upstream).
///
/// Resolution order:
/// 1. OS keyring, if a backend is available.
/// 2. Encrypted local file (0600) under `secrets_dir`.
/// 3. Generate a new secret, then persist it -- keyring first, falling back
///    to the file so the value survives a restart (regenerating would rotate
///    the KEK/HMAC key and invalidate every existing connection string). The
///    keyring write is verified with an immediate read-back: some secret
///    service backends report success without durably persisting (observed
///    against an ephemeral/session-only backend), which would otherwise
///    silently regenerate the secret on every boot.
#[tracing::instrument(name = "resolve_or_generate_standalone_secret", skip_all)]
pub fn resolve_or_generate_standalone_secret(
    keyring_service: &str,
    secrets_dir: &Path,
    name: &str,
) -> Result<String, MappedErrors> {
    if let Some(value) = read_secret_from_keyring(keyring_service, name) {
        return Ok(value);
    }

    if let Some(value) = read_secret_from_encrypted_file(secrets_dir, name)? {
        return Ok(value);
    }

    let generated = uuid::Uuid::new_v4().to_string();
    let keyring_persisted =
        persist_secret_to_keyring(keyring_service, name, &generated)
            && read_secret_from_keyring(keyring_service, name).as_deref()
                == Some(generated.as_str());

    if !keyring_persisted {
        persist_secret_to_encrypted_file(secrets_dir, name, &generated)?;
    }

    Ok(generated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_secrets_dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "myc_standalone_secrets_e2e_{}",
            uuid::Uuid::new_v4()
        ))
    }

    #[test]
    fn first_boot_generates_and_second_boot_reuses_the_same_secret() {
        let dir = temp_secrets_dir();
        let service = "mycelium-standalone-tests";

        let first = resolve_or_generate_standalone_secret(
            service,
            &dir,
            "token_secret",
        )
        .unwrap();

        let second = resolve_or_generate_standalone_secret(
            service,
            &dir,
            "token_secret",
        )
        .unwrap();

        assert_eq!(first, second);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn generated_secret_is_a_valid_uuid() {
        let dir = temp_secrets_dir();

        let secret = resolve_or_generate_standalone_secret(
            "mycelium-standalone-tests",
            &dir,
            "token_secret",
        )
        .unwrap();

        assert!(uuid::Uuid::parse_str(&secret).is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
