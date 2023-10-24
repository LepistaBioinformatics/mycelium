use myc_prisma::repositories::{
    RoleDeletionSqlDbRepository, RoleFetchingSqlDbRepository,
    RoleRegistrationSqlDbRepository, RoleUpdatingSqlDbRepository,
};
use shaku::module;

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
