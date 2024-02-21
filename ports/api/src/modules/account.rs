use myc_prisma::repositories::{
    AccountFetchingSqlDbRepository, AccountRegistrationSqlDbRepository,
    AccountUpdatingSqlDbRepository,
};
use shaku::module;

module! {
    pub AccountRegistrationModule {
        components = [AccountRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub AccountFetchingModule {
        components = [AccountFetchingSqlDbRepository],
        providers = []
    }
}

module! {
    pub AccountUpdatingModule {
        components = [AccountUpdatingSqlDbRepository],
        providers = []
    }
}
