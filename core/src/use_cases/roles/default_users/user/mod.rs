//
// https://dev.to/sirneij/authentication-system-using-rust-actix-web-and-sveltekit-user-registration-580h
//

mod check_email_registration_status;
mod issue_confirmation_token_pasetor;
mod verify_confirmation_token_pasetor;

pub use check_email_registration_status::*;
pub use issue_confirmation_token_pasetor::*;
pub use verify_confirmation_token_pasetor::*;
