mod shared;

mod azure;
pub use azure::{
    check_credentials::check_credentials as az_check_credentials,
    config as azure_config,
};

mod google;
pub use google::{
    check_credentials::check_credentials as gc_check_credentials,
    config as google_config, handlers as google_handlers,
    model as google_models,
};
