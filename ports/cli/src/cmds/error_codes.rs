use clap::Parser;
use log::{error, info};
use myc_core::use_cases::roles::standard::system_manager::error_codes::batch_register_native_error_codes;
use myc_prisma::repositories::ErrorCodeRegistrationSqlDbRepository;

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
    match batch_register_native_error_codes(Box::new(
        &ErrorCodeRegistrationSqlDbRepository {},
    ))
    .await
    {
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
