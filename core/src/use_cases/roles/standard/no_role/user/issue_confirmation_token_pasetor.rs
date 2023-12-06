use crate::{
    domain::{
        dtos::session_token::{SessionToken, TokenSecret},
        entities::SessionTokenRegistration,
    },
    settings::build_session_key,
};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use chrono::Local;
use clean_base::utils::errors::{factories::use_case_err, MappedErrors};
use hex;
use pasetors::{claims::Claims, keys::SymmetricKey, local, version4::V4};
use uuid::Uuid;

pub(super) async fn issue_confirmation_token_pasetor(
    user_id: Uuid,
    token_secret: TokenSecret,
    is_for_password_change: Option<bool>,
    token_registration_repo: Box<&dyn SessionTokenRegistration>,
) -> Result<String, MappedErrors> {
    // I just generate 128 bytes of random data for the session key
    // from something that is cryptographically secure (rand::CryptoRng)
    //
    // You don't necessarily need a random value, but you'll want something
    // that is sufficiently not able to be guessed (you don't want someone getting
    // an old token that is supposed to not be live, and being able to get a valid
    // token from that).
    let session_key: String = {
        let mut buff = [0_u8; 128];
        OsRng.fill_bytes(&mut buff);
        hex::encode(buff)
    };

    let data_storage_key = SessionToken::build_prefixed_session_token(
        session_key.to_owned(),
        is_for_password_change,
    );

    // ? -----------------------------------------------------------------------
    // ? Register session key on data storage
    // ? -----------------------------------------------------------------------

    token_registration_repo
        .create(
            build_session_key(data_storage_key.to_owned()),
            if is_for_password_change.is_some() {
                Local::now() + chrono::Duration::hours(1)
            } else {
                Local::now() +
                    chrono::Duration::hours(token_secret.token_expiration)
            },
        )
        .await?;

    // ? -----------------------------------------------------------------------
    // ? Build session Claims
    // ? -----------------------------------------------------------------------

    let current_date_time = chrono::Local::now();

    let mut claims = Claims::new().unwrap();

    // Set custom expiration, default is 1 hour
    claims
        .expiration(
            &{
                if is_for_password_change.is_some() {
                    current_date_time + chrono::Duration::hours(1)
                } else {
                    current_date_time +
                        chrono::Duration::minutes(
                            token_secret.token_expiration,
                        )
                }
            }
            .to_rfc3339(),
        )
        .unwrap();

    claims
        .add_additional("user_id", serde_json::json!(user_id))
        .unwrap();

    claims
        .add_additional("session_key", serde_json::json!(session_key))
        .unwrap();

    let symmetric_key = match SymmetricKey::<V4>::from(
        token_secret.token_secret_key.get()?.as_bytes(),
    ) {
        Ok(key) => key,
        Err(err) => {
            return use_case_err(format!(
                "Unable to generate confirmation token: {err}"
            ))
            .as_error()
        }
    };

    match local::encrypt(
        &symmetric_key,
        &claims,
        None,
        Some(token_secret.token_hmac_secret.get()?.as_bytes()),
    ) {
        Ok(token) => Ok(token),
        Err(err) => {
            return use_case_err(format!(
                "Unable to generate confirmation token: {err}"
            ))
            .as_error()
        }
    }
}
