use crate::domain::dtos::webhook::{HookTarget, WebHook};

use async_trait::async_trait;
use clean_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait WebHookFetching: Interface + Send + Sync {
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<WebHook, Uuid>, MappedErrors>;
    async fn list(
        &self,
        name: Option<String>,
        target: Option<HookTarget>,
    ) -> Result<FetchManyResponseKind<WebHook>, MappedErrors>;
}
