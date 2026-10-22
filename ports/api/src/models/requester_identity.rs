use actix_web::{HttpMessage, HttpRequest};
use myc_core::domain::dtos::email::Email;
use uuid::Uuid;

/// Who made the request, as far as the gateway resolved it. Stored in the
/// request extensions while identity is resolved and read once, by
/// `log_served_request`, for the single per-request log line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RequesterIdentity {
    /// Already redacted (`Email::redacted_email`).
    pub(crate) email: Option<String>,
    pub(crate) account_id: Option<Uuid>,
}

impl RequesterIdentity {
    pub(crate) fn record_email(req: &HttpRequest, email: &Email) {
        let mut extensions = req.extensions_mut();
        let mut identity = extensions
            .get::<RequesterIdentity>()
            .cloned()
            .unwrap_or_default();

        identity.email = Some(email.redacted_email());
        extensions.insert(identity);
    }

    pub(crate) fn record_account(req: &HttpRequest, account_id: Uuid) {
        let mut extensions = req.extensions_mut();
        let mut identity = extensions
            .get::<RequesterIdentity>()
            .cloned()
            .unwrap_or_default();

        identity.account_id = Some(account_id);
        extensions.insert(identity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    #[test]
    fn email_and_account_accumulate_on_the_request() {
        let req = TestRequest::default().to_http_request();
        let email = Email::from_string("someone@example.com".into()).unwrap();
        let account_id = Uuid::new_v4();

        RequesterIdentity::record_email(&req, &email);
        RequesterIdentity::record_account(&req, account_id);

        let identity = req.extensions().get::<RequesterIdentity>().cloned();

        assert_eq!(
            identity,
            Some(RequesterIdentity {
                email: Some(email.redacted_email()),
                account_id: Some(account_id),
            })
        );
    }

    #[test]
    fn a_clone_of_the_request_shares_the_identity() {
        let req = TestRequest::default().to_http_request();
        let email = Email::from_string("someone@example.com".into()).unwrap();

        RequesterIdentity::record_email(&req.clone(), &email);

        assert!(req.extensions().get::<RequesterIdentity>().is_some());
    }
}
