use myc_prisma::repositories::{
    TenantDeletionSqlDbRepository, TenantFetchingSqlDbRepository,
    TenantRegistrationSqlDbRepository, TenantUpdatingSqlDbRepository,
};

use shaku::module;

module! {
    pub TenantDeletionModule {
        components = [TenantDeletionSqlDbRepository],
        providers = []
    }
}

module! {
    pub TenantFetchingModule {
        components = [TenantFetchingSqlDbRepository],
        providers = []
    }
}

module! {
    pub TenantRegistrationModule {
        components = [TenantRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub TenantUpdatingModule {
        components = [TenantUpdatingSqlDbRepository],
        providers = []
    }
}
