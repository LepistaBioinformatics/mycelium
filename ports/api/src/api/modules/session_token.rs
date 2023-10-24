use myc_prisma::repositories::{
    SessionTokenDeletionSqlDbRepository, SessionTokenFetchingSqlDbRepository,
    SessionTokenRegistrationSqlDbRepository,
};

use shaku::module;

module! {
    pub SessionTokenRegistrationModule {
        components = [SessionTokenRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub SessionTokenDeletionModule {
        components = [SessionTokenDeletionSqlDbRepository],
        providers = []
    }
}

module! {
    pub SessionTokenFetchingModule {
        components = [SessionTokenFetchingSqlDbRepository],
        providers = []
    }
}
