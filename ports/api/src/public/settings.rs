use lazy_static::lazy_static;
use std::env::var_os;

lazy_static! {

    /// The URL path of the token validation
    #[derive(Debug)]
    pub(super) static ref TOKENS_VALIDATION_PATH: String =
        match var_os("TOKENS_VALIDATION_PATH") {
            None => panic!("`TOKENS_VALIDATION_PATH` not configured."),
            Some(path) => path.to_str().unwrap().to_string()
        };
}

lazy_static! {

    /// Configure the service name
    ///
    /// The service name is the same configured at the application gateway, like
    /// service ingress or any custom application gateway.
    #[derive(Debug)]
    pub(super) static ref STANDARD_SERVICE_NAME: String =
        match var_os("SERVICE_NAME") {
            None => String::from("mycelium"),
            Some(path) => path.to_str().unwrap().to_string()
        };
}
