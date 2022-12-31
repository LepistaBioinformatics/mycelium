use myc_core::domain::dtos::token::TokenDTO;

pub(super) fn to_redis_key(token: TokenDTO) -> String {
    format!("{}-{}", token.token.to_string(), token.own_service)
}
