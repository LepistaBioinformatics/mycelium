use crate::{
    models::webhook::WebHook as WebHookModel,
    repositories::{account::created_at_from_text, parse_optional_written_by},
    types::{json_from_text, uuid_from_text},
};

use myc_core::domain::dtos::webhook::WebHook;

/// Rebuilds the domain `WebHook` from its model row. `redact` mirrors the
/// postgres repos' inconsistent-but-intentional behavior: `get`/`list`/
/// `create`/`update` redact the secret token, but `list_by_trigger` (used
/// internally to sign outgoing payloads) does not.
pub(crate) fn map_model_to_dto(model: WebHookModel, redact: bool) -> WebHook {
    let mut webhook = WebHook::new(
        model.name,
        model.description,
        model.url,
        model.trigger.parse().unwrap(),
        model.method.map(|m| m.parse().unwrap()),
        model.secret.map(|s| {
            serde_json::from_value(json_from_text(&s).unwrap()).unwrap()
        }),
        parse_optional_written_by(
            model.created_by.map(|s| json_from_text(&s).unwrap()),
        ),
    );

    webhook.id = Some(uuid_from_text(&model.id).unwrap());
    webhook.is_active = model.is_active;
    webhook.created = created_at_from_text(&model.created);
    webhook.updated = model.updated.map(|dt| created_at_from_text(&dt));
    webhook.updated_by = parse_optional_written_by(
        model.updated_by.map(|s| json_from_text(&s).unwrap()),
    );

    if redact {
        webhook.redact_secret_token();
    }

    webhook
}
