use crate::{
    domain::{
        dtos::{
            hosted_login::{HostedLoginRequest, HostedLoginSession},
            native_error_codes::NativeErrorCodes,
            user::User,
        },
        entities::{
            HostedLoginSessionInvalidation, UserFetching, UserRegistration,
            UserUpdating,
        },
    },
    use_cases::role_scoped::beginner::user::ensure_principal_user_by_email,
};

use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};

/// Exchange an authorization code for the user it was issued to
/// (RFC 6749 §4.1.3 with PKCE, RFC 7636 §4.6).
///
/// The code is consumed before any check, so a failed exchange burns it.
#[tracing::instrument(name = "exchange_hosted_login_code", skip_all)]
pub async fn exchange_hosted_login_code(
    client_id: String,
    redirect_uri: String,
    authorization_code: String,
    code_verifier: String,
    session_invalidation_repo: Box<&dyn HostedLoginSessionInvalidation>,
    user_fetching_repo: Box<&dyn UserFetching>,
    user_registration_repo: Box<&dyn UserRegistration>,
    user_updating_repo: Box<&dyn UserUpdating>,
) -> Result<User, MappedErrors> {
    let session = match session_invalidation_repo
        .consume_authorization_code(&authorization_code)
        .await?
    {
        FetchResponseKind::Found(session) => session,
        FetchResponseKind::NotFound(_) => {
            return invalid_grant("Unknown or expired authorization code")
        }
    };

    check_exchange(&session, &client_id, &redirect_uri, &code_verifier)?;

    let Some(authorization) = session.authorization else {
        return invalid_grant("Authorization code without verified email");
    };

    ensure_principal_user_by_email(
        authorization.email,
        user_fetching_repo,
        user_registration_repo,
        user_updating_repo,
    )
    .await
}

fn check_exchange(
    session: &HostedLoginSession,
    client_id: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<(), MappedErrors> {
    let request = &session.request;

    if request.client_id != client_id || request.redirect_uri != redirect_uri {
        return invalid_grant("Client or redirect URI mismatch");
    }

    if !HostedLoginRequest::is_valid_code_verifier(code_verifier)
        || !request.pkce_matches(code_verifier)
    {
        return invalid_grant("PKCE verification failed");
    }

    Ok(())
}

fn invalid_grant<T>(msg: &str) -> Result<T, MappedErrors> {
    use_case_err(msg)
        .with_code(NativeErrorCodes::MYC00008)
        .with_exp_true()
        .as_error()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::{
        email::Email, hosted_login::HostedLoginAuthorization,
    };

    // RFC 7636 Appendix B test vector
    const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    const REDIRECT_URI: &str = "https://app.example.com/callback";

    fn verified_session() -> HostedLoginSession {
        let mut session = HostedLoginSession::new(HostedLoginRequest {
            client_id: "webapp".to_string(),
            redirect_uri: REDIRECT_URI.to_string(),
            state: "xyz".to_string(),
            code_challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
                .to_string(),
        });

        session.authorization = Some(HostedLoginAuthorization {
            code: "auth-code".to_string(),
            email: Email::from_string("user@example.com".to_string()).unwrap(),
        });

        session
    }

    #[test]
    fn accepts_matching_client_redirect_and_verifier() {
        assert!(check_exchange(
            &verified_session(),
            "webapp",
            REDIRECT_URI,
            VERIFIER
        )
        .is_ok());
    }

    #[test]
    fn rejects_other_client_or_redirect_uri() {
        let session = verified_session();

        assert!(
            check_exchange(&session, "other", REDIRECT_URI, VERIFIER).is_err()
        );
        assert!(check_exchange(
            &session,
            "webapp",
            "https://app.example.com/other",
            VERIFIER
        )
        .is_err());
    }

    #[test]
    fn rejects_wrong_or_malformed_verifier() {
        let session = verified_session();
        let wrong = format!("{}x", &VERIFIER[..VERIFIER.len() - 1]);

        assert!(
            check_exchange(&session, "webapp", REDIRECT_URI, &wrong).is_err()
        );
        assert!(
            check_exchange(&session, "webapp", REDIRECT_URI, "short").is_err()
        );
    }
}
