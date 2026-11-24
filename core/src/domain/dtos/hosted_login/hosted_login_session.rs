use super::{
    generate_hosted_login_secret, HostedLoginAuthorization,
    HostedLoginCodeOutcome, HostedLoginEmailCode, HostedLoginRequest,
};

use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;

/// Server-side state of one hosted login, bound to the browser by a cookie
/// holding `session_id`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostedLoginSession {
    pub session_id: String,
    pub csrf_token: String,
    pub request: HostedLoginRequest,

    /// Set once, when the user submits the email.
    pub email_code: Option<HostedLoginEmailCode>,

    /// Set when the code is verified; exchanged once at the token endpoint.
    pub authorization: Option<HostedLoginAuthorization>,
}

impl HostedLoginSession {
    pub fn new(request: HostedLoginRequest) -> Self {
        Self {
            session_id: generate_hosted_login_secret(),
            csrf_token: generate_hosted_login_secret(),
            request,
            email_code: None,
            authorization: None,
        }
    }

    /// Whether the session already received its single code.
    pub fn code_already_sent(&self) -> bool {
        self.email_code.is_some() || self.authorization.is_some()
    }

    pub fn csrf_matches(&self, candidate: &str) -> bool {
        self.csrf_token
            .as_bytes()
            .ct_eq(candidate.as_bytes())
            .into()
    }

    /// Apply a typed code. On success the emailed code is replaced by an
    /// authorization holding `authorization_code`, so it cannot be used
    /// again; on failure an attempt is counted.
    pub fn check_code(
        &mut self,
        code: &str,
        authorization_code: String,
    ) -> HostedLoginCodeOutcome {
        let Some(email_code) = self.email_code.as_mut() else {
            return HostedLoginCodeOutcome::NoCodeSent;
        };

        if email_code.is_exhausted() {
            return HostedLoginCodeOutcome::Exhausted;
        }

        if email_code.code_matches(code) {
            self.authorization = Some(HostedLoginAuthorization {
                code: authorization_code,
                email: email_code.email.clone(),
            });
            self.email_code = None;
            return HostedLoginCodeOutcome::Verified;
        }

        if email_code.register_failed_attempt() {
            return HostedLoginCodeOutcome::Exhausted;
        }

        HostedLoginCodeOutcome::Rejected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::{
        email::Email, hosted_login::MAX_FAILED_HOSTED_LOGIN_ATTEMPTS,
    };

    fn session_with_code() -> HostedLoginSession {
        let mut session = HostedLoginSession::new(HostedLoginRequest {
            client_id: "webapp".to_string(),
            redirect_uri: "https://app.example.com/callback".to_string(),
            state: "xyz".to_string(),
            code_challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
                .to_string(),
        });

        session.email_code = Some(HostedLoginEmailCode::new(Email {
            username: "user".to_string(),
            domain: "example.com".to_string(),
        }));

        session
    }

    fn wrong_code(session: &HostedLoginSession) -> String {
        let code = &session.email_code.as_ref().unwrap().code;

        format!("{:06}", (code.parse::<u32>().unwrap() + 1) % 1_000_000)
    }

    #[test]
    fn new_session_has_distinct_secrets() {
        let session = session_with_code();

        assert_ne!(session.session_id, session.csrf_token);
        assert!(session.csrf_matches(&session.csrf_token.clone()));
        assert!(!session.csrf_matches(""));
    }

    #[test]
    fn right_code_issues_authorization_code() {
        let mut session = session_with_code();
        let code = session.email_code.as_ref().unwrap().code.clone();

        let outcome = session.check_code(&code, "auth-code".to_string());

        assert_eq!(outcome, HostedLoginCodeOutcome::Verified);
        let authorization = session.authorization.clone().unwrap();
        assert_eq!(authorization.code, "auth-code");
        assert_eq!(authorization.email.email(), "user@example.com");
        assert!(session.email_code.is_none());
        assert!(session.code_already_sent());
    }

    #[test]
    fn wrong_codes_exhaust_the_session() {
        let mut session = session_with_code();
        let wrong = wrong_code(&session);

        for _ in 1..MAX_FAILED_HOSTED_LOGIN_ATTEMPTS {
            assert_eq!(
                session.check_code(&wrong, "auth-code".to_string()),
                HostedLoginCodeOutcome::Rejected
            );
        }

        assert_eq!(
            session.check_code(&wrong, "auth-code".to_string()),
            HostedLoginCodeOutcome::Exhausted
        );
        assert!(session.authorization.is_none());

        // Once exhausted, even the right code is refused
        let right = session.email_code.as_ref().unwrap().code.clone();
        assert_eq!(
            session.check_code(&right, "auth-code".to_string()),
            HostedLoginCodeOutcome::Exhausted
        );
        assert!(session.authorization.is_none());
    }

    #[test]
    fn session_without_code_cannot_be_verified() {
        let mut session = session_with_code();
        session.email_code = None;

        assert_eq!(
            session.check_code("123456", "auth-code".to_string()),
            HostedLoginCodeOutcome::NoCodeSent
        );
    }
}
