use myc_core::{
    domain::{
        dtos::federation::RoleCatalogMode, entities::InstanceSettingsFetching,
    },
    use_cases::federation::read_role_catalog_mode,
};

/// Whether the self-description advertises the role catalog
pub(crate) async fn role_catalog_published(
    settings: &dyn InstanceSettingsFetching,
) -> bool {
    match read_role_catalog_mode(Box::new(settings)).await {
        Ok(mode) => mode != RoleCatalogMode::Off,
        Err(err) => {
            tracing::error!(%err, "role catalog mode unreadable");
            false
        }
    }
}
