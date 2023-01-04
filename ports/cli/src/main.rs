extern crate myc_core;

use clap::Parser;
use clean_base::dtos::enums::ParentEnum;
use clean_base::entities::default_response::GetOrCreateResponseKind;
use log::{debug, error, info};
use myc_core::use_cases::staff::account::create_seed_staff_account;
use myc_prisma::repositories::{
    connector::generate_prisma_client_of_thread,
    AccountRegistrationSqlDbRepository, AccountTypeRegistrationSqlDbRepository,
    UserRegistrationSqlDbRepository,
};
use std::process::id as process_id;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CliArguments {
    email: String,
    account_name: String,
    first_name: String,
    last_name: String,
}

#[tokio::main]
pub async fn main() {
    debug!("Building logger");
    env_logger::init();

    debug!("Start the database connectors");
    generate_prisma_client_of_thread(process_id()).await;

    debug!("Build CLI arguments");
    let args = CliArguments::parse();

    debug!("Start seed staff account creation");
    match create_seed_staff_account(
        args.email,
        args.account_name,
        args.first_name,
        args.last_name,
        Box::new(&UserRegistrationSqlDbRepository {}),
        Box::new(&AccountTypeRegistrationSqlDbRepository {}),
        Box::new(&AccountRegistrationSqlDbRepository {}),
    )
    .await
    {
        Err(err) => error!("{err}"),
        Ok(res) => match res {
            GetOrCreateResponseKind::NotCreated(_, msg) => {
                info!("Seed staff account already exists created: {:?}", msg)
            }
            GetOrCreateResponseKind::Created(account) => {
                match account.owner {
                    ParentEnum::Id(id) => info!(
                        "Seed staff account successfully created: {:?}",
                        id
                    ),
                    ParentEnum::Record(record) => {
                        info!(
                            "\n
Seed staff account successfully created:
  - Email: {}
  - First Name: {}
  - Last Name: {}
  - Account Name: {}
            ",
                            record.email.get_email(),
                            record.first_name.unwrap(),
                            record.last_name.unwrap(),
                            account.name,
                        )
                    }
                };
            }
        },
    };
}
