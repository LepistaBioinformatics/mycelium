use crate::{
    models::{config::DbPoolProvider, message::Message as MessageModel},
    schema::message_queue as message_queue_model,
};

use async_trait::async_trait;
use base64::{engine::general_purpose, Engine};
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        message::{Message, MessageSendingEvent},
        native_error_codes::NativeErrorCodes,
    },
    entities::HealthCheckInfoWrite,
};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use serde::{Deserialize, Serialize};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = HealthCheckInfoWrite)]
pub struct HealthCheckInfoWriteSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl HealthCheckInfoWrite for HealthCheckInfoWriteSqlDbRepository {
    async fn register_health_check_info(&self) -> Result<(), MappedErrors> {
        unimplemented!()
    }
}
