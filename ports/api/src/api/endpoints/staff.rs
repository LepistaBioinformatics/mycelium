use clean_base::dtos::enums::{ChildrenEnum, ParentEnum};
use myc_core::domain::dtos::account::{Account, AccountType, AccountTypeEnum};
use utoipa::OpenApi;

// ? ---------------------------------------------------------------------------
// ? Configure the API documentation
// ? ---------------------------------------------------------------------------

#[derive(OpenApi)]
#[openapi(
    paths(
        account_endpoints::upgrade_account_privileges_url,
        account_endpoints::downgrade_account_privileges_url,
    ),
    components(
        schemas(
            // Default relationship enumerators.
            ChildrenEnum<String, String>,
            ParentEnum<String, String>,

            // Schema models.
            Account,
            AccountType,
            AccountTypeEnum,
        ),
    ),
    tags(
        (
            name = "service",
            description = "Service management endpoints."
        )
    ),
)]
pub struct ApiDoc;

// ? ---------------------------------------------------------------------------
// ? Create endpoints module
// ? ---------------------------------------------------------------------------

pub mod account_endpoints {

    use crate::modules::{
        AccountFetchingModule, AccountTypeRegistrationModule,
        AccountUpdatingModule,
    };

    use actix_web::{patch, web, HttpRequest, HttpResponse, Responder};
    use clean_base::entities::default_response::UpdatingResponseKind;
    use myc_core::{
        domain::{
            dtos::account::AccountTypeEnum,
            entities::{
                AccountFetching, AccountTypeRegistration, AccountUpdating,
            },
        },
        use_cases::staff::account::{
            downgrade_account_privileges, upgrade_account_privileges,
        },
    };
    use myc_http_tools::extractor::extract_profile;
    use serde::Deserialize;
    use shaku_actix::Inject;
    use utoipa::IntoParams;
    use uuid::Uuid;

    // ? -----------------------------------------------------------------------
    // ? Configure application
    // ? -----------------------------------------------------------------------

    pub fn configure(config: &mut web::ServiceConfig) {
        config.service(
            web::scope("/accounts")
                .service(upgrade_account_privileges_url)
                .service(downgrade_account_privileges_url),
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Define API structs
    // ? -----------------------------------------------------------------------

    #[derive(Deserialize, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct UpgradeAccountPrivilegesParams {
        pub target_account_type: AccountTypeEnum,
    }

    // ? -----------------------------------------------------------------------
    // ? Define API paths
    //
    // Account
    //
    // ? -----------------------------------------------------------------------

    /// Upgrade account privileges
    ///
    /// Increase permissions of the refereed account.
    #[utoipa::path(
        patch,
        path = "/staffs/accounts/{account}/upgrade",
        params(
            ("account" = Uuid, Path, description = "The account primary key."),
            UpgradeAccountPrivilegesParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = String,
            ),
            (
                status = 400,
                description = "Account not upgraded.",
                body = String,
            ),
            (
                status = 202,
                description = "Account upgraded.",
                body = Account,
            ),
        ),
    )]
    #[patch("/{account}/upgrade")]
    pub async fn upgrade_account_privileges_url(
        path: web::Path<Uuid>,
        info: web::Query<UpgradeAccountPrivilegesParams>,
        req: HttpRequest,
        account_fetching_repo: Inject<
            AccountFetchingModule,
            dyn AccountFetching,
        >,
        account_updating_repo: Inject<
            AccountUpdatingModule,
            dyn AccountUpdating,
        >,
        account_type_registration_repo: Inject<
            AccountTypeRegistrationModule,
            dyn AccountTypeRegistration,
        >,
    ) -> impl Responder {
        let profile = match extract_profile(req).await {
            Err(err) => return err,
            Ok(res) => res,
        };

        match upgrade_account_privileges(
            profile,
            path.to_owned(),
            info.target_account_type.to_owned(),
            Box::new(&*account_fetching_repo),
            Box::new(&*account_updating_repo),
            Box::new(&*account_type_registration_repo),
        )
        .await
        {
            Err(err) => {
                HttpResponse::InternalServerError().body(err.to_string())
            }
            Ok(res) => match res {
                UpdatingResponseKind::NotUpdated(_, msg) => {
                    HttpResponse::BadRequest().body(msg)
                }
                UpdatingResponseKind::Updated(record) => {
                    HttpResponse::Accepted().json(record)
                }
            },
        }
    }

    /// Downgrade account privileges
    ///
    /// Decrease permissions of the refereed account.
    #[utoipa::path(
        patch,
        path = "/staffs/accounts/{account}/downgrade",
        params(
            ("account" = Uuid, Path, description = "The account primary key."),
            UpgradeAccountPrivilegesParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = String,
            ),
            (
                status = 400,
                description = "Account not downgraded.",
                body = String,
            ),
            (
                status = 202,
                description = "Account downgraded.",
                body = Account,
            ),
        ),
    )]
    #[patch("/{account}/downgrade")]
    pub async fn downgrade_account_privileges_url(
        path: web::Path<Uuid>,
        info: web::Query<UpgradeAccountPrivilegesParams>,
        req: HttpRequest,
        account_fetching_repo: Inject<
            AccountFetchingModule,
            dyn AccountFetching,
        >,
        account_updating_repo: Inject<
            AccountUpdatingModule,
            dyn AccountUpdating,
        >,
        account_type_registration_repo: Inject<
            AccountTypeRegistrationModule,
            dyn AccountTypeRegistration,
        >,
    ) -> impl Responder {
        let profile = match extract_profile(req).await {
            Err(err) => return err,
            Ok(res) => res,
        };

        match downgrade_account_privileges(
            profile,
            path.to_owned(),
            info.target_account_type.to_owned(),
            Box::new(&*account_fetching_repo),
            Box::new(&*account_updating_repo),
            Box::new(&*account_type_registration_repo),
        )
        .await
        {
            Err(err) => {
                HttpResponse::InternalServerError().body(err.to_string())
            }
            Ok(res) => match res {
                UpdatingResponseKind::NotUpdated(_, msg) => {
                    HttpResponse::BadRequest().body(msg)
                }
                UpdatingResponseKind::Updated(record) => {
                    HttpResponse::Accepted().json(record)
                }
            },
        }
    }
}
