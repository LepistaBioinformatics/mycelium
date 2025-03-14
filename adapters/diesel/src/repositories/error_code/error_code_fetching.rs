use crate::{
    models::{config::DbPoolProvider, error_code::ErrorCode as ErrorCodeModel},
    schema::error_code as error_code_model,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{error_code::ErrorCode, native_error_codes::NativeErrorCodes},
    entities::ErrorCodeFetching,
};
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = ErrorCodeFetching)]
pub struct ErrorCodeFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl ErrorCodeFetching for ErrorCodeFetchingSqlDbRepository {
    #[tracing::instrument(name = "get_error_code", skip_all)]
    async fn get(
        &self,
        prefix: String,
        code: i32,
    ) -> Result<FetchResponseKind<ErrorCode, (String, i32)>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let error_code = error_code_model::table
            .filter(
                error_code_model::prefix
                    .eq(&prefix)
                    .and(error_code_model::code.eq(code)),
            )
            .select(ErrorCodeModel::as_select())
            .first::<ErrorCodeModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch error code: {}", e))
            })?;

        match error_code {
            Some(record) => {
                Ok(FetchResponseKind::Found(self.map_model_to_dto(record)))
            }
            None => Ok(FetchResponseKind::NotFound(Some((prefix, code)))),
        }
    }

    #[tracing::instrument(name = "list_error_codes", skip_all)]
    async fn list(
        &self,
        prefix: Option<String>,
        code: Option<i32>,
        is_internal: Option<bool>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<ErrorCode>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut records_query = error_code_model::table.into_boxed();
        let mut total_query = error_code_model::table.into_boxed();

        if let Some(prefix) = prefix {
            let stm = error_code_model::prefix.ilike(format!("%{}%", prefix));
            records_query = records_query.filter(stm.to_owned());
            total_query = total_query.filter(stm);
        }

        if let Some(code) = code {
            let stm = error_code_model::code.eq(code);
            records_query = records_query.filter(stm);
            total_query = total_query.filter(stm);
        }

        if let Some(is_internal) = is_internal {
            let stm = error_code_model::is_internal.eq(is_internal);
            records_query = records_query.filter(stm);
            total_query = total_query.filter(stm);
        }

        // Get total count
        let total =
            total_query.count().get_result::<i64>(conn).map_err(|e| {
                fetching_err(format!("Failed to get total count: {}", e))
            })?;

        // Apply pagination
        let page_size = i64::from(page_size.unwrap_or(10));
        let skip = i64::from(skip.unwrap_or(0));

        let records = records_query
            .offset(skip)
            .limit(page_size)
            .select(ErrorCodeModel::as_select())
            .order(error_code_model::code.asc())
            .order(error_code_model::prefix.asc())
            .load::<ErrorCodeModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch error codes: {}", e))
            })?;

        Ok(FetchManyResponseKind::FoundPaginated {
            count: total,
            skip: Some(skip),
            size: Some(page_size),
            records: records
                .into_iter()
                .map(|r| self.map_model_to_dto(r))
                .collect(),
        })
    }
}

impl ErrorCodeFetchingSqlDbRepository {
    fn map_model_to_dto(&self, model: ErrorCodeModel) -> ErrorCode {
        ErrorCode {
            prefix: model.prefix,
            error_number: model.code,
            code: None,
            message: model.message,
            details: model.details,
            is_internal: model.is_internal,
            is_native: model.is_native,
        }
    }
}
