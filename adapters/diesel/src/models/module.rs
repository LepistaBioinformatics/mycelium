use super::config::PostgresConfig;

use shaku::module;

module! {
    pub DatabaseModule {
        components = [PostgresConfig],
        providers = []
    }
}
