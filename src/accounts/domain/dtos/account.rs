use super::{
    enums::{ChildrenEnum, ParentEnum},
    guest::GuestUserDTO,
    user::UserDTO,
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountTypeDTO {
    pub id: Uuid,

    pub name: String,
    pub description: String,

    /// Inspector accounts allows their guests to walking through specific
    /// accounts aiming to verify records irregularities. Such users can't edit
    /// records, but only relate possible irregularities outside of the scope of
    /// their own accounts.
    pub is_inspector: Option<bool>,

    /// Editor accounts allow their guest users to walking through specific
    /// accounts aiming to verify records irregularities and perform editions if
    /// necessary. Such users can't delete records.
    pub is_editor: Option<bool>,

    /// Manager accounts allow their guest users to walking through specific
    /// accounts aiming to verify records irregularities and perform editions
    /// and deletions if necessary.
    pub is_manager: Option<bool>,

    /// Superuser accounts allow their guest users to walking through specific
    /// accounts aiming to verify records irregularities and perform editions
    /// and deletions if necessary. Such users can perform mass deletions.
    pub is_superuser: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDTO {
    pub id: Uuid,

    pub owner: ParentEnum<Uuid, UserDTO>,
    pub account_type: ParentEnum<Uuid, AccountTypeDTO>,
    pub guest_users: ChildrenEnum<Uuid, GuestUserDTO>,
}
