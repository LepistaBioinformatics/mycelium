use ring::digest::{digest, SHA256};
use std::path::Path;

/// Derives a 32-byte wrapping key from stable local-machine entropy, used to
/// encrypt the standalone secrets file at rest.
///
/// This is defense-in-depth against casual copying of the secrets file, not
/// a substitute for the OS keyring: the derivation is deterministic from
/// machine identity, so anyone with access to the same host (or its
/// `/etc/machine-id`) can reproduce the key. The 0600 file permission is the
/// primary protection.
pub(crate) fn derive_local_wrapping_key() -> [u8; 32] {
    let machine_identity = read_machine_identity();
    let hash = digest(&SHA256, machine_identity.as_bytes());

    let mut key = [0u8; 32];
    key.copy_from_slice(hash.as_ref());
    key
}

fn read_machine_identity() -> String {
    for path in ["/etc/machine-id", "/var/lib/dbus/machine-id"] {
        if let Ok(id) = std::fs::read_to_string(Path::new(path)) {
            let trimmed = id.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    hostname_fallback()
}

fn hostname_fallback() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "mycelium-standalone-fallback-identity".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derivation_is_deterministic() {
        assert_eq!(derive_local_wrapping_key(), derive_local_wrapping_key());
    }
}
