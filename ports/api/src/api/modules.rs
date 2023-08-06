use myc_mem_db::repositories::RoutesFetchingMemDbRepo;
use myc_prisma::repositories::{
    AccountFetchingSqlDbRepository, AccountRegistrationSqlDbRepository,
    AccountTypeDeletionSqlDbRepository, AccountTypeRegistrationSqlDbRepository,
    AccountUpdatingSqlDbRepository, ErrorCodeDeletionDeletionSqlDbRepository,
    ErrorCodeFetchingSqlDbRepository, ErrorCodeRegistrationSqlDbRepository,
    ErrorCodeUpdatingSqlDbRepository, GuestRoleDeletionSqlDbRepository,
    GuestRoleFetchingSqlDbRepository, GuestRoleRegistrationSqlDbRepository,
    GuestRoleUpdatingSqlDbRepository, GuestUserDeletionSqlDbRepository,
    GuestUserFetchingSqlDbRepository,
    GuestUserOnAccountUpdatingSqlDbRepository,
    GuestUserRegistrationSqlDbRepository,
    LicensedResourcesFetchingSqlDbRepository, ProfileFetchingSqlDbRepository,
    RoleDeletionSqlDbRepository, RoleFetchingSqlDbRepository,
    RoleRegistrationSqlDbRepository, RoleUpdatingSqlDbRepository,
    UserRegistrationSqlDbRepository, UserUpdatingSqlDbRepository,
    WebHookDeletionSqlDbRepository, WebHookFetchingSqlDbRepository,
    WebHookRegistrationSqlDbRepository, WebHookUpdatingSqlDbRepository,
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
// ? Guest User
// ? ---------------------------------------------------------------------------

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
    pub GuestRoleRegistrationModule {
        components = [GuestRoleRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub GuestRoleDeletionModule {
        components = [GuestRoleDeletionSqlDbRepository],
        providers = []
    }
}

module! {
    pub GuestRoleUpdatingModule {
        components = [GuestRoleUpdatingSqlDbRepository],
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

// ? ---------------------------------------------------------------------------
// ? Role
// ? ---------------------------------------------------------------------------

module! {
    pub RoleRegistrationModule {
        components = [RoleRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub RoleFetchingModule {
        components = [RoleFetchingSqlDbRepository],
        providers = []
    }
}

module! {
    pub RoleUpdatingModule {
        components = [RoleUpdatingSqlDbRepository],
        providers = []
    }
}

module! {
    pub RoleDeletionModule {
        components = [RoleDeletionSqlDbRepository],
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

module! {
    pub UserUpdatingModule {
        components = [UserUpdatingSqlDbRepository],
        providers = []
    }
}

// ? ---------------------------------------------------------------------------
// ? LicensedResources
// ? ---------------------------------------------------------------------------

module! {
    pub LicensedResourcesFetchingModule {
        components = [LicensedResourcesFetchingSqlDbRepository],
        providers = [],
    }
}

// ? ---------------------------------------------------------------------------
// ? Routes
// ? ---------------------------------------------------------------------------

module! {
    pub RoutesFetchingModule {
        components = [RoutesFetchingMemDbRepo],
        providers = []
    }
}

// ? ---------------------------------------------------------------------------
// ? Error Codes
// ? ---------------------------------------------------------------------------

module! {
    pub ErrorCodeDeletionModule {
        components = [ErrorCodeDeletionDeletionSqlDbRepository],
        providers = []
    }
}

module! {
    pub ErrorCodeFetchingModule {
        components = [ErrorCodeFetchingSqlDbRepository],
        providers = []
    }
}

module! {
    pub ErrorCodeRegistrationModule {
        components = [ErrorCodeRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub ErrorCodeUpdatingModule {
        components = [ErrorCodeUpdatingSqlDbRepository],
        providers = []
    }
}

// ? ---------------------------------------------------------------------------
// ? WebHooks
// ? ---------------------------------------------------------------------------

module! {
    pub WebHookRegistrationModule {
        components = [WebHookRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub WebHookFetchingModule {
        components = [WebHookFetchingSqlDbRepository],
        providers = []
    }
}

module! {
    pub WebHookUpdatingModule {
        components = [WebHookUpdatingSqlDbRepository],
        providers = []
    }
}

module! {
    pub WebHookDeletionModule {
        components = [WebHookDeletionSqlDbRepository],
        providers = []
    }
}
