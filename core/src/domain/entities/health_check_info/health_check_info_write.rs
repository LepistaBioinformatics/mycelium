use async_trait::async_trait;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;
use std::fmt::Result as FmResult;
use std::fmt::{Debug, Display, Formatter};

#[async_trait]
pub trait HealthCheckInfoWrite: Interface + Send + Sync {
    async fn register_health_check_info(&self) -> Result<(), MappedErrors>;
}

impl Display for dyn HealthCheckInfoWrite {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}

impl Debug for dyn HealthCheckInfoWrite {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}
