use crate::models::active_backend_modules::SqlAppModule;

use actix_web::{web, HttpRequest, HttpResponse};
use myc_core::{
    domain::entities::InstanceSettingsFetching, models::AccountLifeCycle,
    use_cases::super_users::staff::bootstrap::staff_bootstrap_is_pending,
};
use myc_http_tools::utils::HttpJsonResponse;
use shaku::HasComponent;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tracing::warn;

pub(crate) const STAFF_BOOTSTRAP_PENDING_MESSAGE: &str =
    "This instance is waiting for its staff bootstrap. Nobody can sign in \
     until the initial staff account is claimed.";

/// Process-wide latch: once the staff bootstrap is seen as claimed it stays
/// claimed, so the gate stops reading the database. Registered as app data.
#[derive(Clone, Default)]
pub(crate) struct StaffBootstrapClaimed(Arc<AtomicBool>);

impl StaffBootstrapClaimed {
    fn is_claimed(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    fn mark_claimed(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

/// Whether sign-in must be refused because the staff bootstrap is enabled
/// (`staff_bootstrap_secret` configured) and not claimed yet.
///
/// Signing in before the bootstrap creates a regular account for the user,
/// which a later bootstrap with the same email cannot turn into Staff. No
/// secret configured means no bootstrap flow and no gate. Fails closed when
/// the state cannot be read.
pub(crate) async fn sign_in_blocked_by_staff_bootstrap(
    req: &HttpRequest,
) -> bool {
    let Some(life_cycle) = req.app_data::<web::Data<AccountLifeCycle>>() else {
        return false;
    };

    if life_cycle.staff_bootstrap_secret.is_none() {
        return false;
    }

    let latch = req.app_data::<web::Data<StaffBootstrapClaimed>>();

    if latch.is_some_and(|latch| latch.is_claimed()) {
        return false;
    }

    let Some(sql_module) = req.app_data::<web::Data<SqlAppModule>>() else {
        warn!("Staff bootstrap gate: no SQL module; refusing sign-in");
        return true;
    };

    let settings_repo: &dyn InstanceSettingsFetching = sql_module.resolve_ref();

    match staff_bootstrap_is_pending(Box::new(settings_repo)).await {
        Ok(true) => true,
        Ok(false) => {
            if let Some(latch) = latch {
                latch.mark_claimed();
            }
            false
        }
        Err(err) => {
            warn!("Staff bootstrap gate: state unreadable ({err}); refusing");
            true
        }
    }
}

pub(crate) fn staff_bootstrap_pending_response() -> HttpResponse {
    HttpResponse::Forbidden().json(HttpJsonResponse::new_message(
        STAFF_BOOTSTRAP_PENDING_MESSAGE.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;
    use myc_config::secret_resolver::SecretResolver;

    fn life_cycle(secret: Option<&str>) -> AccountLifeCycle {
        let mut settings: AccountLifeCycle =
            serde_json::from_value(serde_json::json!({
                "domainName": "example.com",
                "noreplyEmail": "noreply@example.com",
                "supportEmail": "support@example.com",
                "tokenSecret": "placeholder",
                "hmacSecrets": [{ "version": 1, "secret": "placeholder" }],
            }))
            .unwrap();
        settings.staff_bootstrap_secret =
            secret.map(|s| SecretResolver::Value(s.to_string()));
        settings
    }

    #[actix_web::test]
    async fn no_secret_means_no_gate() {
        let req = TestRequest::default()
            .app_data(web::Data::new(life_cycle(None)))
            .to_http_request();

        assert!(!sign_in_blocked_by_staff_bootstrap(&req).await);
    }

    #[actix_web::test]
    async fn claimed_latch_skips_the_database() {
        let latch = StaffBootstrapClaimed::default();
        latch.mark_claimed();

        // No SQL module registered: reaching the database would block.
        let req = TestRequest::default()
            .app_data(web::Data::new(life_cycle(Some("secret"))))
            .app_data(web::Data::new(latch))
            .to_http_request();

        assert!(!sign_in_blocked_by_staff_bootstrap(&req).await);
    }

    #[actix_web::test]
    async fn unreadable_state_fails_closed() {
        let req = TestRequest::default()
            .app_data(web::Data::new(life_cycle(Some("secret"))))
            .app_data(web::Data::new(StaffBootstrapClaimed::default()))
            .to_http_request();

        assert!(sign_in_blocked_by_staff_bootstrap(&req).await);
    }
}
