pub mod dtos;
pub mod functions;
pub mod middleware;
pub mod models;
pub mod providers;
pub mod responses;
pub mod settings;
pub mod utils;
pub mod wrappers;

/// This is a re-exportation from the myc core to allow users to import both
/// from myc-api instead of the myc-core.
pub use myc_core::{
    domain::{
        actors::ActorName,
        dtos::{
            account::Account,
            email::Email,
            guest_role::Permission,
            profile::{LicensedResources, Profile},
            related_accounts::RelatedAccounts,
        },
    },
    settings::DEFAULT_PROFILE_KEY,
};
