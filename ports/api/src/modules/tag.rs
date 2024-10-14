use myc_prisma::repositories::{
    AccountTagDeletionSqlDbRepository, AccountTagRegistrationSqlDbRepository,
    AccountTagUpdatingSqlDbRepository,
};

use shaku::module;

module! {
    pub AccountTagRegistrationModule {
        components = [AccountTagRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub AccountTagDeletionModule {
        components = [AccountTagDeletionSqlDbRepository],
        providers = []
    }
}

module! {
    pub AccountTagUpdatingModule {
        components = [AccountTagUpdatingSqlDbRepository],
        providers = []
    }
}
