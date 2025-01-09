use myc_prisma::repositories::{
    TokenFetchingSqlDbRepository, TokenInvalidationSqlDbRepository,
    TokenRegistrationSqlDbRepository,
};

use shaku::module;

module! {
    pub TokenRegistrationModule {
        components = [TokenRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub TokenInvalidationModule {
        components = [TokenInvalidationSqlDbRepository],
        providers = []
    }
}

module! {
    pub TokenFetchingModule {
        components = [TokenFetchingSqlDbRepository],
        providers = []
    }
}
