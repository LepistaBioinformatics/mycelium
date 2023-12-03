//
// https://dev.to/sirneij/authentication-system-using-rust-actix-web-and-sveltekit-user-registration-580h
//

mod check_email_password_validity;
mod check_email_registration_status;
mod check_token_and_activate_user;
mod create_default_user;
mod issue_confirmation_token_pasetor;
mod notify_internal_user;
mod verify_confirmation_token_pasetor;

pub use check_email_password_validity::*;
pub use check_email_registration_status::*;
pub use check_token_and_activate_user::*;
pub use create_default_user::*;
pub use verify_confirmation_token_pasetor::*;
