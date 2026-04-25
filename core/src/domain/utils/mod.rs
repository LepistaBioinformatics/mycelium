mod derive_key_from_uuid;
pub mod encrypt_string;
pub mod envelope;
mod try_as_uuid;

pub(crate) use derive_key_from_uuid::*;
pub use encrypt_string::{
    decrypt_string, decrypt_string_with_dek, encrypt_string,
};
pub use envelope::{
    build_aad, decrypt_with_dek, encrypt_with_dek, generate_dek, unwrap_dek,
    wrap_dek, AAD_FIELD_HTTP_SECRET, AAD_FIELD_TELEGRAM_BOT_TOKEN,
    AAD_FIELD_TELEGRAM_WEBHOOK_SECRET, AAD_FIELD_TOTP_SECRET, SYSTEM_TENANT_ID,
};
pub use try_as_uuid::*;
