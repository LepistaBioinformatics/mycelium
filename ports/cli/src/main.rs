mod cmds;
mod functions;

use clap::Parser;
use cmds::{accounts, error_codes, migrate_dek, rotate_kek};
use std::env::set_var;

#[derive(Parser, Debug)]
enum Cli {
    /// Create a seed staff account
    Accounts(accounts::Arguments),

    /// Register native error codes
    NativeErrors(error_codes::Arguments),

    /// Migrate v1 encrypted fields to v2 envelope encryption format
    Encryption(migrate_dek::Arguments),

    /// Rewrap every tenant's DEK under a new KEK (without touching
    /// user-data ciphertexts or invalidating connection strings).
    Kek(rotate_kek::Arguments),
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
        Cli::Encryption(sub_args) => match sub_args.cmd {
            migrate_dek::Commands::MigrateDek(args) => {
                migrate_dek::migrate_dek_cmd(args).await
            }
        },
        Cli::Kek(sub_args) => match sub_args.cmd {
            rotate_kek::Commands::RotateKek(args) => {
                rotate_kek::rotate_kek_cmd(args).await
            }
        },
    }
}
