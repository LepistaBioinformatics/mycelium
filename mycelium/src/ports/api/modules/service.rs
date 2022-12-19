extern crate myc;

use myc::adapters::repositories::sql_db::service::profile_fetching::ProfileFetchingSqlDbRepository;
use shaku::module;

module! {
    pub ProfileFetchingModule {
        components = [ProfileFetchingSqlDbRepository],
        providers = []
    }
}
