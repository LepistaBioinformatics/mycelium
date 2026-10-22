use super::hosted_login_session_record::find_active_session;
use crate::config::SqliteDbPoolProvider;

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{
        hosted_login::HostedLoginSession, native_error_codes::NativeErrorCodes,
    },
    entities::HostedLoginSessionFetching,
};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = HostedLoginSessionFetching)]
pub struct HostedLoginSessionFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl HostedLoginSessionFetching for HostedLoginSessionFetchingSqlDbRepository {
    #[tracing::instrument(name = "get_hosted_login_session", skip_all)]
    async fn get_by_session_id(
        &self,
        session_id: &str,
    ) -> Result<FetchResponseKind<HostedLoginSession, String>, MappedErrors>
    {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        match find_active_session(conn, session_id) {
            Ok(Some((_, session))) => Ok(FetchResponseKind::Found(session)),
            Ok(None) => Ok(FetchResponseKind::NotFound(None)),
            Err(e) => fetching_err(format!(
                "Unexpected error on fetching hosted login session: {}",
                e
            ))
            .as_error(),
        }
    }
}
