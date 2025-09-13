use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

#[derive(
    Clone, Debug, Deserialize, Serialize, Eq, PartialEq, ToSchema, ToResponse,
)]
#[serde(rename_all = "camelCase")]
pub enum IDSource {
    /// The ID source is the user ID
    User,

    /// The ID source is the system actor
    Account,
}

#[derive(
    Clone, Debug, Deserialize, Serialize, Eq, PartialEq, ToSchema, ToResponse,
)]
#[serde(rename_all = "camelCase")]
pub struct WrittenBy {
    /// The ID of the user who created the account
    pub id: Uuid,

    /// The ID source
    pub from: IDSource,
}

impl WrittenBy {
    fn new(id: Uuid, from: IDSource) -> Self {
        Self { id, from }
    }

    pub fn new_from_user(id: Uuid) -> Self {
        Self::new(id, IDSource::User)
    }

    pub fn new_from_account(id: Uuid) -> Self {
        Self::new(id, IDSource::Account)
    }

    /// Create a new updated by with no ID
    pub fn new_anemic() -> Self {
        Self::new(Uuid::nil(), IDSource::Account)
    }
}

impl Default for WrittenBy {
    fn default() -> Self {
        Self::new_anemic()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let updated_by = WrittenBy::default();
        println!("updated_by: {:?}", updated_by);
        assert_eq!(updated_by.id, Uuid::nil());
        assert_eq!(updated_by.from, IDSource::Account);
    }
}
