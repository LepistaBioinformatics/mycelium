use lazy_static::lazy_static;
use reqwest::Client;

lazy_static! {
    #[derive(Debug)]
    pub(super) static ref REQWEST_CLIENT: Client = Client::new();
}

pub async fn get_client() -> Client {
    REQWEST_CLIENT.to_owned()
}
