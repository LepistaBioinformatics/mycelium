use crate::{
    domain::{
        dtos::{
            email::Email,
            native_error_codes::NativeErrorCodes,
            user::{MultiFactorAuthentication, Totp},
        },
        entities::{LocalMessageSending, UserFetching, UserUpdating},
    },
    models::AccountLifeCycle,
    settings::DEFAULT_TOTP_DOMAIN,
    use_cases::support::send_email_notification,
};

use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use rand::Rng;
use totp_rs::{Algorithm, Secret, TOTP};
use tracing::error;

#[tracing::instrument(name = "totp_start_activation", skip_all)]
pub async fn totp_start_activation(
    email: Email,
    with_qr_code: Option<bool>,
    life_cycle_settings: AccountLifeCycle,
    user_fetching_repo: Box<&dyn UserFetching>,
    user_updating_repo: Box<&dyn UserUpdating>,
    message_sending_repo: Box<&dyn LocalMessageSending>,
) -> Result<(Option<String>, Option<String>), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch user from email
    // ? -----------------------------------------------------------------------

    let mut user = match user_fetching_repo
        .get_user_by_email(email.to_owned())
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

    // ? -----------------------------------------------------------------------
    // ? Build totp configs
    // ? -----------------------------------------------------------------------

    let account_email = email.email();
    let issuer = DEFAULT_TOTP_DOMAIN.to_string();

    let mut rng = rand::thread_rng();
    let data_byte: [u8; 21] = rng.gen();
    let base32_string = base32::encode(
        base32::Alphabet::RFC4648 { padding: false },
        &data_byte,
    );

    let totp_gen = match TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        Secret::Encoded(base32_string).to_bytes().unwrap(),
        Some(issuer.to_owned()),
        account_email.to_owned(),
    ) {
        Ok(totp) => totp,
        Err(err) => {
            return use_case_err(format!("Error during TOTP activation: {err}"))
                .as_error()
        }
    };

    let otp_base32 = totp_gen.get_secret_base32();

    // ? -----------------------------------------------------------------------
    // ? Update user and persist changes in datastore
    // ? -----------------------------------------------------------------------

    let totp = Totp::Enabled {
        verified: false,
        issuer: issuer.to_owned(),
        secret: Some(otp_base32),
    }
    .encrypt_me(life_cycle_settings.to_owned())
    .await?;

    user.with_mfa(MultiFactorAuthentication {
        totp: totp.to_owned(),
    });

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
        "email/mfa-activation-start",
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

    let qr_code = if let Some(true) = with_qr_code {
        let qr_code = match totp_gen.get_qr_base64() {
            Ok(qr_code) => qr_code,
            Err(err) => {
                error!("Error during TOTP activation: {err}");

                "Error during TOTP activation".to_string()
            }
        };

        Some(qr_code)
    } else {
        None
    };

    Ok((
        Some(totp.build_auth_url(email, life_cycle_settings).await?),
        qr_code,
    ))
}
