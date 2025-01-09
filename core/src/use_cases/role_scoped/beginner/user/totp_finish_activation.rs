use crate::{
    domain::{
        dtos::{
            email::Email,
            native_error_codes::NativeErrorCodes,
            user::{MultiFactorAuthentication, Totp},
        },
        entities::{MessageSending, UserFetching, UserUpdating},
    },
    models::AccountLifeCycle,
    settings::DEFAULT_TOTP_DOMAIN,
    use_cases::support::send_email_notification,
};

use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use totp_rs::{Algorithm, Secret, TOTP};

#[tracing::instrument(name = "totp_finish_activation", skip_all)]
pub async fn totp_finish_activation(
    email: Email,
    token: String,
    life_cycle_settings: AccountLifeCycle,
    user_fetching_repo: Box<&dyn UserFetching>,
    user_updating_repo: Box<&dyn UserUpdating>,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch user from email
    // ? -----------------------------------------------------------------------

    let mut user = match user_fetching_repo
        .get_not_redacted_user_by_email(email.to_owned())
        .await?
    {
        FetchResponseKind::NotFound(_) => {
            return use_case_err(format!(
                "User not already registered: {}",
                email.email()
            ))
            .with_code(NativeErrorCodes::MYC00009)
            .with_exp_true()
            .as_error()
        }
        FetchResponseKind::Found(user) => user,
    };

    if let Totp::Enabled { verified, .. } = user.mfa().totp {
        if verified {
            return use_case_err(format!(
                "User already has TOTP enabled: {}",
                email.email()
            ))
            .with_code(NativeErrorCodes::MYC00021)
            .with_exp_true()
            .as_error();
        }
    }

    if let Totp::Disabled = user.mfa().totp {
        return use_case_err(format!(
            "User does not have TOTP enabled: {}",
            email.email()
        ))
        .with_code(NativeErrorCodes::MYC00022)
        .with_exp_true()
        .as_error();
    }

    let encrypted_user_totp = user.mfa().totp.clone();

    let decrypted_user_totp =
        encrypted_user_totp.decrypt_me(life_cycle_settings.to_owned())?;

    let user_secret_option = match decrypted_user_totp {
        Totp::Enabled { secret, .. } => secret,
        _ => {
            return use_case_err(format!(
                "User does not have TOTP enabled: {}",
                email.email()
            ))
            .with_code(NativeErrorCodes::MYC00022)
            .with_exp_true()
            .as_error();
        }
    };

    let user_secret = match user_secret_option {
        Some(secret) => secret,
        None => {
            return use_case_err(format!(
                "User does not have TOTP correctly configured: {}",
                email.email()
            ))
            .with_code(NativeErrorCodes::MYC00022)
            .with_exp_true()
            .as_error();
        }
    };

    let account_email = email.email();
    let issuer = DEFAULT_TOTP_DOMAIN.to_string();

    let totp = match TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        Secret::Encoded(user_secret).to_bytes().unwrap(),
        Some(issuer.to_owned()),
        account_email.to_owned(),
    ) {
        Ok(totp) => totp,
        Err(err) => {
            return use_case_err(format!("Error during TOTP activation: {err}"))
                .as_error()
        }
    };

    let is_valid = match totp.check_current(&token) {
        Ok(is_valid) => is_valid,
        Err(err) => {
            return use_case_err(format!("Error during TOTP activation: {err}"))
                .as_error()
        }
    };

    if !is_valid {
        return use_case_err(format!("Invalid TOTP token: {}", email.email()))
            .with_code(NativeErrorCodes::MYC00023)
            .with_exp_true()
            .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Update user and persist changes in datastore
    // ? -----------------------------------------------------------------------

    match encrypted_user_totp {
        Totp::Enabled { issuer, secret, .. } => {
            user.with_mfa(MultiFactorAuthentication {
                totp: Totp::Enabled {
                    verified: true,
                    issuer,
                    secret,
                },
            });
        }
        _ => {
            return use_case_err(format!(
                "User does not have TOTP correctly configured: {}",
                email.email()
            ))
            .with_code(NativeErrorCodes::MYC00022)
            .with_exp_true()
            .as_error();
        }
    }

    let user_id = match user.id {
        Some(id) => id,
        None => {
            return use_case_err(format!(
                "Unexpected error: User with email {email} has no id",
                email = email.email()
            ))
            .as_error()
        }
    };

    user_updating_repo.update_mfa(user_id, user.mfa()).await?;

    // ? -----------------------------------------------------------------------
    // ? Inform user about TOTP activation
    // ? -----------------------------------------------------------------------

    if let Err(err) = send_email_notification(
        vec![],
        "email/mfa-activation-validated",
        life_cycle_settings.to_owned(),
        email.to_owned(),
        None,
        message_sending_repo,
    )
    .await
    {
        return use_case_err(format!("Unable to send email: {err}"))
            .with_code(NativeErrorCodes::MYC00010)
            .as_error();
    };

    Ok(())
}
