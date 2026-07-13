use crate::domain::dtos::instance_settings::InstanceSetting;

use async_trait::async_trait;
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use shaku::Interface;

#[async_trait]
pub trait InstanceSettingsFetching: Interface + Send + Sync {
    async fn get(
        &self,
        key: String,
    ) -> Result<FetchResponseKind<InstanceSetting, ()>, MappedErrors>;
}
