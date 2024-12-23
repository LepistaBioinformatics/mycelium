use crate::domain::dtos::route::Route;

use actix_web::http::uri::PathAndQuery;
use async_trait::async_trait;
use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use std::fmt::Result as FmResult;
use std::fmt::{Debug, Display, Formatter};
use uuid::Uuid;

#[async_trait]
pub trait RoutesFetching: Interface + Send + Sync {
    async fn get(
        &self,
        path: PathAndQuery,
    ) -> Result<FetchManyResponseKind<Route>, MappedErrors>;

    async fn list_by_service(
        &self,
        id: Option<Uuid>,
        name: Option<String>,
        include_service_details: Option<bool>,
    ) -> Result<FetchManyResponseKind<Route>, MappedErrors>;
}

impl Display for dyn RoutesFetching {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}

impl Debug for dyn RoutesFetching {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}
