use clap::Parser;
use clean_base::entities::GetOrCreateResponseKind;
use log::{debug, error, info};
use myc_core::use_cases::roles::staff::account::create_seed_staff_account;
use myc_prisma::repositories::{
    connector::generate_prisma_client_of_thread,
    AccountRegistrationSqlDbRepository, AccountTypeRegistrationSqlDbRepository,
};
use std::process::id as process_id;

#[derive(Parser, Debug)]
pub(crate) struct Arguments {
    #[clap(subcommand)]
    pub create_seed_account: Commands,
}

#[derive(Parser, Debug)]
pub(crate) enum Commands {
    CreateSeedAccount(CreateSeedAccountArguments),
}

#[derive(Parser, Debug)]
pub(crate) struct CreateSeedAccountArguments {
    email: String,
    account_name: String,
    first_name: String,
    last_name: String,
}

pub(crate) async fn create_seed_staff_account_cmd(
    args: CreateSeedAccountArguments,
) {
    debug!("Start the database connectors");
    generate_prisma_client_of_thread(process_id()).await;

    match create_seed_staff_account(
        args.email.to_owned(),
        args.account_name.to_owned(),
        args.first_name.to_owned(),
        args.last_name.to_owned(),
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
                info!(
                    "\n
Seed staff account successfully created:
- Email: {}
- First Name: {}
- Last Name: {}
- Account Name: {}
    ",
                    args.email, args.first_name, args.last_name, account.name,
                );
            }
        },
    };
}
