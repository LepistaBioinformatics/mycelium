use myc_prisma::repositories::service::profile_fetching::ProfileFetchingSqlDbRepository;
use shaku::module;

module! {
    pub ProfileFetchingModule {
        components = [ProfileFetchingSqlDbRepository],
        providers = []
    }
}
