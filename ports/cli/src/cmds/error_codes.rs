use clap::Parser;
use log::{error, info};
use myc_core::{
    domain::entities::ErrorCodeRegistration,
    use_cases::role_scoped::system_manager::error_codes::batch_register_native_error_codes,
};
use myc_diesel::repositories::{
    AppModule, DieselDbPoolProvider, DieselDbPoolProviderParameters,
};
use shaku::HasComponent;
use std::sync::Arc;

use crate::functions::try_to_resolve_database_url;

#[derive(Parser, Debug)]
pub(crate) struct Arguments {
    #[clap(subcommand)]
    pub init_native_error_codes: Commands,
}

#[derive(Parser, Debug)]
pub(crate) enum Commands {
    Init,
}

pub(crate) async fn batch_register_native_error_codes_cmd() {
    //
    // Ask for the database url
    //
    let database_url = try_to_resolve_database_url();

    //
    // Initialize the dependency
    //
    let module = Arc::new(
        AppModule::builder()
            .with_component_parameters::<DieselDbPoolProvider>(
                DieselDbPoolProviderParameters {
                    pool: DieselDbPoolProvider::new(&database_url.as_str()),
                },
            )
            .build(),
    );

    let repo: &dyn ErrorCodeRegistration = module.resolve_ref();

    //
    // Batch register the native error codes
    //
    match batch_register_native_error_codes(Box::new(repo)).await {
        Err(err) => error!("{err}"),
        Ok(res) => {
            if res.unpersisted_errors.len() > 0 {
                info!("Native error codes not registered:");

                for error_code in res.unpersisted_errors {
                    info!(
                        "{:?}: {:?}",
                        error_code.error_number, error_code.message
                    );
                }
            }

            info!("{} native error codes registered", res.persisted_errors);
        }
    };
}
