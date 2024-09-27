use crate::domain::dtos::account::{Account, VerboseStatus};

use mycelium_base::utils::errors::{use_case_err, MappedErrors};

/// Try to reach a desired account state
///
/// The desired state is reached when the need set of flags reaches the boolean
/// final state without generate errors.
#[tracing::instrument(name = "try_to_reach_desired_status", skip_all)]
pub(super) async fn try_to_reach_desired_status(
    mut account: Account,
    desired_status: VerboseStatus,
) -> Result<Account, MappedErrors> {
    if !should_perform_state_transition(
        desired_status.to_owned(),
        account.to_owned().verbose_status,
    ) {
        return use_case_err(format!(
            "Could not transit from `{:?}` to `{:?}`",
            account.verbose_status.unwrap(),
            desired_status,
        ))
        .as_error();
    }

    let flags = desired_status.to_flags()?;

    if flags.is_active.is_some() {
        account.is_active = flags.is_active.unwrap();
    }

    if flags.is_checked.is_some() {
        account.is_checked = flags.is_checked.unwrap();
    }

    if flags.is_archived.is_some() {
        account.is_archived = flags.is_archived.unwrap();
    }

    Ok(account)
}

/// Check if the state transition is valid
///
/// Some state transitions are prohibited. This function guarantees that such
/// operations not occurs.
///
fn should_perform_state_transition(
    new_state: VerboseStatus,
    old_state: Option<VerboseStatus>,
) -> bool {
    let mut allowed_statuses = vec![None, Some(new_state.to_owned())];

    match new_state {
        VerboseStatus::Active => allowed_statuses.extend(vec![
            Some(VerboseStatus::Pending),
            Some(VerboseStatus::Inactive),
        ]),

        VerboseStatus::Pending => {
            allowed_statuses.extend(vec![Some(VerboseStatus::Archived)])
        }

        VerboseStatus::Inactive => {
            allowed_statuses.extend(vec![Some(VerboseStatus::Active)])
        }

        VerboseStatus::Archived => allowed_statuses.extend(vec![
            Some(VerboseStatus::Pending),
            Some(VerboseStatus::Active),
            Some(VerboseStatus::Inactive),
        ]),

        VerboseStatus::Unknown => return false,
    };

    allowed_statuses.contains(&old_state.to_owned())
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{should_perform_state_transition, try_to_reach_desired_status};
    use crate::domain::dtos::{
        account::{Account, AccountType, VerboseStatus},
        email::Email,
        user::User,
    };

    use chrono::Local;
    use mycelium_base::dtos::{Children, Parent};

    #[tokio::test]
    async fn test_if_try_to_reach_desired_status_works() {
        let account_type = AccountType {
            id: None,
            name: "".to_string(),
            description: "".to_string(),
            is_subscription: false,
            is_manager: false,
            is_staff: false,
        };

        let user = User::new(
            None,
            "username".to_string(),
            Email::from_string("username@email.domain".to_string()).unwrap(),
            Some("first_name".to_string()),
            Some("last_name".to_string()),
            true,
            Local::now(),
            Some(Local::now()),
            None,
            None,
        )
        .with_principal(false);

        let mut account = Account {
            id: None,
            name: String::from("Account Name"),
            slug: String::from("account-name"),
            tags: None,
            is_active: true,
            is_checked: false,
            is_archived: false,
            verbose_status: None,
            is_default: false,
            owners: Children::Records([user].to_vec()),
            account_type: Parent::Record(account_type),
            guest_users: None,
            created: Local::now(),
            updated: Some(Local::now()),
        };

        account.verbose_status = Some(VerboseStatus::from_flags(
            account.is_active,
            account.is_checked,
            account.is_archived,
        ));

        for status in vec![
            VerboseStatus::Active,
            VerboseStatus::Pending,
            VerboseStatus::Archived,
        ] {
            let response = match try_to_reach_desired_status(
                account.to_owned(),
                status.to_owned(),
            )
            .await
            {
                Err(err) => panic!("{err}"),
                Ok(res) => res,
            };

            println!("\nFrom {:?} To {:?}", account.verbose_status, status);
            println!(
                "Account ({:?})\t-> Res ({:?})\t[is_checked]",
                account.to_owned().is_checked,
                response.is_checked
            );
            println!(
                "Account ({:?})\t-> Res ({:?})\t[is_active]",
                account.to_owned().is_active,
                response.is_active
            );
            println!(
                "Account ({:?})\t-> Res ({:?})\t[is_archived]",
                account.to_owned().is_archived,
                response.is_archived
            );
        }
    }

    #[test]
    fn test_if_should_perform_state_transition_works() {
        for (is_allowed, desired_state, current_state) in [
            // Allowed operations
            (true, VerboseStatus::Active, Some(VerboseStatus::Pending)),
            (true, VerboseStatus::Active, Some(VerboseStatus::Inactive)),
            (true, VerboseStatus::Archived, Some(VerboseStatus::Pending)),
            (true, VerboseStatus::Archived, Some(VerboseStatus::Inactive)),
            (true, VerboseStatus::Pending, Some(VerboseStatus::Archived)),
            (true, VerboseStatus::Inactive, Some(VerboseStatus::Active)),
            // Disallowed operations
            (false, VerboseStatus::Active, Some(VerboseStatus::Archived)),
            (false, VerboseStatus::Inactive, Some(VerboseStatus::Pending)),
        ] {
            let response =
                should_perform_state_transition(desired_state, current_state);

            assert_eq!(is_allowed, response);
        }
    }
}
