use myc_prisma::repositories::{
    TokenInvalidationSqlDbRepository, TokenRegistrationSqlDbRepository,
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
