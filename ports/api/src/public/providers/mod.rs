mod azure;
mod google;

pub use azure::check_credentials::check_credentials as az_check_credentials;
pub use google::check_credentials::check_credentials as gc_check_credentials;
