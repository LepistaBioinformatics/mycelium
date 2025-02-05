use clap::Parser;
use myc_core::use_cases::gateway::routes::load_config_from_yaml;

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
    match load_config_from_yaml(args.path).await {
        Err(err) => tracing::warn!("Invalid database: {}", err),
        Ok(res) => {
            tracing::info!("Database is valid.");

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
