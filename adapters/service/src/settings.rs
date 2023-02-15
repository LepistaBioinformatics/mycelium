use lazy_static::lazy_static;
use std::env::var_os;

lazy_static! {

    /// Try to extract the URL used to validate profile from token.
    #[derive(Debug)]
    pub(crate) static ref PROFILE_FETCHING_URL: String =
        match var_os("PROFILE_FETCHING_URL") {
            Some(path) => path.into_string().unwrap(),
            None => panic!("`PROFILE_FETCHING_URL` not configured."),
        };
}
