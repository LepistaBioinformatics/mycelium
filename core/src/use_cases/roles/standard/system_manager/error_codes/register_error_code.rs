use crate::domain::{
    actors::ActorName,
    dtos::{error_code::ErrorCode, profile::Profile},
    entities::ErrorCodeRegistration,
};

use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};

/// Register a new error code
///
/// This action should be only performed by manager or staff users.
#[tracing::instrument(
    name = "register_error_code",
    skip(profile, message, details, error_code_registration_repo)
)]
pub async fn register_error_code(
    profile: Profile,
    prefix: String,
    message: String,
    details: Option<String>,
    is_internal: bool,
    error_code_registration_repo: Box<&dyn ErrorCodeRegistration>,
) -> Result<ErrorCode, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.get_default_write_ids_or_error(vec![
        ActorName::SystemManager.to_string()
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Build error code object
    // ? -----------------------------------------------------------------------

    let mut error_code = match is_internal {
        true => ErrorCode::new_internal_error(prefix, 0, message, false)?,
        false => ErrorCode::new_external_error(prefix, 0, message, false)?,
    };

    if let Some(msg) = details {
        error_code = error_code.to_owned().with_details(msg);
    }

    // ? -----------------------------------------------------------------------
    // ? Register error code
    // ? -----------------------------------------------------------------------

    match error_code_registration_repo.create(error_code).await? {
        CreateResponseKind::Created(error_code) => Ok(error_code),
        CreateResponseKind::NotCreated(_, msg) => {
            return use_case_err(msg).as_error()
        }
    }
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::register_error_code;
    use crate::domain::{
        dtos::{
            error_code::ErrorCode,
            profile::{Owner, Profile},
        },
        entities::ErrorCodeRegistration,
    };

    use async_trait::async_trait;
    use mycelium_base::{
        entities::CreateResponseKind,
        utils::errors::{use_case_err, MappedErrors},
    };
    use shaku::Component;
    use std::str::FromStr;
    use uuid::Uuid;

    // ? -----------------------------------------------------------------------
    // ? Mock repositories
    // ? -----------------------------------------------------------------------

    #[derive(Component)]
    #[shaku(interface = ErrorCodeRegistration)]
    struct MockErrorCodeRegistrationRepo {
        pub generate_error: bool,
    }

    #[async_trait]
    impl ErrorCodeRegistration for MockErrorCodeRegistrationRepo {
        async fn create(
            &self,
            error_code: ErrorCode,
        ) -> Result<CreateResponseKind<ErrorCode>, MappedErrors> {
            match self.generate_error {
                true => {
                    return use_case_err(
                        "Error while registering error code.".to_string(),
                    )
                    .as_error()
                }
                false => Ok(CreateResponseKind::Created(error_code)),
            }
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Test cases
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_register_error_code() {
        let profile = Profile {
            owners: vec![Owner {
                id: Uuid::from_str("d776e96f-9417-4520-b2a9-9298136031b0")
                    .unwrap(),
                email: "agrobiota-results-expert-creator@biotrop.com.br"
                    .to_string(),
                first_name: Some("first_name".to_string()),
                last_name: Some("last_name".to_string()),
                username: Some("username".to_string()),
            }],
            acc_id: Uuid::from_str("d776e96f-9417-4520-b2a9-9298136031b0")
                .unwrap(),
            is_subscription: false,
            is_manager: true,
            is_staff: false,
            owner_is_active: true,
            account_is_active: true,
            account_was_approved: true,
            account_was_archived: false,
            verbose_status: None,
            licensed_resources: None,
        };

        let details = Some("details".to_string());

        let error_code = register_error_code(
            profile,
            "TEST".to_string(),
            "Test error.".to_string(),
            details.to_owned(),
            true,
            Box::new(&MockErrorCodeRegistrationRepo {
                generate_error: false,
            }),
        )
        .await
        .unwrap();

        assert_eq!(error_code.prefix, "TEST");
        assert_eq!(error_code.error_number, 0);
        assert_eq!(error_code.message, "Test error.");
        assert_eq!(error_code.details, details);
        assert_eq!(error_code.is_internal, true);
    }

    #[tokio::test]
    async fn test_register_error_code_with_invalid_prefix() {
        let profile = Profile {
            owners: vec![Owner {
                id: Uuid::from_str("d776e96f-9417-4520-b2a9-9298136031b0")
                    .unwrap(),
                email: "agrobiota-results-expert-creator@biotrop.com.br"
                    .to_string(),
                first_name: Some("first_name".to_string()),
                last_name: Some("last_name".to_string()),
                username: Some("username".to_string()),
            }],
            acc_id: Uuid::from_str("d776e96f-9417-4520-b2a9-9298136031b0")
                .unwrap(),
            is_subscription: false,
            is_manager: true,
            is_staff: false,
            owner_is_active: true,
            account_is_active: true,
            account_was_approved: true,
            account_was_archived: false,
            verbose_status: None,
            licensed_resources: None,
        };

        let error_code = register_error_code(
            profile,
            "".to_string(),
            "Test error.".to_string(),
            None,
            false,
            Box::new(&MockErrorCodeRegistrationRepo {
                generate_error: false,
            }),
        )
        .await;

        assert!(error_code.is_err());
    }
}
