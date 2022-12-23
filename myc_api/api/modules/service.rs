use myc_core::adapters::repositories::sql_db::service::profile_fetching::ProfileFetchingSqlDbRepository;
use shaku::module;

module! {
    pub ProfileFetchingModule {
        components = [ProfileFetchingSqlDbRepository],
        providers = []
    }
}
