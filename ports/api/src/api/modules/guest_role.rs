use myc_prisma::repositories::{
    GuestRoleDeletionSqlDbRepository, GuestRoleFetchingSqlDbRepository,
    GuestRoleRegistrationSqlDbRepository, GuestRoleUpdatingSqlDbRepository,
};
use shaku::module;

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
