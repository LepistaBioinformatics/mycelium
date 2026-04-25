use super::types::{
    BotToken, InitData, TelegramUser, TelegramUserId, TelegramVerifyError,
};

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

const AUTH_DATE_MAX_AGE_SECS: u64 = 300;
const WEBAPP_DATA_KEY: &[u8] = b"WebAppData";

/// Verify Telegram Mini App initData and extract the authenticated user.
///
/// Algorithm: https://core.telegram.org/bots/webapps#validating-data-received-via-the-mini-app
///
/// Accepts `now` as a parameter so the function is purely testable without
/// mocking the system clock.
pub fn verify_init_data(
    raw: &InitData,
    bot_token: &BotToken,
    now: DateTime<Utc>,
) -> Result<TelegramUser, TelegramVerifyError> {
    let pairs = parse_pairs(&raw.0);
    let hash = extract_hash(&pairs)?;
    let data_check_string = build_data_check_string(&pairs);
    verify_hmac(&data_check_string, bot_token, &hash)?;
    let user = extract_user(&pairs)?;
    let auth_date = extract_auth_date(&pairs);
    check_auth_date(auth_date, now)?;
    Ok(user)
}

// ? ---------------------------------------------------------------------------
// ? Internal helpers
// ? ---------------------------------------------------------------------------

fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                result.push(byte as char);
                i += 3;
                continue;
            }
        }

        if bytes[i] == b'+' {
            result.push(' ');
        } else {
            result.push(bytes[i] as char);
        }

        i += 1;
    }

    result
}

fn parse_pairs(raw: &str) -> Vec<(String, String)> {
    raw.split('&')
        .filter_map(|part| {
            let mut iter = part.splitn(2, '=');
            let key = iter.next()?.to_owned();
            let value = url_decode(iter.next().unwrap_or(""));
            Some((key, value))
        })
        .collect()
}

fn extract_hash(
    pairs: &[(String, String)],
) -> Result<String, TelegramVerifyError> {
    pairs
        .iter()
        .find(|(k, _)| k == "hash")
        .map(|(_, v)| v.clone())
        .ok_or(TelegramVerifyError::MissingHash)
}

fn build_data_check_string(pairs: &[(String, String)]) -> String {
    let mut filtered: Vec<(&str, &str)> = pairs
        .iter()
        .filter(|(k, _)| k != "hash")
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    filtered.sort_by_key(|(k, _)| *k);

    filtered
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn verify_hmac(
    data_check_string: &str,
    bot_token: &BotToken,
    expected_hex: &str,
) -> Result<(), TelegramVerifyError> {
    let mut secret_mac = HmacSha256::new_from_slice(WEBAPP_DATA_KEY)
        .expect("HMAC accepts any key length");
    secret_mac.update(bot_token.0.expose_secret().as_bytes());
    let secret_key = secret_mac.finalize().into_bytes();

    let mut data_mac = HmacSha256::new_from_slice(&secret_key)
        .expect("HMAC accepts any key length");
    data_mac.update(data_check_string.as_bytes());
    let computed = data_mac.finalize().into_bytes();

    let computed_hex = hex::encode(computed);
    let expected_bytes = expected_hex.as_bytes();
    let computed_bytes = computed_hex.as_bytes();

    let len_ok = expected_bytes.len().ct_eq(&computed_bytes.len());
    let bytes_ok = expected_bytes.ct_eq(computed_bytes).unwrap_u8() == 1
        && bool::from(len_ok);

    match bytes_ok {
        true => Ok(()),
        false => Err(TelegramVerifyError::InvalidHmac),
    }
}

fn extract_user(
    pairs: &[(String, String)],
) -> Result<TelegramUser, TelegramVerifyError> {
    let user_json = pairs
        .iter()
        .find(|(k, _)| k == "user")
        .map(|(_, v)| v.as_str())
        .ok_or(TelegramVerifyError::MissingUserField)?;

    let value: serde_json::Value = serde_json::from_str(user_json)
        .map_err(|_| TelegramVerifyError::MalformedUserField)?;

    let id = value
        .get("id")
        .and_then(|v| v.as_i64())
        .ok_or(TelegramVerifyError::MalformedUserField)?;

    let username = value
        .get("username")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned());

    Ok(TelegramUser {
        id: TelegramUserId(id),
        username,
    })
}

fn extract_auth_date(pairs: &[(String, String)]) -> Option<u64> {
    pairs
        .iter()
        .find(|(k, _)| k == "auth_date")
        .and_then(|(_, v)| v.parse::<u64>().ok())
}

fn check_auth_date(
    auth_date: Option<u64>,
    now: DateTime<Utc>,
) -> Result<(), TelegramVerifyError> {
    let Some(ts) = auth_date else {
        return Ok(());
    };

    let now_ts = now.timestamp() as u64;
    let age = now_ts.saturating_sub(ts).max(ts.saturating_sub(now_ts));

    match age <= AUTH_DATE_MAX_AGE_SECS {
        true => Ok(()),
        false => Err(TelegramVerifyError::Expired),
    }
}

