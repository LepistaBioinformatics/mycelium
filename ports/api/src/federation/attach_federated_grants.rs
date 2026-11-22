use super::{
    resolution_limits, resolution_ports, PreparedFederation, RefreshThrottle,
};
use crate::{
    models::active_backend_modules::SqlAppModule, settings::ADMIN_API_SCOPE,
};

use actix_web::{web, HttpRequest};
use myc_core::{
    domain::{
        dtos::{email::Email, profile::Profile, written_by::WrittenBy},
        entities::PeerGrantResolver,
    },
    use_cases::federation::{
        is_federation_enabled, locally_denied, read_federated_grants,
        resolve_own_federated_grants,
    },
};
use shaku::HasComponent;
use std::time::Duration;
use uuid::Uuid;

/// Attach the subject's federated grants, read from the store, to the
/// profile of this request (FED-11, FED-26, FED-30)
///
/// Never calls a peer and never delays the request: a missing or due
/// assertion starts a throttled background refresh when the request is
/// routed to a downstream service. The gateway's own API never starts one:
/// `beginners.federation.resolveGrants` refreshes on demand, and a second,
/// concurrent call to the same peer would be waste. Nothing is attached
/// while federation is off or unconfigured, or for a locally denied subject
/// (FED-19). The profile cache never holds this part.
///
#[tracing::instrument(name = "attach_federated_grants", skip_all)]
pub(crate) async fn attach_federated_grants(
    mut profile: Profile,
    req: &HttpRequest,
    email: &Email,
) -> Profile {
    let (Some(prepared), Some(app_module), Some(subject)) = (
        prepared(req),
        req.app_data::<web::Data<SqlAppModule>>(),
        owner(&profile, email),
    ) else {
        return profile;
    };
    if locally_denied(&profile).is_some() {
        return profile;
    }

    let refresh_after = resolution_limits(prepared).assertion_refresh;
    let read = match read_federated_grants(
        subject,
        refresh_after,
        Box::new(app_module.resolve_ref()),
        Box::new(app_module.resolve_ref()),
        Box::new(app_module.resolve_ref()),
    )
    .await
    {
        Ok(read) => read,
        Err(err) => {
            tracing::error!(%err, "federated grants not read");
            return profile;
        }
    };

    if read.resources.is_empty() && !read.refresh_due {
        return profile;
    }
    if !enabled(app_module).await {
        return profile;
    }
    if read.refresh_due && routed_downstream(req) {
        refresh_in_background(req, &profile, subject, email, refresh_after);
    }
    if !read.resources.is_empty() {
        profile.federated_resources = Some(read.resources);
    }

    profile
}

fn prepared(req: &HttpRequest) -> Option<&PreparedFederation> {
    req.app_data::<web::Data<Option<PreparedFederation>>>()
        .and_then(|prepared| prepared.get_ref().as_ref())
}

/// Whether the request goes to a downstream service, not the gateway's API
fn routed_downstream(req: &HttpRequest) -> bool {
    !req.path()
        .trim_start_matches('/')
        .starts_with(ADMIN_API_SCOPE)
}

fn owner(profile: &Profile, email: &Email) -> Option<Uuid> {
    let email = email.email();

    profile
        .owners
        .iter()
        .find(|owner| owner.email.eq_ignore_ascii_case(&email))
        .map(|owner| owner.id)
}

async fn enabled(app_module: &SqlAppModule) -> bool {
    matches!(
        is_federation_enabled(Box::new(app_module.resolve_ref())).await,
        Ok(true)
    )
}

/// Ask the subject's peers again, off the request (at most once per
/// subject per `assertionRefreshSecs`)
fn refresh_in_background(
    req: &HttpRequest,
    profile: &Profile,
    subject: Uuid,
    email: &Email,
    window: Duration,
) {
    let (Some(prepared), Some(app_module), Some(resolver), Some(throttle)) = (
        prepared(req).cloned(),
        req.app_data::<web::Data<SqlAppModule>>().cloned(),
        req.app_data::<web::Data<dyn PeerGrantResolver>>().cloned(),
        req.app_data::<web::Data<RefreshThrottle>>(),
    ) else {
        return;
    };
    if !throttle.admit(subject, window) {
        return;
    }

    let profile = profile.clone();
    let author = WrittenBy::new_from_user_with_email(subject, &email.email());
    tokio::spawn(async move {
        let ports = resolution_ports(&app_module, resolver.get_ref());
        let limits = resolution_limits(&prepared);

        if let Err(err) = resolve_own_federated_grants(
            &profile, subject, &author, limits, &ports,
        )
        .await
        {
            tracing::warn!(%err, "background federated refresh failed");
        }
    });
}
