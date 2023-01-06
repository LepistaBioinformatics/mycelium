pub mod extractor;
pub mod settings;

/// This is a re-exportation from the myc core to allow users to import both
/// from myc-api instead of the myc-core.
pub use myc_core::{
    domain::dtos::{
        email::Email,
        profile::{LicensedResources, Profile},
    },
    settings::DEFAULT_PROFILE_KEY,
    use_cases::service::profile::{ProfilePack, ProfileResponse},
};
