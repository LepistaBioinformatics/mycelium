use myc_prisma::repositories::{
    manager::guest_user_registration::GuestUserRegistrationSqlDbRepository,
    shared::account_fetching::AccountFetchingSqlDbRepository,
};
use myc_smtp::repositories::message_sending::MessageSendingSqlDbRepository;
use shaku::module;

module! {
    pub AccountFetchingModule {
        components = [AccountFetchingSqlDbRepository],
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
    pub MessageSendingModule {
        components = [MessageSendingSqlDbRepository],
        providers = []
    }
}
