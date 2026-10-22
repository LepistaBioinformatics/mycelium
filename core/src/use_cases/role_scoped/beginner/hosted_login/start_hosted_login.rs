use crate::{
    domain::{
        dtos::{
            hosted_login::{
                HostedLoginClients, HostedLoginRequest, HostedLoginSession,
            },
            native_error_codes::NativeErrorCodes,
        },
        entities::HostedLoginSessionRegistration,
    },
    models::AccountLifeCycle,
};

use chrono::{Duration, Local};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};

/// Validate an authorization request and open a hosted login session.
///
/// Only the authorization code grant with PKCE S256 is accepted, and the
/// redirect URI must be registered for the client (exact match). The caller
/// must not redirect anywhere when this fails.
#[tracing::instrument(name = "start_hosted_login", skip_all)]
pub async fn start_hosted_login(
    clients: HostedLoginClients,
    response_type: String,
    code_challenge_method: String,
    request: HostedLoginRequest,
    life_cycle_settings: AccountLifeCycle,
    session_registration_repo: Box<&dyn HostedLoginSessionRegistration>,
) -> Result<HostedLoginSession, MappedErrors> {
    validate_authorization_request(
        &clients,
        &response_type,
        &code_challenge_method,
        &request,
    )?;

    let expires = Local::now()
        + Duration::seconds(
            life_cycle_settings
                .token_expiration
                .async_get_or_error()
                .await?,
        );

    match session_registration_repo
        .create(HostedLoginSession::new(request), expires)
        .await?
    {
        CreateResponseKind::Created(session) => Ok(session),
        CreateResponseKind::NotCreated(_, msg) => use_case_err(msg).as_error(),
    }
}

fn validate_authorization_request(
    clients: &HostedLoginClients,
    response_type: &str,
    code_challenge_method: &str,
    request: &HostedLoginRequest,
) -> Result<(), MappedErrors> {
    if !clients.allows(&request.client_id, &request.redirect_uri) {
        return invalid_request("Unknown client or redirect URI");
    }

    if response_type != "code" || code_challenge_method != "S256" {
        return invalid_request("Only response_type=code with PKCE S256");
    }

    if request.state.is_empty() || !request.has_valid_code_challenge() {
        return invalid_request("Missing state or invalid code challenge");
    }

    Ok(())
}

fn invalid_request(msg: &str) -> Result<(), MappedErrors> {
    use_case_err(msg)
        .with_code(NativeErrorCodes::MYC00008)
        .with_exp_true()
        .as_error()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::dtos::hosted_login::HostedLoginClient,
        models::{HmacSecretEntry, HmacSecretSet},
    };

    use async_trait::async_trait;
    use chrono::DateTime;
    use myc_config::secret_resolver::SecretResolver;
    use std::sync::Mutex;

    struct StubRegistration {
        created: Mutex<Vec<HostedLoginSession>>,
    }

    #[async_trait]
    impl HostedLoginSessionRegistration for StubRegistration {
        async fn create(
            &self,
            session: HostedLoginSession,
            _expires: DateTime<Local>,
        ) -> Result<CreateResponseKind<HostedLoginSession>, MappedErrors>
        {
            self.created.lock().unwrap().push(session.clone());
            Ok(CreateResponseKind::Created(session))
        }
    }

    fn clients() -> HostedLoginClients {
        HostedLoginClients(vec![HostedLoginClient {
            client_id: "webapp".to_string(),
            redirect_uris: vec!["https://app.example.com/callback".to_string()],
        }])
    }

    fn request() -> HostedLoginRequest {
        HostedLoginRequest {
            client_id: "webapp".to_string(),
            redirect_uri: "https://app.example.com/callback".to_string(),
            state: "xyz".to_string(),
            code_challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
                .to_string(),
        }
    }

    fn life_cycle() -> AccountLifeCycle {
        AccountLifeCycle {
            domain_name: SecretResolver::Value("example.com".to_string()),
            domain_url: None,
            locale: None,
            token_expiration: SecretResolver::Value(900),
            noreply_name: None,
            noreply_email: SecretResolver::Value(
                "noreply@example.com".to_string(),
            ),
            support_name: None,
            support_email: SecretResolver::Value(
                "support@example.com".to_string(),
            ),
            token_secret: SecretResolver::Value("placeholder".to_string()),
            hmac_primary_version: 1,
            hmac_secrets: HmacSecretSet::new(vec![HmacSecretEntry {
                version: 1,
                secret: SecretResolver::Value("placeholder".to_string()),
            }]),
            staff_bootstrap_secret: None,
        }
    }

    async fn start(
        request: HostedLoginRequest,
        response_type: &str,
        method: &str,
        repo: &StubRegistration,
    ) -> Result<HostedLoginSession, MappedErrors> {
        start_hosted_login(
            clients(),
            response_type.to_string(),
            method.to_string(),
            request,
            life_cycle(),
            Box::new(repo),
        )
        .await
    }

    fn repo() -> StubRegistration {
        StubRegistration {
            created: Mutex::new(vec![]),
        }
    }

    #[tokio::test]
    async fn opens_session_for_valid_request() -> Result<(), MappedErrors> {
        let repo = repo();

        let session = start(request(), "code", "S256", &repo).await?;

        assert_eq!(session.request, request());
        assert!(session.email_code.is_none());
        assert_eq!(repo.created.lock().unwrap().len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn rejects_invalid_requests_without_creating_a_session() {
        let repo = repo();

        let mut unknown_redirect = request();
        unknown_redirect.redirect_uri = "https://evil.example".to_string();

        let mut no_state = request();
        no_state.state = String::new();

        let mut bad_challenge = request();
        bad_challenge.code_challenge = "plain-text".to_string();

        assert!(start(unknown_redirect, "code", "S256", &repo)
            .await
            .is_err());
        assert!(start(request(), "token", "S256", &repo).await.is_err());
        assert!(start(request(), "code", "plain", &repo).await.is_err());
        assert!(start(no_state, "code", "S256", &repo).await.is_err());
        assert!(start(bad_challenge, "code", "S256", &repo).await.is_err());
        assert!(repo.created.lock().unwrap().is_empty());
    }
}
