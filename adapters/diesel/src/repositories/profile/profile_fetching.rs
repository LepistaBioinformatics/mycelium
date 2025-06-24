use crate::{
    models::{
        account::Account as AccountModel, config::DbPoolProvider,
        user::User as UserModel,
    },
    schema::{account as account_model, user as user_model},
};
use diesel::prelude::*;
use myc_core::domain::dtos::account_type::AccountType;

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{
        account::VerboseStatus,
        email::Email,
        native_error_codes::NativeErrorCodes,
        profile::{Owner, Profile},
    },
    entities::ProfileFetching,
};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use serde_json::from_value;
use shaku::Component;
use std::sync::Arc;
use tracing::error;

#[derive(Component)]
#[shaku(interface = ProfileFetching)]
pub struct ProfileFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl ProfileFetching for ProfileFetchingSqlDbRepository {
    #[tracing::instrument(name = "get_profile_from_email", skip_all)]
    async fn get_from_email(
        &self,
        email: Email,
    ) -> Result<FetchResponseKind<Profile, String>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let result =
            user_model::table
                .inner_join(account_model::table.on(
                    account_model::id.nullable().eq(user_model::account_id),
                ))
                .filter(
                    user_model::email
                        .eq(email.email())
                        .and(account_model::is_deleted.eq(false)),
                )
                .select((AccountModel::as_select(), UserModel::as_select()))
                .first::<(AccountModel, UserModel)>(conn)
                .optional()
                .map_err(|e| {
                    fetching_err(format!("Failed to fetch profile: {}", e))
                })?;

        match result {
            None => Ok(FetchResponseKind::NotFound(Some(email.email()))),
            Some((account, owner)) => {
                let account_type: AccountType =
                    from_value(account.account_type).map_err(|err| {
                        error!("Error on discovery account type: {err}");
                        fetching_err(
                            "Unexpected error on discovery account type.",
                        )
                    })?;

                let (is_subscription, is_manager, is_staff) = match account_type
                {
                    AccountType::Subscription { .. }
                    | AccountType::RoleAssociated { .. } => {
                        (true, false, false)
                    }
                    AccountType::Manager => (false, true, false),
                    AccountType::Staff => (false, true, true),
                    _ => (false, false, false),
                };

                let is_active = owner.is_principal;
                let owner = Owner {
                    id: owner.id,
                    email: Email::from_string(owner.email)?.email(),
                    first_name: Some(owner.first_name),
                    last_name: Some(owner.last_name),
                    username: Some(owner.username),
                    is_principal: owner.is_principal,
                };

                Ok(FetchResponseKind::Found(Profile::new(
                    vec![owner],
                    account.id,
                    is_subscription,
                    is_manager,
                    is_staff,
                    is_active,
                    account.is_active,
                    account.is_checked,
                    account.is_archived,
                    account.is_deleted,
                    Some(VerboseStatus::from_flags(
                        account.is_active,
                        account.is_checked,
                        account.is_archived,
                        account.is_deleted,
                    )),
                    None,
                    None,
                )))
            }
        }
    }

    async fn get_from_token(
        &self,
        _token: String,
    ) -> Result<FetchResponseKind<Profile, String>, MappedErrors> {
        unimplemented!("Not implemented yet: Fetch profile from token")
    }
}
