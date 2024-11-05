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

    for (key, value) in parameters {
        context.insert(key.to_string(), &value.to_string());
    }

    let email_template =
        match TEMPLATES.render(&template_path.to_string().as_str(), &context) {
            Ok(res) => res,
            Err(err) => {
                return use_case_err(format!(
                    "Unable to render email template: {err}"
                ))
                .as_error();
            }
        };

    let from = Email::from_string(config.noreply_email.get_or_error()?)?;

    message_sending_repo
        .send(Message {
            from: FromEmail::NamedEmail(format!(
                "Sou Agrobiota <{}>",
                from.get_email()
            )),
            to,
            cc,
            subject,
            body: email_template,
        })
        .await
}
