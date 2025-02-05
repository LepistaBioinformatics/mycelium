use shaku::module;

mod config;
mod message_sending_queue;
mod message_sending_smtp;

pub use config::*;
pub(crate) use message_sending_queue::*;
pub(crate) use message_sending_smtp::*;

module! {
    pub NotifierAppModule {
        components = [
            NotifierClientProvider,
            LocalMessageSendingRepository,
            RemoteMessageSendingRepository,
        ],
        providers = []
    }
}
