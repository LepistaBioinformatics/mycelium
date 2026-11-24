use super::{
    audit_translation_change::audit_translation_change,
    ensure_federation_operator, TranslationPorts,
};
use crate::domain::dtos::{
    federation::GrantTranslationRule, profile::Profile,
    resource_audit_log::ResourceAuditEventKind, written_by::WrittenBy,
};

use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use serde_json::json;
use uuid::Uuid;

/// The rules of one peer, or every rule (staff only)
#[tracing::instrument(name = "list_translation_rules", skip_all)]
pub async fn list_translation_rules(
    profile: &Profile,
    peer_id: Option<Uuid>,
    ports: TranslationPorts<'_>,
) -> Result<Vec<GrantTranslationRule>, MappedErrors> {
    ensure_federation_operator(profile)?;
    ports.read.list(peer_id).await
}

/// Enable or disable a rule; it applies from the next request (FED-52)
#[tracing::instrument(name = "set_translation_rule_enabled", skip_all)]
pub async fn set_translation_rule_enabled(
    profile: &Profile,
    rule_id: Uuid,
    enabled: bool,
    ports: TranslationPorts<'_>,
) -> Result<GrantTranslationRule, MappedErrors> {
    ensure_federation_operator(profile)?;
    let author = WrittenBy::new_from_account(profile.acc_id);

    let Some(rule) = ports
        .write
        .set_enabled(rule_id, enabled, Some(author.clone()))
        .await?
    else {
        return use_case_err("unknown rule").with_exp_true().as_error();
    };

    let metadata = json!({ "enabled": enabled });
    let event = ResourceAuditEventKind::Updated;
    audit_translation_change(rule_id, event, metadata, &author, ports).await;

    Ok(rule)
}

/// Delete a rule
#[tracing::instrument(name = "delete_translation_rule", skip_all)]
pub async fn delete_translation_rule(
    profile: &Profile,
    rule_id: Uuid,
    ports: TranslationPorts<'_>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    ensure_federation_operator(profile)?;
    let author = WrittenBy::new_from_account(profile.acc_id);

    let deleted = ports.write.delete(rule_id).await?;
    if matches!(deleted, DeletionResponseKind::Deleted) {
        let event = ResourceAuditEventKind::Deleted;
        audit_translation_change(rule_id, event, json!({}), &author, ports)
            .await;
    }

    Ok(deleted)
}
