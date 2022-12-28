use myc_prisma::repositories::{
    AccountFetchingSqlDbRepository, AccountRegistrationSqlDbRepository,
    AccountTypeDeletionSqlDbRepository, AccountTypeRegistrationSqlDbRepository,
    AccountUpdatingSqlDbRepository, GuestRoleDeletionSqlDbRepository,
    GuestRoleFetchingSqlDbRepository, GuestUserRegistrationSqlDbRepository,
    ProfileFetchingSqlDbRepository, UserRegistrationSqlDbRepository,
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

module! {
    pub AccountUpdatingModule {
        components = [AccountUpdatingSqlDbRepository],
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

module! {
    pub AccountTypeDeletionModule {
        components = [AccountTypeDeletionSqlDbRepository],
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
// ? Guest Role
// ? ---------------------------------------------------------------------------

module! {
    pub GuestRoleFetchingModule {
        components = [GuestRoleFetchingSqlDbRepository],
        providers = []
    }
}

module! {
    pub GuestRoleDeletionModule {
        components = [GuestRoleDeletionSqlDbRepository],
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
