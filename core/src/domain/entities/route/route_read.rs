use crate::domain::dtos::route::Route;

use async_trait::async_trait;
use http::uri::PathAndQuery;
use mycelium_base::entities::FetchResponseKind;
use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait RoutesRead: Interface + Send + Sync {
    async fn match_single_path_or_error(
        &self,
        path: PathAndQuery,
    ) -> Result<FetchResponseKind<Route, String>, MappedErrors>;

    async fn list_routes_paginated(
        &self,
        id: Option<Uuid>,
        name: Option<String>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<Route>, MappedErrors>;

    async fn list_routes(
        &self,
        id: Option<Uuid>,
        name: Option<String>,
    ) -> Result<FetchManyResponseKind<Route>, MappedErrors>;
}
