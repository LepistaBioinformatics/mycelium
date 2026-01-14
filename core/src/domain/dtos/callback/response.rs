use super::{CallbackContext, CallbackError};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait CallbackResponse: Interface + Send + Sync {
    async fn execute(
        &self,
        context: &CallbackContext,
    ) -> Result<(), CallbackError>;

    fn name(&self) -> &str;
}
