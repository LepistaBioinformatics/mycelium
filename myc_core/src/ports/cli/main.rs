use clap::Parser;
use std::env::set_var;

#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CliArguments {
    /// The filesystem path of the file to parse.
    path: String,

    /// Print the loaded database as JSON if True.
    #[clap(long, short, action)]
    print: Option<bool>,
}

#[tokio::main]
pub async fn main() {
    // Build logger
    set_var("RUST_LOG", "debug");
    env_logger::init();

    // Build cli
    let args = CliArguments::parse();
    println!("args: {:?}", args);

    // Try to insert user into database
    //create_staff_account;
}
