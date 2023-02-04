use clap::Parser;
use log::{info, warn};
use myc_core::use_cases::gateway::routes::load_config_from_json;

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub(crate) struct Arguments {
    /// The filesystem path of the file to parse.
    path: String,

    /// Print the loaded database as JSON if True.
    #[clap(long, short, action)]
    print: Option<bool>,
}

pub(crate) async fn check_config_from_json_cmd(args: Arguments) {
    match load_config_from_json(args.path).await {
        Err(err) => warn!("Invalid database: {}", err),
        Ok(res) => {
            info!("Database is valid.");

            match args.print {
                None => (),
                Some(opt) => match opt {
                    true => println!(
                        "{}",
                        serde_json::to_string_pretty(&res).unwrap()
                    ),
                    _ => (),
                },
            }
        }
    };
}
