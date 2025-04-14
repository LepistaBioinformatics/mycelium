use async_trait::async_trait;
use myc_core::domain::entities::ServiceWrite;
use shaku::Component;

#[derive(Component)]
#[shaku(interface = ServiceWrite)]
pub struct ServiceWriteMemDbRepo {}

#[async_trait]
impl ServiceWrite for ServiceWriteMemDbRepo {}
