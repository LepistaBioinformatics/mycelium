mod shared;

mod azure;
pub use azure::{
    config as azure_config,
    functions::check_credentials as az_check_credentials,
};

mod google;
pub use google::{
    config as google_config, endpoints as google_endpoints,
    functions::check_credentials as gc_check_credentials,
    models as google_models,
};
