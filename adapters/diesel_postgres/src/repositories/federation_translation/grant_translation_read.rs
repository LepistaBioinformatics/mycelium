use super::map_translation_row::{
    map_active_row, map_translation_row_to_dto, ActiveTranslationRow,
};
use crate::{
    models::{config::DbPoolProvider, federation_translation::TranslationRow},
    schema::{
        account as account_model, federation_grant_translation as rule_model,
        federation_peer as peer_model, guest_role as role_model,
    },
};

use async_trait::async_trait;
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};
use myc_core::domain::{
    dtos::federation::{
        ActiveTranslation, GrantTranslationRule, TranslationTarget,
    },
    entities::GrantTranslationRead,
};
use mycelium_base::utils::errors::{fetching_err, MappedErrors};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GrantTranslationRead)]
pub struct GrantTranslationReadSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

impl GrantTranslationReadSqlDbRepository {
    fn connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, MappedErrors>
    {
        self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {e}"))
        })
    }
}

fn read_err(e: diesel::result::Error) -> MappedErrors {
    fetching_err(format!("Failed to read translation rules: {e}"))
}

#[async_trait]
impl GrantTranslationRead for GrantTranslationReadSqlDbRepository {
    #[tracing::instrument(name = "grant_translation_list", skip_all)]
    async fn list(
        &self,
        peer_id: Option<Uuid>,
    ) -> Result<Vec<GrantTranslationRule>, MappedErrors> {
        let mut query = rule_model::table
            .select(TranslationRow::as_select())
            .order(rule_model::created.desc())
            .into_boxed();

        if let Some(peer_id) = peer_id {
            query = query.filter(rule_model::peer_id.eq(peer_id));
        }

        Ok(query
            .load::<TranslationRow>(&mut *self.connection()?)
            .map_err(read_err)?
            .into_iter()
            .map(map_translation_row_to_dto)
            .collect())
    }

    #[tracing::instrument(name = "grant_translation_list_active", skip_all)]
    async fn list_active_for_peers(
        &self,
        peer_ids: &[Uuid],
    ) -> Result<Vec<ActiveTranslation>, MappedErrors> {
        let rows = rule_model::table
            .inner_join(peer_model::table)
            .inner_join(account_model::table)
            .inner_join(role_model::table)
            .filter(rule_model::peer_id.eq_any(peer_ids))
            .filter(rule_model::enabled.eq(true))
            .filter(account_model::is_deleted.eq(false))
            .select((
                TranslationRow::as_select(),
                peer_model::instance_id,
                account_model::name,
                account_model::tenant_id,
                role_model::slug,
            ))
            .load::<ActiveTranslationRow>(&mut *self.connection()?)
            .map_err(read_err)?;

        Ok(rows.into_iter().filter_map(map_active_row).collect())
    }

    #[tracing::instrument(name = "grant_translation_target", skip_all)]
    async fn target(
        &self,
        account_id: Uuid,
        guest_role_id: Uuid,
    ) -> Result<Option<TranslationTarget>, MappedErrors> {
        let conn = &mut *self.connection()?;
        let account = account_model::table
            .find(account_id)
            .filter(account_model::is_deleted.eq(false))
            .filter(account_model::is_default.eq(false))
            .select((account_model::name, account_model::tenant_id))
            .first::<(String, Option<Uuid>)>(conn)
            .optional()
            .map_err(read_err)?;
        let role = role_model::table
            .find(guest_role_id)
            .select(role_model::slug)
            .first::<String>(conn)
            .optional()
            .map_err(read_err)?;

        let (Some((account_name, Some(tenant_id))), Some(role_slug)) =
            (account, role)
        else {
            return Ok(None);
        };

        Ok(Some(TranslationTarget {
            account_id,
            account_name,
            tenant_id,
            role_id: guest_role_id,
            role_slug,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::{
        federation_link::live_link_schema::{
            live_pool, provider, user_and_peer,
        },
        federation_translation::GrantTranslationWriteSqlDbRepository,
    };

    use diesel::connection::SimpleConnection;
    use myc_core::domain::{
        dtos::{
            federation::{
                IssuerGrantMatch, LocalGrant, NewGrantTranslationRule,
            },
            guest_role::Permission,
        },
        entities::GrantTranslationWrite,
    };

    /// A tenant account, a system account and a guest role: `(account,
    /// system account, role, tenant)`
    fn targets(
        pool: &crate::models::config::DbPool,
    ) -> (Uuid, Uuid, Uuid, Uuid) {
        let ids = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        pool.get()
            .unwrap()
            .batch_execute(&format!(
                "INSERT INTO account (id, tenant_id) VALUES ('{}', '{}');
                 INSERT INTO account (id, tenant_id, is_default) VALUES ('{}', '{}', true);
                 INSERT INTO guest_role (id) VALUES ('{}');",
                ids.0, ids.3, ids.1, ids.3, ids.2
            ))
            .unwrap();
        ids
    }

    #[tokio::test]
    async fn rules_are_listed_applied_while_enabled_and_never_target_a_system_account(
    ) {
        let Some(pool) = live_pool() else { return };
        let provider = provider(pool.clone());
        let read = GrantTranslationReadSqlDbRepository {
            db_config: provider.clone(),
        };
        let write = GrantTranslationWriteSqlDbRepository {
            db_config: provider,
        };
        let (_, peer) = user_and_peer(&pool);
        let (account, system, role, tenant) = targets(&pool);

        assert!(read.target(system, role).await.unwrap().is_none());
        let target = read.target(account, role).await.unwrap().unwrap();
        assert_eq!(
            (target.tenant_id, target.role_slug.as_str()),
            (tenant, "reviewer")
        );

        let rule = write
            .create(
                NewGrantTranslationRule {
                    peer_id: peer,
                    issuer: IssuerGrantMatch {
                        tenant_id: Uuid::new_v4(),
                        role: "editor".to_string(),
                        min_perm: Permission::Read,
                    },
                    local: LocalGrant {
                        account_id: account,
                        guest_role_id: role,
                        perm: Permission::Write,
                    },
                },
                None,
            )
            .await
            .unwrap();

        assert_eq!(read.list(Some(peer)).await.unwrap(), vec![rule.clone()]);
        let active = read.list_active_for_peers(&[peer]).await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].target, target);

        write
            .set_enabled(rule.id, false, None)
            .await
            .unwrap()
            .unwrap();
        assert!(read
            .list_active_for_peers(&[peer])
            .await
            .unwrap()
            .is_empty());
        assert!(write
            .set_enabled(Uuid::new_v4(), true, None)
            .await
            .unwrap()
            .is_none());
        write.delete(rule.id).await.unwrap();
        assert!(read.list(Some(peer)).await.unwrap().is_empty());
    }
}
