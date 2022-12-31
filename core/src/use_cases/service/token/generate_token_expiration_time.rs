use crate::settings::TOKENS_EXPIRATION_TIME;

use chrono::{DateTime, Duration, Local};

/// Calculate expiration time.
///
/// The expiration time should be used to filter expired tokens.
pub async fn generate_token_expiration_time() -> DateTime<Local> {
    Local::now() + Duration::seconds(*TOKENS_EXPIRATION_TIME)
}
