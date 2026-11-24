use super::{render_service_attribution, resolve_service_email_locale};
use crate::{
    domain::{
        dtos::{
            send_service_email_request::SendServiceEmailRequest,
            service_notification_config::ServiceNotificationConfig,
        },
        entities::TenantFetching,
    },
    models::{AccountLifeCycle, RenderedServiceEmail, ServiceTemplates},
};

use mycelium_base::utils::errors::MappedErrors;
use serde_json::Value;
use uuid::Uuid;

/// Render a service email and append the gateway attribution footer
///
/// The template is looked up only under the service own sender tag. The
/// footer is rendered in the locale the template was actually taken from.
#[tracing::instrument(name = "render_service_email", skip_all)]
pub(crate) async fn render_service_email(
    config: &ServiceNotificationConfig,
    request: &SendServiceEmailRequest,
    tenant_id: Option<Uuid>,
    service_templates: &ServiceTemplates,
    life_cycle: &AccountLifeCycle,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<RenderedServiceEmail, MappedErrors> {
    let locale = resolve_service_email_locale(
        request.locale.to_owned(),
        tenant_id,
        life_cycle,
        tenant_fetching_repo,
    )
    .await?;

    let mut rendered = service_templates.render(
        &config.sender_tag,
        &locale,
        &request.template,
        Value::Object(request.variables.to_owned()),
    )?;

    let footer = render_service_attribution(
        &rendered.locale,
        &config.display_name,
        life_cycle,
    )
    .await?;

    rendered.body = format!("{}\n{footer}", rendered.body);

    Ok(rendered)
}
