use myc_core::adapters::smtp::message_sending::MessageSendingSqlDbRepository;
use myc_prisma::repositories::{
    manager::guest_user_registration::GuestUserRegistrationSqlDbRepository,
    shared::account_fetching::AccountFetchingSqlDbRepository,
};
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
