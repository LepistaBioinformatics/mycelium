use crate::domain::{
    dtos::{
        email::Email,
        hosted_login::{
            HostedLoginCodeAttachment, HostedLoginEmailCode,
            HostedLoginSession, MAX_PENDING_HOSTED_LOGIN_CODES_PER_EMAIL,
        },
        native_error_codes::NativeErrorCodes,
    },
    entities::{HostedLoginSessionFetching, HostedLoginSessionUpdating},
};

use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};

/// Bind a new emailed code to the session behind the browser cookie.
///
/// A session receives one code only: asking again means a new session, which
/// counts against the per-email cap.
#[tracing::instrument(name = "attach_hosted_login_code", skip_all)]
pub async fn attach_hosted_login_code(
    session_id: String,
    csrf_token: String,
    email: Email,
    session_fetching_repo: Box<&dyn HostedLoginSessionFetching>,
    session_updating_repo: Box<&dyn HostedLoginSessionUpdating>,
) -> Result<(HostedLoginSession, HostedLoginEmailCode), MappedErrors> {
    let session =
        match session_fetching_repo.get_by_session_id(&session_id).await? {
            FetchResponseKind::Found(session) => session,
            FetchResponseKind::NotFound(_) => {
                return rejected("Hosted login session not found")
            }
        };

    if !session.csrf_matches(&csrf_token) {
        return rejected("Invalid CSRF token");
    }

    let email_code = HostedLoginEmailCode::new(email);

    match session_updating_repo
        .attach_email_code(
            &session_id,
            email_code.clone(),
            MAX_PENDING_HOSTED_LOGIN_CODES_PER_EMAIL,
        )
        .await?
    {
        HostedLoginCodeAttachment::Attached => Ok((session, email_code)),
        HostedLoginCodeAttachment::AlreadySent => {
            rejected("A code was already sent for this session")
        }
        HostedLoginCodeAttachment::EmailCapReached => {
            rejected("Too many pending codes for this email")
        }
        HostedLoginCodeAttachment::NotFound => {
            rejected("Hosted login session not found")
        }
    }
}

fn rejected<T>(msg: &str) -> Result<T, MappedErrors> {
    use_case_err(msg)
        .with_code(NativeErrorCodes::MYC00008)
        .with_exp_true()
        .as_error()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        dtos::hosted_login::HostedLoginRequest,
        entities::{
            MockHostedLoginSessionFetching, MockHostedLoginSessionUpdating,
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

    fn email() -> Email {
        Email::from_string("user@example.com".to_string()).unwrap()
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

    fn updating_returning(
        attachment: HostedLoginCodeAttachment,
        times: usize,
    ) -> MockHostedLoginSessionUpdating {
        let mut updating = MockHostedLoginSessionUpdating::new();
        updating
            .expect_attach_email_code()
            .times(times)
            .returning(move |_, _, _| Ok(attachment.clone()));
        updating
    }

    async fn attach(
        csrf_token: String,
        fetching: MockHostedLoginSessionFetching,
        updating: MockHostedLoginSessionUpdating,
    ) -> Result<(HostedLoginSession, HostedLoginEmailCode), MappedErrors> {
        attach_hosted_login_code(
            "session".to_string(),
            csrf_token,
            email(),
            Box::new(&fetching),
            Box::new(&updating),
        )
        .await
    }

    #[tokio::test]
    async fn attaches_code_with_valid_csrf() -> Result<(), MappedErrors> {
        let session = session();
        let csrf = session.csrf_token.clone();

        let (_, email_code) = attach(
            csrf,
            fetching_returning(session),
            updating_returning(HostedLoginCodeAttachment::Attached, 1),
        )
        .await?;

        assert_eq!(email_code.email, email());
        assert_eq!(email_code.failed_attempts, 0);

        Ok(())
    }

    #[tokio::test]
    async fn rejects_wrong_csrf_without_touching_the_session() {
        let result = attach(
            "forged".to_string(),
            fetching_returning(session()),
            updating_returning(HostedLoginCodeAttachment::Attached, 0),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_second_code_and_email_cap() {
        for attachment in [
            HostedLoginCodeAttachment::AlreadySent,
            HostedLoginCodeAttachment::EmailCapReached,
            HostedLoginCodeAttachment::NotFound,
        ] {
            let session = session();
            let csrf = session.csrf_token.clone();

            let result = attach(
                csrf,
                fetching_returning(session),
                updating_returning(attachment, 1),
            )
            .await;

            assert!(result.is_err());
        }
    }
}
