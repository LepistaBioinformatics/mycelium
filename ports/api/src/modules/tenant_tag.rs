use myc_prisma::repositories::{
    TenantTagDeletionSqlDbRepository, TenantTagRegistrationSqlDbRepository,
    TenantTagUpdatingSqlDbRepository,
};

use shaku::module;

module! {
    pub TenantTagRegistrationModule {
        components = [TenantTagRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub TenantTagDeletionModule {
        components = [TenantTagDeletionSqlDbRepository],
        providers = []
    }
}

module! {
    pub TenantTagUpdatingModule {
        components = [TenantTagUpdatingSqlDbRepository],
        providers = []
    }
}
