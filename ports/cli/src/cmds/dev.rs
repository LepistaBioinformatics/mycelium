use clap::Parser;
use log::{info, warn};
use myc_core::{
    settings::init_bearer_secret,
    use_cases::gateway::token::generate_bearer_token_from_email,
};

#[derive(Parser, Debug)]
pub(crate) struct Arguments {
    #[clap(subcommand)]
    pub generate_token: Commands,
}

#[derive(Parser, Debug)]
pub(crate) enum Commands {
    GenerateToken(BearerGenerationArguments),
}

#[derive(Parser, Debug)]
pub(crate) struct BearerGenerationArguments {
    email: String,
}

pub(crate) async fn generate_bearer_token_from_email_cmd(
    args: BearerGenerationArguments,
) {
    warn!("This is a development feature only. Does not use to generate production tokens.");

    init_bearer_secret().await;

    match generate_bearer_token_from_email(args.email).await {
        Err(err) => warn!("Unexpected error: {}", err),
        Ok(res) => info!("Token generated:\nBearer {}", res),
    };
}
