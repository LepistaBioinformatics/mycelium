use crate::dtos::MyceliumProfileData;

use crate::models::active_backend_modules::SqlAppModule;
use actix_web::{get, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        dtos::{
            account_type::AccountType,
            related_accounts::RelatedAccounts,
            resource_audit_log::{ResourceAuditLog, ResourceAuditResourceType},
        },
        entities::AccountFetching,
    },
    use_cases::shared::audit::fetch_resource_audit_trail,
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        fetch_many_response_kind, handle_mapped_error,
    },
};
use mycelium_base::entities::FetchResponseKind;
use serde::Deserialize;
use shaku::HasComponent;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(fetch_resource_audit_trail_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct FetchResourceAuditTrailParams {
    /// The coarse category of the audited resource.
    resource_type: ResourceAuditResourceType,

    /// The audited resource's own identifier.
    resource_id: Uuid,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
//
// Resource audit trail
//
// This endpoint is deliberately not `role_scoped`: staff, tenant
// owners/managers, and personal-account owners all call the same route --
// the permission branching happens inside `fetch_resource_audit_trail`, not
// via separate role-scoped copies. See
// `.claude/specs/features/audit-log/design.md`.
//
// ? ---------------------------------------------------------------------------

/// Fetch the audit trail of a resource
///
/// Returns the immutable audit trail for a single resource, newest first.
/// Staff can read any resource's trail; tenant owners/managers can read
/// resources belonging to their own tenant; personal-account owners can read
/// their own account's trail. Every other caller is denied.
#[utoipa::path(
    get,
    operation_id = "fetch_resource_audit_trail",
    params(FetchResourceAuditTrailParams),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = HttpJsonResponse,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 400,
            description = "Invalid resource reference.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "No audit rows for this resource.",
        ),
        (
            status = 200,
            description = "Audit trail fetched, newest first.",
            body = [ResourceAuditLog],
        ),
    ),
)]
#[get("")]
pub async fn fetch_resource_audit_trail_url(
    query: web::Query<FetchResourceAuditTrailParams>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let (tenant_id, resource_owner_account_id) = match resolve_audit_context(
        &query.resource_type,
        query.resource_id,
        &app_module,
    )
    .await
    {
        Ok(context) => context,
        Err(response) => return response,
    };

    match fetch_resource_audit_trail(
        profile.to_profile(),
        query.resource_type.to_owned(),
        query.resource_id,
        tenant_id,
        resource_owner_account_id,
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => fetch_many_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

// ? ---------------------------------------------------------------------------
// ? Private helpers
// ? ---------------------------------------------------------------------------

/// Resolve `tenant_id`/`resource_owner_account_id` for the permission check.
///
/// Only `Account` and `Tenant` have a natural, already-known owner/tenant --
/// for a `Tenant` resource, `resource_id` already is the tenant's own id
/// (see design.md's tenant lane). For every other resource type the pair
/// stays `(None, None)`, which `fetch_resource_audit_trail` correctly treats
/// as "no tenant or personal standing to check", falling through to
/// staff-only.
async fn resolve_audit_context(
    resource_type: &ResourceAuditResourceType,
    resource_id: Uuid,
    app_module: &web::Data<SqlAppModule>,
) -> Result<(Option<Uuid>, Option<Uuid>), HttpResponse> {
    match resource_type {
        ResourceAuditResourceType::Tenant => Ok((Some(resource_id), None)),
        ResourceAuditResourceType::Account => {
            resolve_account_audit_context(resource_id, app_module).await
        }
        _ => Ok((None, None)),
    }
}

/// Look up the account's real `tenant_id` (personal accounts have none) so
/// the permission check runs against the account's actual standing instead
/// of caller-supplied, unverified values. An account owns itself, so
/// `resource_owner_account_id` is always `resource_id` here.
///
/// `RelatedAccounts::HasStaffPrivileges` bypasses the repository's
/// account-visibility filter (see
/// `AccountFetchingSqlDbRepository::get`'s `_ => ()` arm): this is an
/// internal lookup used only to build inputs for the real permission check
/// performed afterwards by `fetch_resource_audit_trail`, not a value
/// returned to the caller.
async fn resolve_account_audit_context(
    account_id: Uuid,
    app_module: &web::Data<SqlAppModule>,
) -> Result<(Option<Uuid>, Option<Uuid>), HttpResponse> {
    let account_fetching_repo: &dyn AccountFetching = app_module.resolve_ref();

    let account = match account_fetching_repo
        .get(account_id, RelatedAccounts::HasStaffPrivileges)
        .await
    {
        Ok(FetchResponseKind::Found(account)) => account,
        Ok(FetchResponseKind::NotFound(_)) => {
            return Err(HttpResponse::BadRequest()
                .json(HttpJsonResponse::new_message("Invalid account ID")))
        }
        Err(err) => return Err(handle_mapped_error(err)),
    };

    Ok((account_tenant_id(&account.account_type), Some(account_id)))
}

/// Extract the tenant a tenant-dependent account belongs to.
///
/// Personal accounts (`Staff`/`Manager`/`User`) and actor-associated
/// accounts have no tenant; only the tenant-dependent variants embed one
/// (see `AccountType::is_tenant_dependent`).
fn account_tenant_id(account_type: &AccountType) -> Option<Uuid> {
    match account_type {
        AccountType::Subscription { tenant_id } => Some(*tenant_id),
        AccountType::RoleAssociated { tenant_id, .. } => Some(*tenant_id),
        AccountType::TenantManager { tenant_id } => Some(*tenant_id),
        _ => None,
    }
}
