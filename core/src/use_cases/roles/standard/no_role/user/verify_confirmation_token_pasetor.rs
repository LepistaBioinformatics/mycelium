use crate::{
    domain::{
        dtos::session_token::{SessionToken, TokenSecret},
        entities::{SessionTokenDeletion, SessionTokenFetching},
    },
    settings::build_session_key,
};

use clean_base::utils::errors::{factories::use_case_err, MappedErrors};
use pasetors::{
    claims::ClaimsValidationRules, keys::SymmetricKey, local,
    token::UntrustedToken, version4::V4, Local,
};

pub async fn verify_confirmation_token_pasetor(
    token: String,
    is_for_password_change: Option<bool>,
    token_secret: TokenSecret,
    token_fetching_repo: Box<&dyn SessionTokenFetching>,
    token_deletion_repo: Box<&dyn SessionTokenDeletion>,
) -> Result<SessionToken, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Validate token
    // ? -----------------------------------------------------------------------

    let symmetric_key =
        SymmetricKey::<V4>::from(token_secret.token_secret_key.as_bytes())
            .unwrap();

    let validation_rules = ClaimsValidationRules::new();
    let untrusted_token = match UntrustedToken::<Local, V4>::try_from(&token) {
        Ok(token) => token,
        Err(err) => {
            token_deletion_repo.delete(token).await?;
            return use_case_err(format!("Pasetor: {err}")).as_error();
        }
    };

    let trusted_token = match local::decrypt(
        &symmetric_key,
        &untrusted_token,
        &validation_rules,
        None,
        Some(token_secret.token_hmac_secret.as_bytes()),
    ) {
        Ok(token) => token,
        Err(err) => {
            token_deletion_repo.delete(token).await?;
            return use_case_err(format!("Pasetor: {err}")).as_error();
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Build session Claims
    // ? -----------------------------------------------------------------------

    let claims = trusted_token.payload_claims().unwrap();

    let uid =
        serde_json::to_value(claims.get_claim("user_id").unwrap()).unwrap();

    match serde_json::from_value::<String>(uid) {
        Ok(uuid_string) => match uuid::Uuid::parse_str(&uuid_string) {
            Ok(user_uuid) => {
                let sss_key = serde_json::to_value(
                    claims.get_claim("session_key").unwrap(),
                )
                .unwrap();
                let session_key =
                    match serde_json::from_value::<String>(sss_key) {
                        Ok(session_key) => session_key,
                        Err(e) => {
                            return use_case_err(format!("{}", e)).as_error()
                        }
                    };

                let data_storage_key =
                    SessionToken::build_prefixed_session_token(
                        session_key.to_owned(),
                        is_for_password_change,
                    );

                token_fetching_repo
                    .get(build_session_key(data_storage_key.to_owned()))
                    .await?;

                token_deletion_repo
                    .delete(build_session_key(data_storage_key))
                    .await?;

                Ok(SessionToken { user_id: user_uuid })
            }
            Err(err) => use_case_err(format!("{err}")).as_error(),
        },

        Err(err) => use_case_err(format!("{err}")).as_error(),
    }
}
