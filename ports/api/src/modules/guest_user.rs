use myc_prisma::repositories::{
    GuestUserDeletionSqlDbRepository, GuestUserFetchingSqlDbRepository,
    GuestUserOnAccountUpdatingSqlDbRepository,
    GuestUserRegistrationSqlDbRepository,
};
use shaku::module;

module! {
    pub GuestUserFetchingModule {
        components = [GuestUserFetchingSqlDbRepository],
        providers = []
    }
}

module! {
    pub GuestUserRegistrationModule {
        components = [GuestUserRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub GuestUserOnAccountUpdatingModule {
        components = [GuestUserOnAccountUpdatingSqlDbRepository],
        providers = []
    }
}

module! {
    pub GuestUserDeletionModule {
        components = [GuestUserDeletionSqlDbRepository],
        providers = []
    }
}
