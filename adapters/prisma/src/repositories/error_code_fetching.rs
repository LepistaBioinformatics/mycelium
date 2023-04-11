use crate::{
    prisma::error_code as error_code_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use clean_base::{
    dtos::PaginatedRecord,
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{factories::fetching_err, MappedErrors},
};
use myc_core::domain::{
    dtos::error_code::ErrorCode, entities::ErrorCodeFetching,
};
use prisma_client_rust::Direction;
use shaku::Component;
use std::process::id as process_id;

#[derive(Component, Debug)]
#[shaku(interface = ErrorCodeFetching)]
pub struct ErrorCodeFetchingSqlDbRepository {}

#[async_trait]
impl ErrorCodeFetching for ErrorCodeFetchingSqlDbRepository {
    async fn get(
        &self,
        prefix: String,
        code: i32,
    ) -> Result<FetchResponseKind<ErrorCode, (String, i32)>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Build and execute the database query
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return fetching_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code("MYC00001".to_string())
                .as_error()
            }
            Some(res) => res,
        };

        let response = client
            .error_code()
            .find_unique(error_code_model::prefix_code(
                prefix.to_owned(),
                code.to_owned(),
            ))
            .exec()
            .await
            .unwrap();

        // ? -------------------------------------------------------------------
        // ? Evaluate and parse the database response
        // ? -------------------------------------------------------------------

        match response {
            Some(record) => Ok(FetchResponseKind::Found(ErrorCode {
                prefix: record.prefix,
                code: record.code,
                message: record.message,
                details: record.details,
                is_internal: record.is_internal,
            })),
            None => Ok(FetchResponseKind::NotFound(Some((prefix, code)))),
        }
    }

    async fn list(
        &self,
        prefix: Option<String>,
        code: Option<i32>,
        is_internal: Option<bool>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<ErrorCode>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return fetching_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code("MYC00001".to_string())
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build list query statement
        // ? -------------------------------------------------------------------

        let page_size = page_size.unwrap_or(10);
        let skip = skip.unwrap_or(0);
        let mut query_stmt = vec![];

        if prefix.is_some() {
            query_stmt.push(error_code_model::prefix::contains(prefix.unwrap()))
        }

        if code.is_some() {
            query_stmt.push(error_code_model::code::equals(code.unwrap()))
        }

        if is_internal.is_some() {
            query_stmt.push(error_code_model::is_internal::equals(
                is_internal.unwrap(),
            ))
        }

        // ? -------------------------------------------------------------------
        // ? Get the user
        // ? -------------------------------------------------------------------

        let (count, response) = match client
            ._batch((
                client.error_code().count(query_stmt.to_owned()),
                client
                    .error_code()
                    .find_many(query_stmt)
                    .skip(skip.into())
                    .take(page_size.into())
                    .order_by(error_code_model::code::order(Direction::Desc)),
            ))
            .await
        {
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on fetch error codes: {err}",
                ))
                .as_error()
            }
            Ok(res) => res,
        };

        if response.len() == 0 {
            return Ok(FetchManyResponseKind::NotFound);
        }

        let records: Vec<ErrorCode> = response
            .into_iter()
            .map(|record| ErrorCode {
                prefix: record.prefix,
                code: record.code,
                message: record.message,
                details: record.details,
                is_internal: record.is_internal,
            })
            .collect::<Vec<ErrorCode>>();

        Ok(FetchManyResponseKind::FoundPaginated(PaginatedRecord {
            count,
            skip: Some(skip.into()),
            size: Some(page_size.into()),
            records,
        }))
    }
}
