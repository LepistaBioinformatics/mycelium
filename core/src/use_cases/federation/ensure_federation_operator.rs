use crate::domain::dtos::{
    native_error_codes::NativeErrorCodes, profile::Profile,
};

use mycelium_base::utils::errors::{use_case_err, MappedErrors};

/// Only staff operate federation: list and quarantine peers, switch it off
/// (FED-33, FED-34, FED-35)
pub fn ensure_federation_operator(
    profile: &Profile,
) -> Result<(), MappedErrors> {
    if profile.is_staff {
        return Ok(());
    }

    use_case_err("Only staff users operate federation")
        .with_exp_true()
        .with_code(NativeErrorCodes::MYC00019)
        .as_error()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staff_operates_federation() {
        let mut profile = Profile::default();
        profile.is_staff = true;

        assert!(ensure_federation_operator(&profile).is_ok());
    }

    #[test]
    fn everyone_else_is_forbidden() {
        let mut profile = Profile::default();
        profile.is_manager = true;

        let error = ensure_federation_operator(&profile).unwrap_err();

        assert!(error.is_in(vec![NativeErrorCodes::MYC00019]));
    }
}
