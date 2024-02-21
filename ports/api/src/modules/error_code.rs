use myc_prisma::repositories::{
    ErrorCodeDeletionDeletionSqlDbRepository, ErrorCodeFetchingSqlDbRepository,
    ErrorCodeRegistrationSqlDbRepository, ErrorCodeUpdatingSqlDbRepository,
};
use shaku::module;

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
