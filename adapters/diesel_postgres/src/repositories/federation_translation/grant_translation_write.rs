use super::map_translation_row::map_translation_row_to_dto;
use crate::{
    models::{config::DbPoolProvider, federation_translation::TranslationRow},
    schema::federation_grant_translation as rule_model,
};

use async_trait::async_trait;
use chrono::Utc;
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};
use myc_core::domain::{
    dtos::{
        federation::{GrantTranslationRule, NewGrantTranslationRule},
        written_by::WrittenBy,
    },
    entities::GrantTranslationWrite,
};
use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use serde_json::Value as JsonValue;
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GrantTranslationWrite)]
pub struct GrantTranslationWriteSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

impl GrantTranslationWriteSqlDbRepository {
    fn connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, MappedErrors>
    {
        self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {e}"))
        })
    }
}

fn write_err(e: diesel::result::Error) -> MappedErrors {
    creation_err(format!("Failed to write a translation rule: {e}"))
}

fn author(written_by: Option<WrittenBy>) -> Option<JsonValue> {
    written_by.and_then(|author| serde_json::to_value(author).ok())
}

#[async_trait]
impl GrantTranslationWrite for GrantTranslationWriteSqlDbRepository {
    #[tracing::instrument(name = "grant_translation_create", skip_all)]
    async fn create(
        &self,
        rule: NewGrantTranslationRule,
        created_by: Option<WrittenBy>,
    ) -> Result<GrantTranslationRule, MappedErrors> {
        diesel::insert_into(rule_model::table)
            .values((
                rule_model::peer_id.eq(rule.peer_id),
                rule_model::issuer_tenant_id.eq(rule.issuer.tenant_id),
                rule_model::issuer_role.eq(&rule.issuer.role),
                rule_model::issuer_perm.eq(rule.issuer.min_perm.to_i32()),
                rule_model::local_account_id.eq(rule.local.account_id),
                rule_model::local_guest_role_id.eq(rule.local.guest_role_id),
                rule_model::local_perm.eq(rule.local.perm.to_i32()),
                rule_model::created_by.eq(author(created_by)),
            ))
            .returning(TranslationRow::as_returning())
            .get_result::<TranslationRow>(&mut *self.connection()?)
            .map(map_translation_row_to_dto)
            .map_err(write_err)
    }

    #[tracing::instrument(name = "grant_translation_set_enabled", skip_all)]
    async fn set_enabled(
        &self,
        rule_id: Uuid,
        enabled: bool,
        updated_by: Option<WrittenBy>,
    ) -> Result<Option<GrantTranslationRule>, MappedErrors> {
        diesel::update(rule_model::table.find(rule_id))
            .set((
                rule_model::enabled.eq(enabled),
                rule_model::updated.eq(Some(Utc::now())),
                rule_model::updated_by.eq(author(updated_by)),
            ))
            .returning(TranslationRow::as_returning())
            .get_result::<TranslationRow>(&mut *self.connection()?)
            .optional()
            .map(|row| row.map(map_translation_row_to_dto))
            .map_err(write_err)
    }

    #[tracing::instrument(name = "grant_translation_delete", skip_all)]
    async fn delete(
        &self,
        rule_id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        let deleted = diesel::delete(rule_model::table.find(rule_id))
            .execute(&mut *self.connection()?)
            .map_err(write_err)?;

        Ok(match deleted {
            0 => DeletionResponseKind::NotDeleted(
                rule_id,
                "unknown rule".to_string(),
            ),
            _ => DeletionResponseKind::Deleted,
        })
    }
}
