use async_trait::async_trait;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;
use std::fmt::Result as FmResult;
use std::fmt::{Debug, Display, Formatter};
use uuid::Uuid;

use crate::domain::dtos::health_check_info::HealthStatus;

#[async_trait]
pub trait ServiceWrite: Interface + Send + Sync {
    async fn inform_health_status(
        &self,
        id: Uuid,
        name: String,
        health_status: HealthStatus,
    ) -> Result<(), MappedErrors>;
}

impl Display for dyn ServiceWrite {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}

impl Debug for dyn ServiceWrite {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}
