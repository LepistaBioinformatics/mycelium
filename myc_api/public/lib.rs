pub mod extractor;

/// This is a re-exportation from the myc core to allow users to import both
/// from myc-api instead of the myc-core.
pub use myc_core::{
    domain::dtos::{
        email::EmailDTO,
        profile::{LicensedResourcesDTO, ProfileDTO},
    },
    settings::DEFAULT_PROFILE_KEY,
};
