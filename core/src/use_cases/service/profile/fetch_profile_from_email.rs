use crate::domain::{
    dtos::{
        email::Email,
        profile::{LicensedResources, Profile, TenantsOwnership},
        security_group::PermissionedRole,
    },
    entities::{LicensedResourcesFetching, ProfileFetching},
};

use futures::future;
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ProfileResponse {
    RegisteredUser(Profile),
    UnregisteredUser(Email),
}

/// Fetch the user profile from email address.
#[tracing::instrument(
    name = "fetch_profile_from_email", 
    fields(
        email = %email.redacted_email(),
    ),
    skip(email, profile_fetching_repo, licensed_resources_fetching_repo)
)]
pub async fn fetch_profile_from_email(
    email: Email,
    was_verified: Option<bool>,
    tenant: Option<Uuid>,
    roles: Option<Vec<PermissionedRole>>,
    profile_fetching_repo: Box<&dyn ProfileFetching>,
    licensed_resources_fetching_repo: Box<&dyn LicensedResourcesFetching>,
) -> Result<ProfileResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch the profile and guest from database
    // ? -----------------------------------------------------------------------

    let (profile, licenses, ownership) = future::join3(
        profile_fetching_repo.get_from_email(email.to_owned()),
        licensed_resources_fetching_repo.list_licensed_resources(
            email.to_owned(),
            tenant,
            roles,
            None,
            was_verified,
        ),
        licensed_resources_fetching_repo
            .list_tenants_ownership(email.to_owned(), tenant),
    )
    .await;

    // ? -----------------------------------------------------------------------
    // ? Validate profile response
    // ? -----------------------------------------------------------------------

    let mut profile = match profile? {
        FetchResponseKind::NotFound(_) => {
            return Ok(ProfileResponse::UnregisteredUser(email))
        }
        FetchResponseKind::Found(profile) => profile,
    };

    // ? -----------------------------------------------------------------------
    // ? Validate guests response
    // ? -----------------------------------------------------------------------

    profile.licensed_resources = match licenses? {
        FetchManyResponseKind::NotFound => None,
        FetchManyResponseKind::Found(records) => {
            Some(LicensedResources::Records(records))
        }
        FetchManyResponseKind::FoundPaginated { .. } => return fetching_err(
            "Paginated licenses not supported when fetch profile from email",
        )
        .as_error(),
    };

    // ? -----------------------------------------------------------------------
    // ? Validate ownership response
    // ? -----------------------------------------------------------------------

    profile.tenants_ownership = match ownership? {
        FetchManyResponseKind::NotFound => None,
        FetchManyResponseKind::Found(records) => {
            Some(TenantsOwnership::Records(records))
        }
        FetchManyResponseKind::FoundPaginated { .. } => return fetching_err(
            "Paginated ownership not supported when fetch profile from email",
        )
        .as_error(),
    };

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(ProfileResponse::RegisteredUser(profile))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::{
        profile::{LicensedResource, Owner, TenantOwnership},
        related_accounts::RelatedAccounts,
    };

    use async_trait::async_trait;

    struct MockProfileRepo;

    #[async_trait]
    impl ProfileFetching for MockProfileRepo {
        async fn get_from_email(
            &self,
            email: Email,
        ) -> Result<FetchResponseKind<Profile, String>, MappedErrors> {
            Ok(FetchResponseKind::Found(Profile::new(
                vec![Owner {
                    id: Uuid::new_v4(),
                    email: email.email(),
                    first_name: None,
                    last_name: None,
                    username: None,
                    is_principal: true,
                }],
                Uuid::new_v4(),
                false,
                false,
                false,
                true,
                true,
                true,
                false,
                false,
                None,
                None,
                None,
            )))
        }

        async fn get_from_token(
            &self,
            _: String,
        ) -> Result<FetchResponseKind<Profile, String>, MappedErrors> {
            unimplemented!()
        }
    }

    struct MockLicensedResourcesRepo {
        paginated_licenses: bool,
        paginated_ownership: bool,
    }

    #[async_trait]
    impl LicensedResourcesFetching for MockLicensedResourcesRepo {
        async fn list_licensed_resources(
            &self,
            _: Email,
            _: Option<Uuid>,
            _: Option<Vec<PermissionedRole>>,
            _: Option<RelatedAccounts>,
            _: Option<bool>,
        ) -> Result<FetchManyResponseKind<LicensedResource>, MappedErrors>
        {
            match self.paginated_licenses {
                true => Ok(FetchManyResponseKind::FoundPaginated {
                    count: 0,
                    skip: None,
                    size: None,
                    records: vec![],
                }),
                false => Ok(FetchManyResponseKind::NotFound),
            }
        }

        async fn list_tenants_ownership(
            &self,
            _: Email,
            _: Option<Uuid>,
        ) -> Result<FetchManyResponseKind<TenantOwnership>, MappedErrors>
        {
            match self.paginated_ownership {
                true => Ok(FetchManyResponseKind::FoundPaginated {
                    count: 0,
                    skip: None,
                    size: None,
                    records: vec![],
                }),
                false => Ok(FetchManyResponseKind::NotFound),
            }
        }
    }

    async fn run(
        paginated_licenses: bool,
        paginated_ownership: bool,
    ) -> Result<ProfileResponse, MappedErrors> {
        fetch_profile_from_email(
            Email::from_string("user@domain.com".to_string()).unwrap(),
            None,
            None,
            None,
            Box::new(&MockProfileRepo),
            Box::new(&MockLicensedResourcesRepo {
                paginated_licenses,
                paginated_ownership,
            }),
        )
        .await
    }

    #[tokio::test]
    async fn non_paginated_responses_return_registered_user() {
        let response = run(false, false).await;

        assert!(matches!(response, Ok(ProfileResponse::RegisteredUser(_))));
    }

    #[tokio::test]
    async fn paginated_licenses_return_error_instead_of_panic() {
        assert!(run(true, false).await.is_err());
    }

    #[tokio::test]
    async fn paginated_ownership_return_error_instead_of_panic() {
        assert!(run(false, true).await.is_err());
    }
}
