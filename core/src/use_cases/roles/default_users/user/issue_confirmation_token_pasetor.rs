use crate::domain::entities::{
    ConfirmationTokenRegistration, ConfirmationTokenUpdating,
};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use hex;
use pasetors::claims::{Claims, ClaimsValidationRules};

use uuid::Uuid;

pub async fn issue_confirmation_token_pasetor(
    user_id: Uuid,
    is_for_password_change: Option<bool>,
    token_registration_repo: Box<&dyn ConfirmationTokenRegistration>,
    token_updating_repo: Box<&dyn ConfirmationTokenUpdating>,
) {
    // I just generate 128 bytes of random data for the session key
    // from something that is cryptographically secure (rand::CryptoRng)
    //
    // You don't necessarily need a random value, but you'll want something
    // that is sufficiently not able to be guessed (you don't want someone getting
    // an old token that is supposed to not be live, and being able to get a valid
    // token from that).
    let session_key: String = {
        let mut buff = [0_u8; 128];
        OsRng.fill_bytes(&mut buff);
        hex::encode(buff)
    };
}
