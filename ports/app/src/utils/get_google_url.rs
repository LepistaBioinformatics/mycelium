use std::collections::HashMap;
use url::Url;

use load_dotenv::load_dotenv;

load_dotenv!();

pub fn get_google_url(from: Option<&str>) -> String {
    let client_id = std::env!("GOOGLE_OAUTH_CLIENT_ID");
    let redirect_uri = std::env!("GOOGLE_OAUTH_REDIRECT_URL");

    let root_url = "https://accounts.google.com/o/oauth2/v2/auth";
    let mut options = HashMap::new();
    options.insert("redirect_uri", redirect_uri);
    options.insert("client_id", client_id);
    options.insert("access_type", "offline");
    options.insert("response_type", "code");
    options.insert("prompt", "consent");
    options.insert(
        "scope",
        "https://www.googleapis.com/auth/userinfo.profile https://www.googleapis.com/auth/userinfo.email",
    );
    options.insert("state", from.unwrap_or_default());

    let url = Url::parse_with_params(root_url, &options).unwrap();
    let qs = url.query().unwrap();

    format!("{}?{}", root_url, qs)
}
