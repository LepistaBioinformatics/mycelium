//
// https://dev.to/sirneij/authentication-system-using-rust-actix-web-and-sveltekit-user-registration-580h
//

mod issue_confirmation_token_pasetor;
mod verify_confirmation_token_pasetor;

pub use issue_confirmation_token_pasetor::*;
pub use verify_confirmation_token_pasetor::*;
