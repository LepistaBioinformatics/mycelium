use myc_prisma::repositories::{
    account_registration::AccountRegistrationSqlDbRepository,
    account_type_registration::AccountTypeRegistrationSqlDbRepository,
    user_registration::UserRegistrationSqlDbRepository,
};
use shaku::module;

module! {
    pub AccountRegistrationModule {
        components = [AccountRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub AccountTypeRegistrationModule {
        components = [AccountTypeRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub UserRegistrationModule {
        components = [UserRegistrationSqlDbRepository],
        providers = []
    }
}
