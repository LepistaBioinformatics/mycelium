use lazy_static::lazy_static;
use std::env::var_os;

// Keys used over the profile fetch and validation pipeline.
pub const DEFAULT_PROFILE_KEY: &str = "x-mycelium-profile";
pub const DEFAULT_TOKEN_KEY: &str = "x-mycelium-token";

lazy_static! {

    /// A static value to be used on calculation of the tokens expiration time.
    #[derive(Debug)]
    pub(crate) static ref TOKENS_EXPIRATION_TIME: i64 =
        match var_os("TOKENS_EXPIRATION_TIME") {
            Some(path) => {
                path.into_string().unwrap().parse::<i64>().unwrap().into()
            }
            None => 10,
        };
}
