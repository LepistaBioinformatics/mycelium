use crate::sqlite::{
    models::{
        identity_provider::IdentityProvider as IdentityProviderModel,
        user::User as UserModel,
    },
    repositories::account::created_at_from_text,
    types::uuid_from_text,
};

use myc_core::domain::dtos::{
    email::Email,
    user::{MultiFactorAuthentication, PasswordHash, Provider, User},
};
use mycelium_base::{dtos::Parent, utils::errors::MappedErrors};

/// Rebuilds the domain `User` from a `(user, identity_provider)` row pair.
/// Shared by all three `UserFetching` methods, which differ only in whether
/// MFA secrets are redacted (`get_not_redacted_user_by_email` keeps them).
pub(crate) fn map_user_row_to_dto(
    user_record: UserModel,
    provider_record: IdentityProviderModel,
    redact_mfa: bool,
) -> Result<User, MappedErrors> {
    let provider = decode_provider(provider_record)?;

    let mut user = User::new(
        Some(uuid_from_text(&user_record.id).unwrap()),
        user_record.username,
        Email::from_string(user_record.email)?,
        Some(user_record.first_name),
        Some(user_record.last_name),
        user_record.is_active,
        created_at_from_text(&user_record.created),
        user_record.updated.map(|dt| created_at_from_text(&dt)),
        user_record
            .account_id
            .map(|id| Parent::Id(uuid_from_text(&id).unwrap())),
        Some(provider),
    )
    .with_principal(user_record.is_principal);

    let Some(mfa) = user_record.mfa else {
        return Ok(user);
    };

    let mut mfa: MultiFactorAuthentication = serde_json::from_str(&mfa)
        .map_err(|err| {
            mycelium_base::utils::errors::fetching_err(format!(
                "Failed to parse MFA data: {err}"
            ))
        })?;

    if redact_mfa {
        mfa.redact_secrets();
    }

    user = user.with_mfa(mfa);

    Ok(user)
}

fn decode_provider(
    provider_record: IdentityProviderModel,
) -> Result<Provider, MappedErrors> {
    if let Some(password_hash) = provider_record.password_hash {
        return Ok(Provider::Internal(PasswordHash::new_from_hash(
            password_hash,
        )));
    }

    if let Some(name) = provider_record.name {
        return Ok(Provider::External(name));
    }

    mycelium_base::utils::errors::fetching_err(
        "User has invalid provider configuration",
    )
    .as_error()
}
