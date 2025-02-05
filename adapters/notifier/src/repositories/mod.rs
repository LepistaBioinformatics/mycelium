use myc_adapters_shared_lib::models::SharedClientImpl;
use shaku::module;

mod config;
mod local_message_sending;
mod remote_message_sending;

pub use config::*;
pub(crate) use local_message_sending::*;
pub(crate) use remote_message_sending::*;

module! {
    pub NotifierAppModule {
        components = [
            SharedClientImpl,
            NotifierClientImpl,
            LocalMessageSendingRepository,
            RemoteMessageSendingRepository,
        ],
        providers = []
    }
}
