use myc_prisma::repositories::{
    account_fetching::AccountFetchingSqlDbRepository,
    guest_user_registration::GuestUserRegistrationSqlDbRepository,
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
