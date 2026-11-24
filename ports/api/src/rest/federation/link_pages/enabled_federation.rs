use crate::{
    federation::PreparedFederation,
    models::active_backend_modules::SqlAppModule,
};

use actix_web::web;
use myc_core::{
    domain::entities::InstanceSettingsFetching,
    use_cases::federation::is_federation_enabled,
};
use shaku::HasComponent;

/// The prepared federation while it is present and switched on; linking
/// pages answer 404 otherwise, as every other federation surface
pub(super) async fn enabled_federation<'a>(
    prepared: &'a web::Data<Option<PreparedFederation>>,
    sql_module: &web::Data<SqlAppModule>,
) -> Option<&'a PreparedFederation> {
    let prepared = prepared.get_ref().as_ref()?;
    let settings: &dyn InstanceSettingsFetching = sql_module.resolve_ref();

    match is_federation_enabled(Box::new(settings)).await {
        Ok(true) => Some(prepared),
        _ => None,
    }
}
