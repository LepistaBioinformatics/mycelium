use crate::domain::{
    dtos::webhook::{WebHookPayloadArtifact, WebHookTrigger},
    entities::WebHookRegistration,
};

use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use uuid::Uuid;

/// Register a webhook dispatching event
///
/// Webhooks should be dispatched asynchronously. Thus, webhooks should be
/// registered after their effective execution.
///
#[tracing::instrument(name = "register_webhook_dispatching_event", skip_all)]
pub(crate) async fn register_webhook_dispatching_event<
    PayloadBody: serde::ser::Serialize + Clone + Send + Sync + 'static,
>(
    correspondence_id: Uuid,
    trigger: WebHookTrigger,
    payload: PayloadBody,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
) -> Result<Uuid, MappedErrors> {
    tracing::trace!("Registering webhook dispatching event");

    // ? -----------------------------------------------------------------------
    // ? Initialize webhook response
    // ? -----------------------------------------------------------------------

    let artifact = WebHookPayloadArtifact::new(
        Some(correspondence_id),
        match serde_json::to_string(&payload) {
            Ok(payload) => payload,
            Err(err) => {
                tracing::error!("Failed to serialize payload: {:?}", err);

                return creation_err("Failed to serialize payload").as_error();
            }
        },
        trigger,
    )
    .encode_payload()?;

    // ? -----------------------------------------------------------------------
    // ? Register the webhook in datastore
    // ? -----------------------------------------------------------------------

    match webhook_registration_repo
        .register_execution_event(artifact)
        .await?
    {
        CreateResponseKind::Created(id) => {
            tracing::trace!("Webhook execution event registered");

            Ok(id)
        }
        CreateResponseKind::NotCreated(_, msgs) => {
            tracing::error!(
                "Failed to register webhook execution event: {:?}",
                msgs
            );

            return creation_err("Failed to register webhook execution event")
                .as_error();
        }
    }
}
