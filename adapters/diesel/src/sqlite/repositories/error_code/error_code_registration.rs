use super::shared::map_model_to_dto;
use crate::sqlite::{
    config::SqliteDbPoolProvider,
    models::error_code::ErrorCode as ErrorCodeModel, schema::error_code,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{error_code::ErrorCode, native_error_codes::NativeErrorCodes},
    entities::ErrorCodeRegistration,
};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = ErrorCodeRegistration)]
pub struct ErrorCodeRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl ErrorCodeRegistration for ErrorCodeRegistrationSqlDbRepository {
    #[tracing::instrument(name = "create_error_code", skip_all)]
    async fn create(
        &self,
        error_code_dto: ErrorCode,
    ) -> Result<CreateResponseKind<ErrorCode>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Check if error code already exists
        let existing = error_code::table
            .filter(error_code::prefix.eq(&error_code_dto.prefix))
            .filter(error_code::code.eq(error_code_dto.error_number))
            .select(ErrorCodeModel::as_select())
            .first::<ErrorCodeModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!(
                    "Failed to check existing error code: {}",
                    e
                ))
            })?;

        if let Some(record) = existing {
            return Ok(CreateResponseKind::NotCreated(
                map_model_to_dto(record),
                "Error code already exists".to_string(),
            ));
        }

        // Create new error code. `code` has no server-side default in the
        // SQLite migration (postgres uses SERIAL), but the domain always
        // supplies `error_number` explicitly here, matching postgres exactly.
        let new_error = ErrorCodeModel {
            code: error_code_dto.error_number,
            prefix: error_code_dto.prefix,
            message: error_code_dto.message,
            details: error_code_dto.details,
            is_internal: error_code_dto.is_internal,
            is_native: error_code_dto.is_native,
        };

        let created = diesel::insert_into(error_code::table)
            .values(&new_error)
            .returning(ErrorCodeModel::as_returning())
            .get_result::<ErrorCodeModel>(conn)
            .map_err(|e| {
                creation_err(format!("Failed to create error code: {}", e))
            })?;

        Ok(CreateResponseKind::Created(map_model_to_dto(created)))
    }
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::{
        repositories::error_code::{
            ErrorCodeDeletionSqlDbRepository, ErrorCodeFetchingSqlDbRepository,
            ErrorCodeUpdatingSqlDbRepository,
        },
        test_support::setup_temp_db,
    };
    use myc_core::domain::entities::{
        ErrorCodeDeletion, ErrorCodeFetching, ErrorCodeUpdating,
    };
    use mycelium_base::entities::{
        DeletionResponseKind, FetchResponseKind, UpdatingResponseKind,
    };

    fn new_error_code(prefix: &str, number: i32, message: &str) -> ErrorCode {
        ErrorCode {
            prefix: prefix.into(),
            error_number: number,
            code: None,
            message: message.into(),
            details: None,
            is_internal: false,
            is_native: false,
        }
    }

    #[tokio::test]
    async fn error_code_lifecycle_round_trips_through_sqlite(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let registration = ErrorCodeRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let fetching = ErrorCodeFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let updating = ErrorCodeUpdatingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let deletion = ErrorCodeDeletionSqlDbRepository {
            db_config: db.provider.clone(),
        };

        // Create
        let created = match registration
            .create(new_error_code("MYC", 1, "Something failed"))
            .await?
        {
            CreateResponseKind::Created(err) => err,
            CreateResponseKind::NotCreated(..) => {
                panic!("expected the error code to be created")
            }
        };
        assert_eq!(created.error_number, 1);

        // Duplicate is a no-op
        let not_created = registration
            .create(new_error_code("MYC", 1, "Different message"))
            .await?;
        assert!(matches!(not_created, CreateResponseKind::NotCreated(..)));

        // Fetch
        let found = match fetching.get("MYC".into(), 1).await? {
            FetchResponseKind::Found(err) => err,
            FetchResponseKind::NotFound(_) => {
                panic!("expected the error code to be found")
            }
        };
        assert_eq!(found.message, "Something failed");

        // Update
        let mut update_payload = found.clone();
        update_payload.message = "Updated message".into();
        let updated = match updating.update(update_payload).await? {
            UpdatingResponseKind::Updated(err) => err,
            UpdatingResponseKind::NotUpdated(..) => {
                panic!("expected the error code to be updated")
            }
        };
        assert_eq!(updated.message, "Updated message");

        // List
        let listed = fetching.list(None, None, None, None, None).await?;
        match listed {
            mycelium_base::entities::FetchManyResponseKind::FoundPaginated {
                count,
                records,
                ..
            } => {
                assert_eq!(count, 1);
                assert_eq!(records.len(), 1);
            }
            _ => panic!("expected paginated error codes"),
        }

        // Delete
        let deleted = deletion.delete("MYC".into(), 1).await?;
        assert!(matches!(deleted, DeletionResponseKind::Deleted));

        let after_delete = fetching.get("MYC".into(), 1).await?;
        assert!(matches!(after_delete, FetchResponseKind::NotFound(_)));

        Ok(())
    }
}
