use myc_prisma::repositories::{
    WebHookDeletionSqlDbRepository, WebHookFetchingSqlDbRepository,
    WebHookRegistrationSqlDbRepository, WebHookUpdatingSqlDbRepository,
};
use shaku::module;

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
