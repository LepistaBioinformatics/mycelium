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

use account_manager::{
    guest_endpoints as account_manager_guest_endpoints,
    guest_role_endpoints as account_manager_guest_role_endpoints,
};
use actix_web::{dev::Service, web};
use beginners::{
    account_endpoints as beginners_account_endpoints,
    guest_user_endpoints as beginners_guest_user_endpoints,
    meta_endpoints as beginners_meta_endpoints,
    profile_endpoints as beginners_profile_endpoints,
    tenant_endpoints as beginners_tenant_endpoints,
    token_endpoints as beginners_token_endpoints,
    user_endpoints as beginners_user_endpoints,
};
use gateway_manager::{
    route_endpoints as gateway_manager_route_endpoints,
    service_endpoints as gateway_manager_service_endpoints,
    tools_endpoints as gateway_manager_tools_endpoints,
};
use guest_manager::guest_role_endpoints as guest_manager_guest_role_endpoints;
use myc_core::domain::actors::SystemActor;
use subscriptions_manager::{
    account_endpoints as subscription_manager_account_endpoints,
    guest_endpoints as subscription_manager_guest_endpoints,
    guest_role_endpoints as subscription_manager_guest_role_endpoints,
    tag_endpoints as subscription_manager_tag_endpoints,
};
use system_manager::{
    error_code_endpoints as system_manager_error_code_endpoints,
    webhook_endpoints as system_manager_webhook_endpoints,
};
use tenant_manager::{
    account_endpoints as tenant_manager_account_endpoints,
    guest_endpoints as tenant_manager_guest_endpoints,
    tag_endpoints as tenant_manager_tag_endpoints,
    tenant_endpoints as tenant_manager_tenant_endpoints,
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
            web::scope(SystemActor::Beginner.str())
                //
                // Configure the standard role endpoints
                //
                .service(
                    web::scope(UrlGroup::Accounts.str())
                        .configure(beginners_account_endpoints::configure),
                )
                .service(
                    web::scope(UrlGroup::Guests.str())
                        .configure(beginners_guest_user_endpoints::configure),
                )
                .service(
                    web::scope(UrlGroup::Meta.str())
                        .configure(beginners_meta_endpoints::configure),
                )
                .service(
                    web::scope(UrlGroup::Profile.str())
                        .configure(beginners_profile_endpoints::configure),
                )
                .service(
                    web::scope(UrlGroup::Tenants.str())
                        .configure(beginners_tenant_endpoints::configure),
                )
                .service(
                    web::scope(UrlGroup::Tokens.str())
                        .configure(beginners_token_endpoints::configure),
                )
                .service(
                    web::scope(UrlGroup::Users.str())
                        .configure(beginners_user_endpoints::configure),
                ),
        )
        //
        // Gateway Managers
        //
        .service(
            web::scope(SystemActor::GatewayManager.str())
                //
                // Inject a header to be collected by the MyceliumProfileData
                // extractor.
                //
                // Endpoints restricted to users with the role:
                // - GatewayManager
                //
                .wrap_fn(|req, srv| {
                    let req = insert_role_header(
                        req,
                        vec![SystemActor::GatewayManager],
                    );
                    srv.call(req)
                })
                //
                // Configure the standard role endpoints
                //
                .service(
                    web::scope(UrlGroup::Routes.str())
                        .configure(gateway_manager_route_endpoints::configure),
                )
                .service(
                    web::scope(UrlGroup::Services.str()).configure(
                        gateway_manager_service_endpoints::configure,
                    ),
                )
                .service(
                    web::scope(UrlGroup::Tools.str())
                        .configure(gateway_manager_tools_endpoints::configure),
                ),
        )
        //
        // Guest Managers
        //
        .service(
            web::scope(SystemActor::GuestsManager.str())
                //
                // Inject a header to be collected by the MyceliumProfileData
                // extractor.
                //
                // Endpoints restricted to users with the role:
                // - GuestManager
                //
                .wrap_fn(|req, srv| {
                    let req = insert_role_header(
                        req,
                        vec![SystemActor::GuestsManager],
                    );
                    srv.call(req)
                })
                //
                // Configure the standard role endpoints
                //
                .service(
                    web::scope(UrlGroup::GuestRoles.str()).configure(
                        guest_manager_guest_role_endpoints::configure,
                    ),
                ),
        )
        //
        // Subscription Managers
        //
        .service(
            web::scope(SystemActor::SubscriptionsManager.str())
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
                            SystemActor::TenantManager,
                            SystemActor::SubscriptionsManager,
                        ],
                    );
                    srv.call(req)
                })
                //
                // Configure the standard role endpoints
                //
                .service(web::scope(UrlGroup::Accounts.str()).configure(
                    subscription_manager_account_endpoints::configure,
                ))
                .service(
                    web::scope(UrlGroup::Tags.str()).configure(
                        subscription_manager_tag_endpoints::configure,
                    ),
                )
                .service(
                    web::scope(UrlGroup::Guests.str()).configure(
                        subscription_manager_guest_endpoints::configure,
                    ),
                )
                .service(web::scope(UrlGroup::GuestRoles.str()).configure(
                    subscription_manager_guest_role_endpoints::configure,
                )),
        )
        //
        // Account Managers
        //
        .service(
            web::scope(SystemActor::AccountManager.str())
                //
                // Inject a header to be collected by the MyceliumProfileData
                // extractor.
                //
                // Endpoints restricted to users with the role:
                // - AccountManager
                //
                //.wrap_fn(|req, srv| {
                //    let req = insert_role_header(
                //        req,
                //        vec![SystemActor::AccountManager],
                //    );
                //    srv.call(req)
                //})
                //
                // Configure the standard role endpoints
                //
                .service(
                    web::scope(UrlGroup::Guests.str())
                        .configure(account_manager_guest_endpoints::configure),
                )
                .service(web::scope(UrlGroup::GuestRoles.str()).configure(
                    account_manager_guest_role_endpoints::configure,
                )),
        )
        //
        // System Managers
        //
        .service(
            web::scope(SystemActor::SystemManager.str())
                //
                // Inject a header to be collected by the MyceliumProfileData
                // extractor.
                //
                // Endpoints restricted to users with the role:
                // - SystemManager
                //
                .wrap_fn(|req, srv| {
                    let req = insert_role_header(
                        req,
                        vec![SystemActor::SystemManager],
                    );
                    srv.call(req)
                })
                //
                // Configure the standard role endpoints
                //
                .service(
                    web::scope(UrlGroup::ErrorCodes.str()).configure(
                        system_manager_error_code_endpoints::configure,
                    ),
                )
                .service(
                    web::scope(UrlGroup::Webhooks.str())
                        .configure(system_manager_webhook_endpoints::configure),
                ),
        )
        //
        // Tenant Manager
        //
        .service(
            web::scope(SystemActor::TenantManager.str())
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
                        vec![SystemActor::TenantManager],
                    );
                    srv.call(req)
                })
                //
                // Configure the standard role endpoints
                //
                .service(
                    web::scope(UrlGroup::Accounts.str())
                        .configure(tenant_manager_account_endpoints::configure),
                )
                .service(
                    web::scope(UrlGroup::Guests.str())
                        .configure(tenant_manager_guest_endpoints::configure),
                )
                .service(
                    web::scope(UrlGroup::Tags.str())
                        .configure(tenant_manager_tag_endpoints::configure),
                )
                .service(
                    web::scope(UrlGroup::Tenants.str())
                        .configure(tenant_manager_tenant_endpoints::configure),
                ),
        )
        //
        // Tenant Owner
        //
        .service(
            web::scope(SystemActor::TenantOwner.str())
                //
                // Configure the standard role endpoints
                //
                .service(
                    web::scope(UrlGroup::Accounts.str())
                        .configure(tenant_owner_account_endpoints::configure),
                )
                .service(
                    web::scope(UrlGroup::Meta.str())
                        .configure(tenant_owner_meta_endpoints::configure),
                )
                .service(
                    web::scope(UrlGroup::Owners.str())
                        .configure(tenant_owner_owner_endpoints::configure),
                )
                .service(
                    web::scope(UrlGroup::Tenants.str())
                        .configure(tenant_owner_tenant_endpoints::configure),
                ),
        )
        //
        // User Accounts Managers
        //
        .service(
            web::scope(SystemActor::UsersManager.str())
                //
                // Inject a header to be collected by the MyceliumProfileData
                // extractor.
                //
                // Endpoints restricted to users with the role:
                // - UsersManager
                //
                .wrap_fn(|req, srv| {
                    let req = insert_role_header(
                        req,
                        vec![SystemActor::UsersManager],
                    );
                    srv.call(req)
                })
                //
                // Configure the standard role endpoints
                //
                .service(
                    web::scope(UrlGroup::Accounts.str())
                        .configure(user_manager_account_endpoints::configure),
                ),
        );
}
