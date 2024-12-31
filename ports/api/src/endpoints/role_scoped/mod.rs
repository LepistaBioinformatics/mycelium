pub(crate) mod account_manager;
pub(crate) mod beginners;
pub(crate) mod gateway_manager;
pub(crate) mod guest_manager;
pub(crate) mod subscriptions_manager;
pub(crate) mod system_manager;
pub(crate) mod tenant_manager;
pub(crate) mod tenant_owner;
pub(crate) mod users_manager;

use super::shared::{insert_role_header, UrlGroup};

use account_manager::guest_endpoints as account_manager_guest_endpoints;
use actix_web::{dev::Service, web};
use beginners::{
    account_endpoints as no_role_account_endpoints,
    guest_user_endpoints as no_role_guest_user_endpoints,
    profile_endpoints as no_role_profile_endpoints,
    user_endpoints as no_role_user_endpoints,
};
use gateway_manager::{
    route_endpoints as gateway_manager_route_endpoints,
    service_endpoints as gateway_manager_service_endpoints,
};
use guest_manager::{
    guest_role_endpoints as guest_manager_guest_role_endpoints,
    role_endpoints as guest_manager_role_endpoints,
    token_endpoints as guest_manager_token_endpoints,
};
use myc_core::domain::actors::SystemActor;
use subscriptions_manager::{
    account_endpoints as subscription_manager_account_endpoints,
    guest_endpoints as subscription_manager_guest_endpoints,
    tag_endpoints as subscription_manager_tag_endpoints,
};
use system_manager::{
    error_code_endpoints as system_manager_error_code_endpoints,
    webhook_endpoints as system_manager_webhook_endpoints,
};
use tenant_manager::{
    account_endpoints as tenant_manager_account_endpoints,
    tag_endpoints as tenant_manager_tag_endpoints,
    token_endpoints as tenant_manager_token_endpoints,
};
use tenant_owner::{
    account_endpoints as tenant_owner_account_endpoints,
    meta_endpoints as tenant_owner_meta_endpoints,
    owner_endpoints as tenant_owner_owner_endpoints,
    tenant_endpoints as tenant_owner_tenant_endpoints,
};
use users_manager::account_endpoints as user_manager_account_endpoints;

// ? ---------------------------------------------------------------------------
// ? Configure application re-routing
// ? ---------------------------------------------------------------------------

