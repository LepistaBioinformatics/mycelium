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
    template_path: T,
    config: AccountLifeCycle,
    to: Email,
    cc: Option<Email>,
    subject: String,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors> {
    let mut context = Context::new();

    context.insert("domain_name", config.domain_name.as_str());
    context.insert("support_email", &config.support_email.get_or_error()?);

    if let Some(domain_url) = config.domain_url {
        context.insert("domain_url", domain_url.as_str());
    }

    for (key, value) in parameters {
        context.insert(key.to_string(), &value.to_string());
    }

    let locale_path = format!(
        "{locale}/{path}",
        locale = config.locale.unwrap_or_else(|| "en-us".to_string()),
        path = template_path.to_string()
    );

    let email_template = match TEMPLATES.render(&locale_path.as_str(), &context)
    {
        Ok(res) => res,
        Err(err) => {
            return use_case_err(format!(
                "Unable to render email template: {err}"
            ))
            .as_error();
        }
    };

    let from_email = Email::from_string(config.noreply_email.get_or_error()?)?;

    let from = if let Some(name) = config.noreply_name {
        FromEmail::NamedEmail(format!("{} <{}>", name, from_email.get_email()))
    } else {
        FromEmail::Email(from_email)
    };

    message_sending_repo
        .send(Message {
            from,
            to,
            cc,
            subject,
            body: email_template,
        })
        .await
}
