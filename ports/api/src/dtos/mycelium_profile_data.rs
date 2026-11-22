use crate::middleware::{
    fetch_profile_from_request_connection_string,
    fetch_profile_from_request_token,
};

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures::Future;
use myc_core::domain::dtos::{
    account::VerboseStatus,
    profile::{FederatedResources, LicensedResources, Owner, TenantsOwnership},
    security_group::PermissionedRole,
};
use myc_http_tools::{
    responses::GatewayError,
    settings::{
        DEFAULT_CONNECTION_STRING_KEY, DEFAULT_MYCELIUM_ROLE_KEY,
        DEFAULT_TENANT_ID_KEY,
    },
    AccountMetaKey, Profile,
};
use serde::Deserialize;
use std::{collections::HashMap, pin::Pin};
use tracing::{error, trace};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MyceliumProfileData {
    pub owners: Vec<Owner>,
    pub acc_id: Uuid,
    pub is_subscription: bool,
    pub is_manager: bool,
    pub is_staff: bool,
    pub owner_is_active: bool,
    pub account_is_active: bool,
    pub account_was_approved: bool,
    pub account_was_archived: bool,
    pub account_was_deleted: bool,
    pub verbose_status: Option<VerboseStatus>,
    pub licensed_resources: Option<LicensedResources>,
    #[serde(default)]
    pub federated_resources: Option<FederatedResources>,
    pub tenants_ownership: Option<TenantsOwnership>,
    pub meta: Option<HashMap<AccountMetaKey, String>>,
}

impl MyceliumProfileData {
    pub(crate) fn from_profile(profile: Profile) -> Self {
        Self {
            owners: profile.owners,
            acc_id: profile.acc_id,
            is_subscription: profile.is_subscription,
            is_manager: profile.is_manager,
            is_staff: profile.is_staff,
            owner_is_active: profile.owner_is_active,
            account_is_active: profile.account_is_active,
            account_was_approved: profile.account_was_approved,
            account_was_archived: profile.account_was_archived,
            account_was_deleted: profile.account_was_deleted,
            verbose_status: profile.verbose_status,
            licensed_resources: profile.licensed_resources,
            federated_resources: profile.federated_resources,
            tenants_ownership: profile.tenants_ownership,
            meta: profile.meta,
        }
    }

    pub(crate) fn to_profile(&self) -> Profile {
        let mut profile = Profile::new(
            self.owners.to_owned(),
            self.acc_id,
            self.is_subscription,
            self.is_manager,
            self.is_staff,
            self.owner_is_active,
            self.account_is_active,
            self.account_was_approved,
            self.account_was_archived,
            self.account_was_deleted,
            self.verbose_status.to_owned(),
            self.licensed_resources.to_owned(),
            self.tenants_ownership.to_owned(),
        );

        profile.meta = self.meta.to_owned();
        profile.federated_resources = self.federated_resources.to_owned();

        profile
    }
}

impl FromRequest for MyceliumProfileData {
    type Error = GatewayError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    #[tracing::instrument(name = "mycelium_profile_from_request", skip_all)]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req_clone = req.clone();

        //
        // Get the tenant from the request
        //
        let tenant = match req.headers().get(DEFAULT_TENANT_ID_KEY) {
            Some(tenant) => match tenant.to_str() {
                Ok(tenant) => match Uuid::parse_str(tenant) {
                    Ok(tenant_uuid) => Some(tenant_uuid),
                    Err(err) => {
                        error!("Failed to parse tenant: {err}");
                        None
                    }
                },
                Err(err) => {
                    error!("Failed to parse tenant: {err}");

                    None
                }
            },
            None => None,
        };

        if let Some(tenant) = tenant {
            trace!("Requested tenant: {:?}", tenant);
        }

        //
        // Get the roles from the request
        //
        let roles: Option<Vec<PermissionedRole>> =
            match req_clone.headers().get(DEFAULT_MYCELIUM_ROLE_KEY) {
                Some(roles) => {
                    let roles: Option<Vec<String>> =
                        match serde_json::from_str(roles.to_str().unwrap()) {
                            Ok(roles) => roles,
                            Err(err) => {
                                error!("Failed to parse roles: {err}");

                                None
                            }
                        };

                    if let Some(roles) = roles {
                        if roles.is_empty() {
                            None
                        } else {
                            let roles = roles
                                .iter()
                                .map(|r| PermissionedRole {
                                    name: r.to_owned(),
                                    permission: None,
                                })
                                .collect();

                            Some(roles)
                        }
                    } else {
                        None
                    }
                }
                None => None,
            };

        if let Some(roles) = roles.to_owned() {
            trace!("Requested roles: {:?}", roles);
        }

        if let Some(connection_string) =
            req_clone.headers().get(DEFAULT_CONNECTION_STRING_KEY)
        {
            if !connection_string.is_empty() {
                return Box::pin(async move {
                    fetch_profile_from_request_connection_string(
                        req_clone, tenant, roles,
                    )
                    .await
                });
            }
        }

        Box::pin(async move {
            fetch_profile_from_request_token(req_clone, tenant, roles).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::connection_string_test_fixtures::*;

    use actix_web::{http::StatusCode, test, web, App, HttpResponse};

    async fn profile_handler(profile: MyceliumProfileData) -> HttpResponse {
        HttpResponse::Ok().body(profile.acc_id.to_string())
    }

    async fn call_rest_handler(
        connection_string: &str,
    ) -> (StatusCode, String) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(life_cycle()))
                .route("/profile", web::get().to(profile_handler)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/profile")
            .insert_header((DEFAULT_CONNECTION_STRING_KEY, connection_string))
            .to_request();

        let response = test::call_service(&app, req).await;
        let status = response.status();
        let body = test::read_body(response).await;

        (status, String::from_utf8_lossy(&body).to_string())
    }

    #[actix_web::test]
    async fn rest_handler_rejects_notification_string() {
        let connection_string =
            notification_connection_string(&life_cycle()).await;

        let (status, body) = call_rest_handler(&connection_string).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert!(body.contains("MYC00033"), "{body}");
    }

    #[actix_web::test]
    async fn rest_handler_takes_general_string_to_the_database_lookup() {
        let connection_string = general_connection_string(&life_cycle()).await;

        // No SQL module is registered, so a string accepted by every check
        // before the lookup ends in 500 rather than 401.
        let (status, _) = call_rest_handler(&connection_string).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }
}
