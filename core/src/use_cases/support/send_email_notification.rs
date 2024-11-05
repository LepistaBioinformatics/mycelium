use crate::{
    domain::{
        dtos::{email::Email, message::Message},
        entities::MessageSending,
    },
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
    from: Email,
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
