use crate::functions::try_to_resolve_database_url;

use clap::Parser;
use myc_core::{
    domain::entities::{AccountRegistration, UserRegistration},
    use_cases::super_users::staff::account::create_seed_staff_account,
};
use myc_diesel::repositories::{
    DieselDbPoolProvider, DieselDbPoolProviderParameters, SqlAppModule,
};
use mycelium_base::entities::GetOrCreateResponseKind;
use shaku::HasComponent;
use std::sync::Arc;

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
    //
    // Ask for the database url
    //
    let database_url = try_to_resolve_database_url();

    //
    // Ask for the password
    //
    let password =
        rpassword::prompt_password("Your password: ".to_string()).unwrap();

    //
    // Initialize the dependency
    //
    let module = Arc::new(
        SqlAppModule::builder()
            .with_component_parameters::<DieselDbPoolProvider>(
                DieselDbPoolProviderParameters {
                    pool: DieselDbPoolProvider::new(&database_url.as_str()),
                },
            )
            .build(),
    );

    let user_repo: &dyn UserRegistration = module.resolve_ref();
    let account_repo: &dyn AccountRegistration = module.resolve_ref();

    //
    // Create the seed staff account
    //
    match create_seed_staff_account(
        args.email.to_owned(),
        args.account_name.to_owned(),
        args.first_name.to_owned(),
        args.last_name.to_owned(),
        password,
        Box::new(user_repo),
        Box::new(account_repo),
    )
    .await
    {
        Err(err) => tracing::error!("{err}"),
        Ok(res) => match res {
            GetOrCreateResponseKind::NotCreated(_, msg) => {
                tracing::info!(
                    "Seed staff account already exists created: {:?}",
                    msg
                )
            }
            GetOrCreateResponseKind::Created(account) => {
                tracing::info!(
                    "\n
    Seed staff account successfully created:
    - Email: {}
    - First Name: {}
    - Last Name: {}
    - Account Name: {}
        ",
                    args.email,
                    args.first_name,
                    args.last_name,
                    account.name,
                );
            }
        },
    };
}
