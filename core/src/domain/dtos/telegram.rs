use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Canonical Telegram user identifier. Immutable — never changes for a user.
/// The only key used for lookup and linking. Username must never be used.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema,
)]
pub struct TelegramUserId(pub i64);

/// Update identifier sent by Telegram in every webhook delivery.
/// Used for idempotency deduplication in KV store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelegramUpdateId(pub u64);

/// Verified Telegram user identity.
/// `username` is display-only — never used for lookup or linking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TelegramUser {
    pub id: TelegramUserId,
    pub username: Option<String>,
}