// ? ---------------------------------------------------------------------------
// ? Tests
// ? ---------------------------------------------------------------------------

#[cfg(test)]
fn percent_encode(s: &str) -> String {
    let mut out = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~' => out.push(byte as char),
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use secrecy::SecretString;

    const TEST_BOT_TOKEN: &str = "1234567890:AABBCCDDEEFF";

    fn make_bot_token() -> BotToken {
        BotToken(SecretString::new(TEST_BOT_TOKEN.into()))
    }

    fn make_init_data_with_hash(
        bot_token: &BotToken,
        user_json: &str,
        auth_date: u64,
    ) -> String {
        // data_check_string uses decoded values (raw JSON), sorted by key
        let mut decoded_pairs = vec![
            format!("auth_date={}", auth_date),
            format!("user={}", user_json),
        ];
        decoded_pairs.sort();
        let data_check_string = decoded_pairs.join("\n");

        let mut secret_mac =
            HmacSha256::new_from_slice(WEBAPP_DATA_KEY).unwrap();
        secret_mac.update(bot_token.0.expose_secret().as_bytes());
        let secret_key = secret_mac.finalize().into_bytes();

        let mut data_mac = HmacSha256::new_from_slice(&secret_key).unwrap();
        data_mac.update(data_check_string.as_bytes());
        let hash = hex::encode(data_mac.finalize().into_bytes());

        // raw initData has user value percent-encoded (as Telegram sends it)
        format!(
            "auth_date={}&user={}&hash={}",
            auth_date,
            percent_encode(user_json),
            hash
        )
    }

    #[test]
    fn valid_init_data_returns_user() {
        let bot_token = make_bot_token();
        let now = Utc.timestamp_opt(1_000_000, 0).unwrap();
        let auth_date = 1_000_000u64;
        let user_json = r#"{"id":123456,"username":"alice"}"#;

        let raw = make_init_data_with_hash(&bot_token, user_json, auth_date);
        let result = verify_init_data(&InitData(raw), &bot_token, now);

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.id, TelegramUserId(123456));
        assert_eq!(user.username, Some("alice".to_owned()));
    }

    #[test]
    fn expired_auth_date_returns_error() {
        let bot_token = make_bot_token();
        let auth_date = 1_000_000u64;
        let now = Utc
            .timestamp_opt((auth_date + AUTH_DATE_MAX_AGE_SECS + 1) as i64, 0)
            .unwrap();
        let user_json = r#"{"id":123456}"#;

        let raw = make_init_data_with_hash(&bot_token, user_json, auth_date);
        let result = verify_init_data(&InitData(raw), &bot_token, now);

        assert_eq!(result, Err(TelegramVerifyError::Expired));
    }

    #[test]
    fn wrong_hash_returns_invalid_hmac() {
        let bot_token = make_bot_token();
        let now = Utc.timestamp_opt(1_000_000, 0).unwrap();
        let raw = "auth_date=1000000&user=%7B%22id%22%3A1%7D&hash=deadbeef";

        let result =
            verify_init_data(&InitData(raw.to_owned()), &bot_token, now);

        assert_eq!(result, Err(TelegramVerifyError::InvalidHmac));
    }

    #[test]
    fn missing_hash_returns_error() {
        let bot_token = make_bot_token();
        let now = Utc.timestamp_opt(1_000_000, 0).unwrap();
        let raw = "auth_date=1000000&user=%7B%22id%22%3A1%7D";

        let result =
            verify_init_data(&InitData(raw.to_owned()), &bot_token, now);

        assert_eq!(result, Err(TelegramVerifyError::MissingHash));
    }

    #[test]
    fn missing_user_field_returns_error() {
        let bot_token = make_bot_token();
        let now = Utc.timestamp_opt(1_000_000, 0).unwrap();

        let pairs_no_user = vec!["auth_date=1000000".to_owned()];
        let data_check_string = pairs_no_user.join("\n");

        let mut secret_mac =
            HmacSha256::new_from_slice(WEBAPP_DATA_KEY).unwrap();
        secret_mac.update(bot_token.0.expose_secret().as_bytes());
        let secret_key = secret_mac.finalize().into_bytes();
        let mut data_mac = HmacSha256::new_from_slice(&secret_key).unwrap();
        data_mac.update(data_check_string.as_bytes());
        let hash = hex::encode(data_mac.finalize().into_bytes());

        let raw = format!("auth_date=1000000&hash={}", hash);
        let result = verify_init_data(&InitData(raw), &bot_token, now);

        assert_eq!(result, Err(TelegramVerifyError::MissingUserField));
    }

    #[test]
    fn user_without_username_returns_none() {
        let bot_token = make_bot_token();
        let now = Utc.timestamp_opt(1_000_000, 0).unwrap();
        let auth_date = 1_000_000u64;
        let user_json = r#"{"id":999}"#;

        let raw = make_init_data_with_hash(&bot_token, user_json, auth_date);
        let result = verify_init_data(&InitData(raw), &bot_token, now);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().username, None);
    }
}
