use async_trait::async_trait;
use chrono::{DateTime, Local};
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;
use uuid::Uuid;

/// Single use of a signed peer request's `jti`, shared by every pod (D-9)
#[async_trait]
pub trait RequestNonceRegistration: Interface + Send + Sync {
    /// `true` when this call is the first to see `jti`; `false` on a replay
    async fn claim(
        &self,
        jti: String,
        peer_id: Uuid,
        expires_at: DateTime<Local>,
    ) -> Result<bool, MappedErrors>;

    /// Delete the nonces past `expires_at`; returns how many were deleted
    async fn purge_expired(&self) -> Result<u64, MappedErrors>;
}
