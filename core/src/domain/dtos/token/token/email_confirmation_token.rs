// ? ---------------------------------------------------------------------------
// ? EmailConfirmationTokenMeta
//
// Data type used during the email confirmation procedure
//
// ? ---------------------------------------------------------------------------

use crate::domain::dtos::token::UserRelatedMeta;

pub type EmailConfirmationTokenMeta = UserRelatedMeta<String>;
