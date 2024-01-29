use myc_prisma::repositories::{
    TagDeletionSqlDbRepository, TagRegistrationSqlDbRepository,
    TagUpdatingSqlDbRepository,
};

use shaku::module;

module! {
    pub TagRegistrationModule {
        components = [TagRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub TagDeletionModule {
        components = [TagDeletionSqlDbRepository],
        providers = []
    }
}

module! {
    pub TagUpdatingModule {
        components = [TagUpdatingSqlDbRepository],
        providers = []
    }
}
