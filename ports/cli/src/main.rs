mod cmds;
mod functions;

use clap::Parser;
use cmds::{accounts, error_codes};
use std::env::set_var;

#[derive(Parser, Debug)]
enum Cli {
    /// Create a seed staff account
    Accounts(accounts::Arguments),

    /// Register native error codes
    NativeErrors(error_codes::Arguments),
}

#[tokio::main]
pub async fn main() {
    unsafe {
        set_var("RUST_LOG", "info");
    }

    env_logger::init();

    let args = Cli::parse();

    match args {
        Cli::Accounts(sub_args) => match sub_args.create_seed_account {
            accounts::Commands::CreateSeedAccount(args) => {
                accounts::create_seed_staff_account_cmd(args).await
            }
        },
        Cli::NativeErrors(sub_args) => match sub_args.init_native_error_codes {
            error_codes::Commands::Init => {
                error_codes::batch_register_native_error_codes_cmd().await
            }
        },
    }
}