pub(crate) fn configure(config: &mut web::ServiceConfig) {
    config
        //
        // Beginners
        //
        .service(
            web::scope(&format!(
                "/{}",
                SystemActor::Beginner.to_string().as_str()
            ))
            //
            // Configure the standard role endpoints
            //
            .service(
                web::scope(&format!("/{}", UrlGroup::Accounts))
                    .configure(no_role_account_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Guests))
                    .configure(no_role_guest_user_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Profile))
                    .configure(no_role_profile_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Users))
                    .configure(no_role_user_endpoints::configure),
            ),
        )
        //
        // Gateway Managers
        //
        .service(
            web::scope(&format!(
                "/{}",
                SystemActor::GatewayManager.to_string().as_str()
            ))
            //
            // Inject a header to be collected by the MyceliumProfileData
            // extractor.
            //
            // Endpoints restricted to users with the role:
            // - GatewayManager
            //
            .wrap_fn(|req, srv| {
                let req =
                    insert_role_header(req, vec![SystemActor::GatewayManager]);
                srv.call(req)
            })
            //
            // Configure the standard role endpoints
            //
            .service(
                web::scope(&format!("/{}", UrlGroup::Routes))
                    .configure(gateway_manager_route_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Services))
                    .configure(gateway_manager_service_endpoints::configure),
            ),
        )
        //
        // Guest Managers
        //
        .service(
            web::scope(&format!(
                "/{}",
                SystemActor::GuestManager.to_string().as_str()
            ))
            //
            // Inject a header to be collected by the MyceliumProfileData
            // extractor.
            //
            // Endpoints restricted to users with the role:
            // - GuestManager
            //
            .wrap_fn(|req, srv| {
                let req =
                    insert_role_header(req, vec![SystemActor::GuestManager]);
                srv.call(req)
            })
            //
            // Configure the standard role endpoints
            //
            .service(
                web::scope(&format!("/{}", UrlGroup::Roles))
                    .configure(guest_manager_role_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::GuestRoles))
                    .configure(guest_manager_guest_role_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Tokens))
                    .configure(guest_manager_token_endpoints::configure),
            ),
        )
        //
        // Subscription Managers
        //
        .service(
            web::scope(&format!(
                "/{}",
                SystemActor::SubscriptionsManager.to_string().as_str()
            ))
            //
            // Inject a header to be collected by the MyceliumProfileData
            // extractor.
            //
            // Endpoints restricted to users with the role:
            // - TenantOwner
            // - TenantManager
            // - SubscriptionsManager
            //
            .wrap_fn(|req, srv| {
                let req = insert_role_header(
                    req,
                    vec![
                        SystemActor::TenantOwner,
                        SystemActor::TenantManager,
                        SystemActor::SubscriptionsManager,
                    ],
                );
                srv.call(req)
            })
            //
            // Configure the standard role endpoints
            //
            .service(
                web::scope(&format!("/{}", UrlGroup::Accounts)).configure(
                    subscription_manager_account_endpoints::configure,
                ),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Tags))
                    .configure(subscription_manager_tag_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Guests))
                    .configure(subscription_manager_guest_endpoints::configure),
            ),
        )
        //
        // Account Managers
        //
        .service(
            web::scope(&format!(
                "/{}",
                SystemActor::AccountManager.to_string().as_str()
            ))
            //
            // Inject a header to be collected by the MyceliumProfileData
            // extractor.
            //
            // Endpoints restricted to users with the role:
            // - AccountManager
            //
            .wrap_fn(|req, srv| {
                let req =
                    insert_role_header(req, vec![SystemActor::AccountManager]);
                srv.call(req)
            })
            //
            // Configure the standard role endpoints
            //
            .service(
                web::scope(&format!("/{}", UrlGroup::Guests))
                    .configure(account_manager_guest_endpoints::configure),
            ),
        )
        //
        // System Managers
        //
        .service(
            web::scope(&format!(
                "/{}",
                SystemActor::SystemManager.to_string().as_str()
            ))
            //
            // Inject a header to be collected by the MyceliumProfileData
            // extractor.
            //
            // Endpoints restricted to users with the role:
            // - SystemManager
            //
            .wrap_fn(|req, srv| {
                let req =
                    insert_role_header(req, vec![SystemActor::SystemManager]);
                srv.call(req)
            })
            //
            // Configure the standard role endpoints
            //
            .service(
                web::scope(&format!("/{}", UrlGroup::ErrorCodes))
                    .configure(system_manager_error_code_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Webhooks))
                    .configure(system_manager_webhook_endpoints::configure),
            ),
        )
        //
        // Tenant Manager
        //
        .service(
            web::scope(&format!(
                "/{}",
                SystemActor::TenantManager.to_string().as_str()
            ))
            //
            // Inject a header to be collected by the MyceliumProfileData
            // extractor.
            //
            // Endpoints restricted to users with the role:
            // - TenantOwner
            // - TenantManager
            //
            .wrap_fn(|req, srv| {
                let req = insert_role_header(
                    req,
                    vec![SystemActor::TenantOwner, SystemActor::TenantManager],
                );
                srv.call(req)
            })
            //
            // Configure the standard role endpoints
            //
            .service(
                web::scope(&format!("/{}", UrlGroup::Accounts))
                    .configure(tenant_manager_account_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Tags))
                    .configure(tenant_manager_tag_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Tokens))
                    .configure(tenant_manager_token_endpoints::configure),
            ),
        )
        //
        // Tenant Owner
        //
        .service(
            web::scope(&format!(
                "/{}",
                SystemActor::TenantOwner.to_string().as_str()
            ))
            //
            // Configure the standard role endpoints
            //
            .service(
                web::scope(&format!("/{}", UrlGroup::Accounts))
                    .configure(tenant_owner_account_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Meta))
                    .configure(tenant_owner_meta_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Owners))
                    .configure(tenant_owner_owner_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Tenants))
                    .configure(tenant_owner_tenant_endpoints::configure),
            ),
        )
        //
        // User Accounts Managers
        //
        .service(
            web::scope(&format!(
                "/{}",
                SystemActor::UsersManager.to_string().as_str()
            ))
            //
            // Inject a header to be collected by the MyceliumProfileData
            // extractor.
            //
            // Endpoints restricted to users with the role:
            // - UsersManager
            //
            .wrap_fn(|req, srv| {
                let req =
                    insert_role_header(req, vec![SystemActor::UsersManager]);
                srv.call(req)
            })
            //
            // Configure the standard role endpoints
            //
            .service(
                web::scope(&format!("/{}", UrlGroup::Accounts))
                    .configure(user_manager_account_endpoints::configure),
            ),
        );
}
