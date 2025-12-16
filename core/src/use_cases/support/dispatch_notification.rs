use crate::{
    domain::{
        dtos::{
            email::Email,
            message::{FromEmail, Message, MessageSendingEvent},
            tenant::TenantMetaKey,
        },
        entities::{LocalMessageWrite, TenantFetching},
    },
    models::AccountLifeCycle,
    settings::TEMPLATES,
};

use mycelium_base::{
    entities::{CreateResponseKind, FetchResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use tera::Context;
use uuid::Uuid;

#[tracing::instrument(name = "dispatch_notification", skip_all)]
pub(crate) async fn dispatch_notification<T: ToString>(
    parameters: Vec<(T, String)>,
    template_path_prefix: T,
    config: AccountLifeCycle,
    to: Email,
    cc: Option<Email>,
    local_message_write_repo: Box<&dyn LocalMessageWrite>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors> {
    tracing::info!("Dispatching notification");

    let (context, locale) =
        populate_tenant_info(&parameters, &config, tenant_fetching_repo)
            .await?;

    let locale = if let Some(locale) = locale {
        //
        // Use the tenant preferred locale if available
        //
        tracing::trace!("Communicating with tenant locale: {:?}", locale);
        locale
    } else if let Some(locale) = config.locale {
        //
        // Use the account locale if available
        //
        tracing::trace!("Communicating with system locale: {:?}", locale);
        locale.async_get_or_error().await?
    } else {
        //
        // Use the default locale if no locale is available
        //
        tracing::trace!("Communicating with default locale: 'en-us'");
        "en-us".to_string()
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

    local_message_write_repo
        .send(MessageSendingEvent::new(Message {
            from,
            to,
            cc,
            subject: subject_,
            body,
        }))
        .await
}

async fn populate_tenant_info<T: ToString>(
    parameters: &Vec<(T, String)>,
    config: &AccountLifeCycle,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<(Context, Option<String>), MappedErrors> {
    let mut context = Context::new();
    let mut optional_locale = None;

    if let Some((_, tenant_id)) = parameters
        .iter()
        .find(|(key, _)| key.to_string() == "tenant_id")
    {
        if let Ok(tenant_id) = tenant_id.parse::<Uuid>() {
            if let FetchResponseKind::Found(tenant) = tenant_fetching_repo
                .get_tenant_public_by_id(tenant_id)
                .await?
            {
                //
                // Inject the tenant name
                //
                context.insert("domain_name", tenant.name.as_str());

                if let Some(meta) = &tenant.meta {
                    //
                    // Inject the tenant website URL
                    //
                    if let Some(website_url) =
                        meta.get(&TenantMetaKey::WebsiteUrl)
                    {
                        context.insert("domain_url", website_url.as_str());
                    }

                    //
                    // Inject the tenant support email
                    //
                    if let Some(support_email) =
                        meta.get(&TenantMetaKey::SupportEmail)
                    {
                        context.insert("support_email", support_email.as_str());
                    }

                    //
                    // Populate the tenant preferred locale
                    //
                    optional_locale = meta
                        .get(&TenantMetaKey::Locale)
                        .map(|locale| locale.to_owned());
                }
            }
        }
    } else {
        context.insert(
            "domain_name",
            config.domain_name.async_get_or_error().await?.as_str(),
        );

        if let Some(domain_url) = &config.domain_url {
            context.insert(
                "domain_url",
                domain_url.async_get_or_error().await?.as_str(),
            );
        }

        context.insert(
            "support_email",
            &config.support_email.async_get_or_error().await?,
        );
    }

    for (key, value) in parameters {
        context.insert(key.to_string(), &value.to_string());
    }

    Ok((context, optional_locale))
}
