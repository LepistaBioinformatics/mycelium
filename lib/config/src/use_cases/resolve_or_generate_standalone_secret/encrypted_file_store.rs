use super::derive_local_wrapping_key::derive_local_wrapping_key;

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use ring::{
    aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM},
    rand::{SecureRandom, SystemRandom},
};
use std::path::{Path, PathBuf};

fn secret_file_path(secrets_dir: &Path, name: &str) -> PathBuf {
    secrets_dir.join(format!("{name}.secret"))
}

/// Reads and decrypts a previously-persisted secret, if the file exists.
pub(crate) fn read_secret_from_encrypted_file(
    secrets_dir: &Path,
    name: &str,
) -> Result<Option<String>, MappedErrors> {
    let path = secret_file_path(secrets_dir, name);

    let raw = match std::fs::read(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None)
        }
        Err(err) => {
            return execution_err(format!(
                "Failed to read standalone secret file {path:?}: {err}"
            ))
            .as_error()
        }
    };

    if raw.len() < 12 {
        return execution_err(format!(
            "Standalone secret file {path:?} is corrupted (too short)"
        ))
        .as_error();
    }

    let (nonce_bytes, ciphertext) = raw.split_at(12);
    let key = build_key()?;
    let nonce =
        Nonce::try_assume_unique_for_key(nonce_bytes).map_err(|_| {
            execution_err("Invalid nonce in standalone secret file")
        })?;

    let mut in_out = ciphertext.to_vec();
    let plain = key
        .open_in_place(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| {
            execution_err(format!(
                "Failed to decrypt standalone secret file {path:?}"
            ))
        })?;

    String::from_utf8(plain.to_vec()).map(Some).map_err(|err| {
        execution_err(format!(
            "Standalone secret file {path:?} did not decode as UTF-8: {err}"
        ))
    })
}

/// Encrypts and persists a secret to the standalone secrets directory,
/// creating the directory and restricting the file to owner-only access
/// (0600) on Unix targets.
pub(crate) fn persist_secret_to_encrypted_file(
    secrets_dir: &Path,
    name: &str,
    value: &str,
) -> Result<(), MappedErrors> {
    std::fs::create_dir_all(secrets_dir).map_err(|err| {
        execution_err(format!(
            "Failed to create standalone secrets directory {secrets_dir:?}: {err}"
        ))
    })?;

    let key = build_key()?;
    let rand = SystemRandom::new();
    let mut nonce_bytes = [0u8; 12];

    rand.fill(&mut nonce_bytes).map_err(|_| {
        execution_err("Failed to generate nonce for standalone secret")
    })?;

    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = value.as_bytes().to_vec();

    key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| execution_err("Failed to encrypt standalone secret"))?;

    let mut persisted = nonce_bytes.to_vec();
    persisted.extend_from_slice(&in_out);

    let path = secret_file_path(secrets_dir, name);

    std::fs::write(&path, &persisted).map_err(|err| {
        execution_err(format!(
            "Failed to write standalone secret file {path:?}: {err}"
        ))
    })?;

    restrict_permissions(&path)
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> Result<(), MappedErrors> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|err| {
            execution_err(format!(
                "Failed to restrict permissions on {path:?}: {err}"
            ))
        })
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> Result<(), MappedErrors> {
    Ok(())
}

fn build_key() -> Result<LessSafeKey, MappedErrors> {
    let key_bytes = derive_local_wrapping_key();

    UnboundKey::new(&AES_256_GCM, &key_bytes)
        .map(LessSafeKey::new)
        .map_err(|_| {
            execution_err(
                "Failed to build local wrapping key for standalone secrets",
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_returns_none() {
        let dir = std::env::temp_dir().join(format!(
            "myc_standalone_secrets_missing_{}",
            uuid::Uuid::new_v4()
        ));

        let result =
            read_secret_from_encrypted_file(&dir, "token_secret").unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn persisted_secret_round_trips() {
        let dir = std::env::temp_dir()
            .join(format!("myc_standalone_secrets_{}", uuid::Uuid::new_v4()));

        persist_secret_to_encrypted_file(
            &dir,
            "token_secret",
            "my-secret-value",
        )
        .unwrap();

        let recovered =
            read_secret_from_encrypted_file(&dir, "token_secret").unwrap();

        assert_eq!(recovered, Some("my-secret-value".to_string()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[cfg(unix)]
    fn persisted_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!(
            "myc_standalone_secrets_perms_{}",
            uuid::Uuid::new_v4()
        ));

        persist_secret_to_encrypted_file(&dir, "token_secret", "value")
            .unwrap();

        let meta =
            std::fs::metadata(secret_file_path(&dir, "token_secret")).unwrap();

        assert_eq!(meta.permissions().mode() & 0o777, 0o600);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
