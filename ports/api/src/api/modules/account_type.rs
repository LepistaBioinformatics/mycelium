use myc_prisma::repositories::{
    AccountTypeDeletionSqlDbRepository, AccountTypeRegistrationSqlDbRepository,
};
use shaku::module;

module! {
    pub AccountTypeRegistrationModule {
        components = [AccountTypeRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub AccountTypeDeletionModule {
        components = [AccountTypeDeletionSqlDbRepository],
        providers = []
    }
}
