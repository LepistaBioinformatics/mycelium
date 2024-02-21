use myc_prisma::repositories::{
    UserDeletionSqlDbRepository, UserFetchingSqlDbRepository,
    UserRegistrationSqlDbRepository, UserUpdatingSqlDbRepository,
};
use shaku::module;

module! {
    pub UserRegistrationModule {
        components = [UserRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub UserUpdatingModule {
        components = [UserUpdatingSqlDbRepository],
        providers = []
    }
}

module! {
    pub UserDeletionModule {
        components = [UserDeletionSqlDbRepository],
        providers = []
    }
}

module! {
    pub UserFetchingModule {
        components = [UserFetchingSqlDbRepository],
        providers = []
    }
}
