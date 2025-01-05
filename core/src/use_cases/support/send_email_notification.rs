use crate::{
    domain::{
        dtos::{
            email::Email,
            message::{FromEmail, Message},
        },
        entities::MessageSending,
    },
    models::AccountLifeCycle,
    settings::TEMPLATES,
};

use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use tera::Context;
use uuid::Uuid;

#[tracing::instrument(name = "send_email_notification", skip_all)]
pub(crate) async fn send_email_notification<T: ToString>(
    parameters: Vec<(T, String)>,
    template_path_prefix: T,
    config: AccountLifeCycle,
    to: Email,
    cc: Option<Email>,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors> {
    let mut context = Context::new();

    context.insert(
        "domain_name",
        config.domain_name.async_get_or_error().await?.as_str(),
    );
    context.insert(
        "support_email",
        &config.support_email.async_get_or_error().await?,
    );

    if let Some(domain_url) = config.domain_url {
        context.insert(
            "domain_url",
            domain_url.async_get_or_error().await?.as_str(),
        );
    }

    for (key, value) in parameters {
        context.insert(key.to_string(), &value.to_string());
    }

    let locale = match config.locale {
        Some(locale) => locale.async_get_or_error().await?,
        None => "en-us".to_string(),
    };

    let body_path = format!(
        "{locale}/{path}",
        locale = locale,
        path = format!(
            "{prefix}.jinja",
            prefix = template_path_prefix.to_string()
        )
    );

    let body = match TEMPLATES.render(&body_path.as_str(), &context) {
        Ok(res) => res,
        Err(err) => {
            return use_case_err(format!(
                "Unable to render email template: {err}"
            ))
            .as_error();
        }
    };

    let subject_path = format!(
        "{locale}/{path}",
        locale = locale,
        path = format!(
            "{prefix}.subject",
            prefix = template_path_prefix.to_string()
        )
    );

    let subject_ =
        match TEMPLATES.render(&subject_path.as_str(), &Context::new()) {
            Ok(res) => res,
            Err(err) => {
                return use_case_err(format!(
                    "Unable to render email subject: {err}"
                ))
                .as_error();
            }
        };

    let from_email =
        Email::from_string(config.noreply_email.async_get_or_error().await?)?;

    let from = if let Some(name) = config.noreply_name {
        FromEmail::NamedEmail(format!(
            "{} <{}>",
            name.async_get_or_error().await?,
            from_email.email()
        ))
    } else {
        FromEmail::Email(from_email)
    };

    message_sending_repo
        .send(Message {
            from,
            to,
            cc,
            subject: format!(
                "[{}] {}",
                config.domain_name.async_get_or_error().await?,
                subject_
            ),
            body,
        })
        .await
}
