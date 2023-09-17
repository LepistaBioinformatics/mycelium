//
// https://dev.to/sirneij/authentication-system-using-rust-actix-web-and-sveltekit-user-registration-580h
//

use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use clean_base::utils::errors::{factories::use_case_err, MappedErrors};

pub async fn verify_password(
    hash: &str,
    password: &[u8],
) -> Result<(), MappedErrors> {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(hash) => hash,
        Err(err) => {
            return use_case_err(format!(
                "Unable to parse password hash: {err}",
            ))
            .as_error()
        }
    };

    match Argon2::default().verify_password(password, &parsed_hash) {
        Ok(_) => Ok(()),
        Err(err) => {
            use_case_err(format!("Unable to verify password: {err}")).as_error()
        }
    }
}
