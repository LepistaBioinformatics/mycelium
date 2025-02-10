use shaku::module;

mod config;
mod redis;
mod redis_config;

pub use config::*;
pub use redis::RedisClientWrapper;
pub use redis_config::*;

module! {
    pub SharedAppModule {
        components = [SharedClientImpl],
        providers = []
    }
}
