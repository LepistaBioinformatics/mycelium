use clap::Parser;
use log::{debug, error, info};
use myc_core::use_cases::roles::super_users::staff::account::create_seed_staff_account;
use myc_prisma::repositories::{
    connector::generate_prisma_client_of_thread,
    AccountRegistrationSqlDbRepository, UserRegistrationSqlDbRepository,
};
use mycelium_base::entities::GetOrCreateResponseKind;
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

    let password =
        rpassword::prompt_password("Your password: ".to_string()).unwrap();

    match create_seed_staff_account(
        args.email.to_owned(),
        args.account_name.to_owned(),
        args.first_name.to_owned(),
        args.last_name.to_owned(),
        password,
        Box::new(&UserRegistrationSqlDbRepository {}),
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
