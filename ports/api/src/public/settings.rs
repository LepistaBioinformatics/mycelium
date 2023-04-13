use std::env::var_os;

use lazy_static::lazy_static;
use reqwest::Client;

// ? ---------------------------------------------------------------------------
// ? Authentication and authorization
// ? ---------------------------------------------------------------------------

lazy_static! {
    #[derive(Debug)]
    pub(super) static ref REQWEST_CLIENT: Client = Client::new();
}

pub(super) async fn get_client() -> Client {
    REQWEST_CLIENT.to_owned()
}

lazy_static! {

    #[derive(Debug)]
    pub(crate) static ref PROFILE_FETCHING_URL: String =
        match var_os("PROFILE_FETCHING_URL") {
            Some(path) => path.into_string().unwrap(),
            None => panic!("PROFILE_FETCHING_URL not configured."),
        };
}
