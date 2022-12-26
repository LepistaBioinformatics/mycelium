use myc_prisma::repositories::{
    default_users::user_registration::UserRegistrationSqlDbRepository,
    manager::account_type_registration::AccountTypeRegistrationSqlDbRepository,
    shared::account_registration::AccountRegistrationSqlDbRepository,
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
