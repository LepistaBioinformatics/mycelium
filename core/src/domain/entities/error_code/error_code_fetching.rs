use crate::domain::dtos::error_code::ErrorCode;

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait ErrorCodeFetching: Interface + Send + Sync {
    async fn get(
        &self,
        prefix: String,
        code: i32,
    ) -> Result<FetchResponseKind<ErrorCode, (String, i32)>, MappedErrors>;

    /// List error codes
    ///
    /// `is_native = None` keeps native rows; `Some(false)` lists custom codes
    /// only.
    async fn list(
        &self,
        prefix: Option<String>,
        code: Option<i32>,
        is_internal: Option<bool>,
        is_native: Option<bool>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<ErrorCode>, MappedErrors>;
}
