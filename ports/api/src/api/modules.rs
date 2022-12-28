use myc_prisma::repositories::{
    AccountFetchingSqlDbRepository, AccountRegistrationSqlDbRepository,
    AccountTypeRegistrationSqlDbRepository,
    GuestUserRegistrationSqlDbRepository, ProfileFetchingSqlDbRepository,
    UserRegistrationSqlDbRepository,
};
use myc_smtp::repositories::MessageSendingSqlDbRepository;
use shaku::module;

// ? ---------------------------------------------------------------------------
// ? Account
// ? ---------------------------------------------------------------------------

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

// ? ---------------------------------------------------------------------------
// ? Account Type
// ? ---------------------------------------------------------------------------

module! {
    pub AccountTypeRegistrationModule {
        components = [AccountTypeRegistrationSqlDbRepository],
        providers = []
    }
}

// ? ---------------------------------------------------------------------------
// ? User
// ? ---------------------------------------------------------------------------

module! {
    pub UserRegistrationModule {
        components = [UserRegistrationSqlDbRepository],
        providers = []
    }
}

// ? ---------------------------------------------------------------------------
// ? Guest User
// ? ---------------------------------------------------------------------------

module! {
    pub GuestUserRegistrationModule {
        components = [GuestUserRegistrationSqlDbRepository],
        providers = []
    }
}

// ? ---------------------------------------------------------------------------
// ? Message
// ? ---------------------------------------------------------------------------

module! {
    pub MessageSendingModule {
        components = [MessageSendingSqlDbRepository],
        providers = []
    }
}

// ? ---------------------------------------------------------------------------
// ? Profile
// ? ---------------------------------------------------------------------------

module! {
    pub ProfileFetchingModule {
        components = [ProfileFetchingSqlDbRepository],
        providers = []
    }
}
