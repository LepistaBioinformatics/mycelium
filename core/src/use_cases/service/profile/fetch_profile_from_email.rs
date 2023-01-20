use crate::{
    domain::{
        dtos::{
            email::Email,
            profile::{LicensedResources, Profile},
        },
        entities::{GuestUserFetching, ProfileFetching, TokenRegistration},
    },
    use_cases::service::token::register_token,
};

use chrono::DateTime;
use clean_base::{
    dtos::enums::{ChildrenEnum, ParentEnum},
    entities::default_response::{
        CreateResponseKind, FetchManyResponseKind, FetchResponseKind,
    },
    utils::errors::{use_case_err, MappedErrors},
};
use futures::future;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePack {
    pub profile: Profile,
    pub token: Uuid,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ProfileResponse {
    RegisteredUser(ProfilePack),
    UnregisteredUser(Email),
}

/// Fetch the user profile from email address.
///
/// Together the profile a token is registered and their id is returned to be
/// used during the response validation.
pub async fn fetch_profile_from_email(
    email: Email,
    requesting_service: String,
    profile_fetching_repo: Box<&dyn ProfileFetching>,
    guest_user_fetching_repo: Box<&dyn GuestUserFetching>,
    token_registration_repo: Box<&dyn TokenRegistration>,
) -> Result<ProfileResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch the profile and guest from database
    // ? -----------------------------------------------------------------------

    let (profile, guests) = future::join(
        profile_fetching_repo.get(email.to_owned()),
        guest_user_fetching_repo.list(None, Some(email.to_owned())),
    )
    .await;

    // ? -----------------------------------------------------------------------
    // ? Validate profile response
    // ? -----------------------------------------------------------------------

    let mut profile = match profile {
        Err(err) => return Err(err),
        Ok(res) => match res {
            FetchResponseKind::NotFound(_) => {
                return Ok(ProfileResponse::UnregisteredUser(email))
            }
            FetchResponseKind::Found(profile) => profile,
        },
    };

    // ? -----------------------------------------------------------------------
    // ? Validate guests response
    // ? -----------------------------------------------------------------------

    let guests = match guests {
        Err(err) => return Err(err),
        Ok(res) => match res {
            FetchManyResponseKind::NotFound => None,
            FetchManyResponseKind::Found(records) => {
                for record in records.to_owned() {
                    match record.guest_role {
                        ParentEnum::Id(_) => {
                            error!(
                                "Unexpected error on extract profile information
                                from guest {:?} associated to email {:?}",
                                record,
                                email.get_email().to_owned(),
                            );

                            return Err(use_case_err(
                                String::from(
                                    "Unexpected error on fetch profile from email.",
                                ),
                                None,
                                None,
                            ));
                        }
                        _ => (),
                    }
                }

                let guests = records
                    .to_owned()
                    .into_iter()
                    .map(|guest| {
                        let accounts = match guest.accounts {
                            None => vec![],
                            Some(res) => match res {
                                ChildrenEnum::Ids(ids) => ids,
                                ChildrenEnum::Records(records) => records
                                    .into_iter()
                                    .map(|record| record.id.unwrap())
                                    .collect::<Vec<Uuid>>(),
                            },
                        };

                        let guest_role = match guest.guest_role {
                            ParentEnum::Id(_) => panic!("expr"),
                            ParentEnum::Record(record) => record,
                        };

                        accounts
                            .into_iter()
                            .map(|account_id| LicensedResources {
                                guest_account_id: account_id,
                                role: guest_role.to_owned().name,
                                permissions: guest_role.to_owned().permissions,
                                created: guest.created.into(),
                                updated: match guest.updated {
                                    None => None,
                                    Some(res) => Some(DateTime::from(res)),
                                },
                            })
                            .collect::<Vec<LicensedResources>>()
                    })
                    .flatten()
                    .collect::<Vec<LicensedResources>>();

                Some(guests)
            }
        },
    };

    // ? -----------------------------------------------------------------------
    // ? Update profile response to include guests
    // ? -----------------------------------------------------------------------

    profile.licensed_resources = guests;

    debug!("Build profile: {:?}", profile);

    // ? -----------------------------------------------------------------------
    // ? Register a new token
    // ? -----------------------------------------------------------------------

    let token =
        match register_token(requesting_service, token_registration_repo).await
        {
            Err(err) => return Err(err),
            Ok(res) => match res {
                CreateResponseKind::NotCreated(_, msg) => {
                    return Err(use_case_err(msg, None, None))
                }
                CreateResponseKind::Created(token) => token,
            },
        };

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(ProfileResponse::RegisteredUser(ProfilePack {
        profile,
        token: token.token,
    }))
}
