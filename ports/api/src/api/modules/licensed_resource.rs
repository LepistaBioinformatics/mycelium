use myc_prisma::repositories::LicensedResourcesFetchingSqlDbRepository;
use shaku::module;

module! {
    pub LicensedResourcesFetchingModule {
        components = [LicensedResourcesFetchingSqlDbRepository],
        providers = [],
    }
}
