use myc_prisma::repositories::{
    TokenFetchingSqlDbRepository, TokenRegistrationSqlDbRepository,
};

use shaku::module;

module! {
    pub TokenRegistrationModule {
        components = [TokenRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub TokenFetchingModule {
        components = [TokenFetchingSqlDbRepository],
        providers = []
    }
}
