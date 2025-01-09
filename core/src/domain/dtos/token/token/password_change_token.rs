// ? ---------------------------------------------------------------------------
// ? PasswordChangeTokenMeta
//
// Data type used during password change procedure
//
// ? ---------------------------------------------------------------------------

use crate::domain::dtos::token::UserRelatedMeta;

pub type PasswordChangeTokenMeta = UserRelatedMeta<String>;
