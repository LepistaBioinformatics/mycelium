use crate::domain::dtos::route::Route;

use actix_web::http::uri::PathAndQuery;
use async_trait::async_trait;
use clean_base::{
    entities::default_response::FetchManyResponseKind,
    utils::errors::MappedErrors,
};
use shaku::Interface;
use std::fmt::Result as FmResult;
use std::fmt::{Debug, Display, Formatter};

#[async_trait]
pub trait RoutesFetching: Interface + Send + Sync {
    async fn list(
        &self,
        path: PathAndQuery,
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
