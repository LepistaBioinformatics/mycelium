use lazy_static::lazy_static;
use std::env::var_os;

lazy_static! {

    /// A static value to be used on calculation of the tokens expiration time.
    #[derive(Debug)]
    pub(super) static ref TOKENS_VALIDATION_PATH: String =
        match var_os("TOKENS_VALIDATION_PATH") {
            None => panic!("`TOKENS_VALIDATION_PATH` not configured."),
            Some(path) => path.to_str().unwrap().to_string()
        };
}
