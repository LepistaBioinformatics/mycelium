mod client;

mod profile_fetching;
mod telegram_config;

pub use profile_fetching::{
    ProfileFetchingSvcRepo, ProfileFetchingSvcRepoParameters,
};
pub use telegram_config::TelegramConfigSvcRepo;
