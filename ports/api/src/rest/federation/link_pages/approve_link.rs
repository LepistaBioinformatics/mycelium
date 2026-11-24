use super::{
    enabled_federation::enabled_federation, render_link_page::render_link_error,
};
use crate::{
    federation::PreparedFederation,
    models::active_backend_modules::SqlAppModule,
};

use actix_web::{http::header::LOCATION, web, HttpResponse};
use myc_core::{
    domain::entities::PeerLinkNotifier,
    use_cases::federation::complete_link_on_issuer,
};
use myc_http_tools::federation::{link_secret_matches, LinkSecretPurpose};
use serde::Deserialize;
use shaku::HasComponent;
use uuid::Uuid;

const REFUSED: &str = "The link could not be recorded: it was already \
                       confirmed, it expired, or the other instance did not \
                       accept it. Start again from the application.";

#[derive(Deserialize)]
pub(crate) struct ApproveLinkForm {
    consent: Uuid,
    csrf_token: String,
}

/// The user confirmed: record the link on both sides and go back to the
/// consumer (FED-12, FED-46)
#[tracing::instrument(name = "approve_link", skip_all)]
pub(crate) async fn approve_link(
    form: web::Form<ApproveLinkForm>,
    prepared: web::Data<Option<PreparedFederation>>,
    sql_module: web::Data<SqlAppModule>,
    notifier: web::Data<dyn PeerLinkNotifier>,
) -> HttpResponse {
    let Some(prepared) = enabled_federation(&prepared, &sql_module).await
    else {
        return HttpResponse::NotFound().finish();
    };
    if !link_secret_matches(
        &prepared.signing.primary,
        LinkSecretPurpose::Csrf,
        form.consent,
        &form.csrf_token,
    ) {
        return render_link_error(REFUSED);
    }

    match complete_link_on_issuer(
        form.consent,
        Box::new(sql_module.resolve_ref()),
        Box::new(sql_module.resolve_ref()),
        Box::new(sql_module.resolve_ref()),
        Box::new(sql_module.resolve_ref()),
        Box::new(notifier.get_ref()),
    )
    .await
    {
        Ok(consent) => HttpResponse::SeeOther()
            .insert_header((LOCATION, consent.return_url))
            .finish(),
        Err(err) => {
            tracing::warn!(%err, "link approval failed");
            render_link_error(REFUSED)
        }
    }
}
