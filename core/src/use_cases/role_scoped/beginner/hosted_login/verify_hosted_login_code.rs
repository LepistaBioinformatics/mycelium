use crate::domain::{
    dtos::{
        hosted_login::{generate_hosted_login_secret, HostedLoginCodeCheck},
        native_error_codes::NativeErrorCodes,
    },
    entities::{HostedLoginSessionFetching, HostedLoginSessionInvalidation},
};

use chrono::{Duration, Local};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};

/// How long the client has to exchange the authorization code.
pub const HOSTED_LOGIN_AUTHORIZATION_CODE_TTL_SECONDS: i64 = 60;

/// Check the code typed on the hosted login page. On success the session
/// holds a short-lived authorization code for the client to exchange.
#[tracing::instrument(name = "verify_hosted_login_code", skip_all)]
pub async fn verify_hosted_login_code(
    session_id: String,
    csrf_token: String,
    code: String,
    session_fetching_repo: Box<&dyn HostedLoginSessionFetching>,
    session_invalidation_repo: Box<&dyn HostedLoginSessionInvalidation>,
) -> Result<HostedLoginCodeCheck, MappedErrors> {
    let session =
        match session_fetching_repo.get_by_session_id(&session_id).await? {
            FetchResponseKind::Found(session) => session,
            FetchResponseKind::NotFound(_) => {
                return Ok(HostedLoginCodeCheck::NotFound)
            }
        };

    if !session.csrf_matches(&csrf_token) {
        return use_case_err("Invalid CSRF token")
            .with_code(NativeErrorCodes::MYC00008)
            .with_exp_true()
            .as_error();
    }

    session_invalidation_repo
        .check_code_and_issue_authorization_code(
            &session_id,
            &code,
            generate_hosted_login_secret(),
            Local::now()
                + Duration::seconds(
                    HOSTED_LOGIN_AUTHORIZATION_CODE_TTL_SECONDS,
                ),
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        dtos::hosted_login::{HostedLoginRequest, HostedLoginSession},
        entities::{
            MockHostedLoginSessionFetching, MockHostedLoginSessionInvalidation,
        },
    };

    fn session() -> HostedLoginSession {
        HostedLoginSession::new(HostedLoginRequest {
            client_id: "webapp".to_string(),
            redirect_uri: "https://app.example.com/callback".to_string(),
            state: "xyz".to_string(),
            code_challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
                .to_string(),
        })
    }

    fn fetching_returning(
        session: HostedLoginSession,
    ) -> MockHostedLoginSessionFetching {
        let mut fetching = MockHostedLoginSessionFetching::new();
        fetching
            .expect_get_by_session_id()
            .returning(move |_| Ok(FetchResponseKind::Found(session.clone())));
        fetching
    }

    #[tokio::test]
    async fn delegates_code_check_with_short_lived_authorization_code(
    ) -> Result<(), MappedErrors> {
        let session = session();
        let csrf = session.csrf_token.clone();
        let verified = session.clone();

        let mut invalidation = MockHostedLoginSessionInvalidation::new();
        invalidation
            .expect_check_code_and_issue_authorization_code()
            .times(1)
            .withf(|_, code, authorization_code, expires| {
                let ttl = *expires - Local::now();

                code == "123456"
                    && authorization_code.len() == 43
                    && ttl <= Duration::seconds(60)
                    && ttl > Duration::seconds(50)
            })
            .returning(move |_, _, _, _| {
                Ok(HostedLoginCodeCheck::Verified(verified.clone()))
            });

        let check = verify_hosted_login_code(
            "session".to_string(),
            csrf,
            "123456".to_string(),
            Box::new(&fetching_returning(session)),
            Box::new(&invalidation),
        )
        .await?;

        assert!(matches!(check, HostedLoginCodeCheck::Verified(_)));

        Ok(())
    }

    #[tokio::test]
    async fn rejects_wrong_csrf_before_checking_the_code() {
        let mut invalidation = MockHostedLoginSessionInvalidation::new();
        invalidation
            .expect_check_code_and_issue_authorization_code()
            .times(0);

        let result = verify_hosted_login_code(
            "session".to_string(),
            "forged".to_string(),
            "123456".to_string(),
            Box::new(&fetching_returning(session())),
            Box::new(&invalidation),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn missing_session_is_not_found() -> Result<(), MappedErrors> {
        let mut fetching = MockHostedLoginSessionFetching::new();
        fetching
            .expect_get_by_session_id()
            .returning(|_| Ok(FetchResponseKind::NotFound(None)));

        let check = verify_hosted_login_code(
            "session".to_string(),
            "csrf".to_string(),
            "123456".to_string(),
            Box::new(&fetching),
            Box::new(&MockHostedLoginSessionInvalidation::new()),
        )
        .await?;

        assert_eq!(check, HostedLoginCodeCheck::NotFound);

        Ok(())
    }
}
