extern crate myc_core;

mod cmds;

use clap::Parser;
use cmds::{accounts, check, error_codes};
use std::env::set_var;

#[derive(Parser, Debug)]
enum Cli {
    Check(check::Arguments),
    Accounts(accounts::Arguments),
    NativeErrors(error_codes::Arguments),
}

#[tokio::main]
pub async fn main() {
    set_var("RUST_LOG", "info");
    env_logger::init();

    let args = Cli::parse();

    match args {
        Cli::Accounts(sub_args) => match sub_args.create_seed_account {
            accounts::Commands::CreateSeedAccount(args) => {
                accounts::create_seed_staff_account_cmd(args).await
            }
        },
        Cli::Check(sub_args) => {
            check::check_config_from_json_cmd(sub_args).await
        }
        Cli::NativeErrors(sub_args) => match sub_args.init_native_error_codes {
            error_codes::Commands::Init => {
                error_codes::batch_register_native_error_codes_cmd().await
            }
        },
    }
}
