use myc_prisma::repositories::ProfileFetchingSqlDbRepository;
use shaku::module;

module! {
    pub ProfileFetchingModule {
        components = [ProfileFetchingSqlDbRepository],
        providers = []
    }
}
