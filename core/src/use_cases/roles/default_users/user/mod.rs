//
// https://dev.to/sirneij/authentication-system-using-rust-actix-web-and-sveltekit-user-registration-580h
//

mod check_email_registration_status;
mod create_default_user;
mod issue_confirmation_token_pasetor;
mod verify_confirmation_token_pasetor;

pub use check_email_registration_status::*;
pub use create_default_user::*;
pub use verify_confirmation_token_pasetor::*;
