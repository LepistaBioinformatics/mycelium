use crate::dtos::claims::Claims;

use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use jsonwebtoken::{
    decode, errors::Error, Algorithm, DecodingKey, TokenData, Validation,
};

pub fn decode_jwt_hs512(
    auth: Authorization<Bearer>,
    jwt_token: String,
) -> Result<TokenData<Claims>, Error> {
    decode::<Claims>(
        &auth.into_scheme().token().to_string(),
        &DecodingKey::from_secret(jwt_token.as_bytes()),
        &Validation::new(Algorithm::HS512),
    )
}
