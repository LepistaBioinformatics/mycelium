use crate::domain::{
    dtos::{
        account::Account, profile::Profile, related_accounts::RelatedAccounts,
    },
    entities::AccountFetching,
};

use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use uuid::Uuid;

pub async fn get_my_account_details(
    profile: Profile,
    account_fetching_repo: Box<&dyn AccountFetching>,
) -> Result<FetchResponseKind<Account, Uuid>, MappedErrors> {
    account_fetching_repo
        .get(
            profile.acc_id,
            RelatedAccounts::AllowedAccounts(vec![profile.acc_id]),
        )
        .await
}
