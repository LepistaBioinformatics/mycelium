use myc_redis::repositories::{
    SessionTokenDeletionRedisDbRepository,
    SessionTokenFetchingRedisDbRepository,
    SessionTokenRegistrationRedisDbRepository,
    SessionTokenUpdatingRedisDbRepository,
};
use shaku::module;

module! {
    pub SessionTokenRegistrationModule {
        components = [SessionTokenRegistrationRedisDbRepository],
        providers = []
    }
}

module! {
    pub SessionTokenDeletionModule {
        components = [SessionTokenDeletionRedisDbRepository],
        providers = []
    }
}

module! {
    pub SessionTokenFetchingModule {
        components = [SessionTokenFetchingRedisDbRepository],
        providers = []
    }
}

module! {
    pub SessionTokenUpdatingModule {
        components = [SessionTokenUpdatingRedisDbRepository],
        providers = []
    }
}
