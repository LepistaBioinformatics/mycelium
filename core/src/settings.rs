use lazy_static::lazy_static;
use std::env::var_os;

/// This key is used to set and get the user profile to and from requests.
pub const DEFAULT_PROFILE_KEY: &str = "x-mycelium-profile";

lazy_static! {

    /// A static value to be used on calculation of the tokens expiration time.
    #[derive(Debug)]
    pub static ref TOKENS_EXPIRATION_TIME: i64 =
        match var_os("TOKENS_EXPIRATION_TIME") {
            Some(path) => {
                path.into_string().unwrap().parse::<i64>().unwrap().into()
            }
            None => 10,
        };
}
