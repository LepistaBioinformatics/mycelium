use keyring::Entry;

/// Reads a secret from the OS keyring. Any failure (including "no backend
/// available", the expected case on headless Linux hosts per OC-2) is
/// treated as "not found" rather than propagated -- the caller falls back
/// to the encrypted file.
pub(crate) fn read_secret_from_keyring(
    service: &str,
    name: &str,
) -> Option<String> {
    let entry = Entry::new(service, name).ok()?;
    entry.get_password().ok()
}

/// Best-effort keyring write. Returns `true` on success so the caller can
/// decide whether the encrypted-file fallback is still needed.
pub(crate) fn persist_secret_to_keyring(
    service: &str,
    name: &str,
    value: &str,
) -> bool {
    let Ok(entry) = Entry::new(service, name) else {
        return false;
    };

    entry.set_password(value).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On CI/sandboxed hosts there is typically no Secret Service / Keychain
    /// daemon (OC-2's expected primary case) -- the assertion here is that
    /// this does not panic, matching the "degrade gracefully" requirement.
    #[test]
    fn read_does_not_panic_when_no_backend_is_available() {
        let _ = read_secret_from_keyring(
            "mycelium-standalone-tests",
            "nonexistent-secret",
        );
    }
}
