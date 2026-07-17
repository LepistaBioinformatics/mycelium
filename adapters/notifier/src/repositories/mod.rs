use myc_adapters_shared_lib::models::SharedClientImpl;
use shaku::module;

mod config;
mod local_message_sending;
#[cfg(feature = "local-transport")]
mod local_transport_sending;
mod remote_message_sending;
#[cfg(feature = "local-transport")]
mod render_stub_email_for_terminal;
pub(crate) mod shared;

pub use config::*;
pub(crate) use local_message_sending::*;
#[cfg(feature = "local-transport")]
pub use local_transport_sending::*;
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

#[cfg(feature = "local-transport")]
module! {
    pub LocalNotifierAppModule {
        components = [
            LocalTransportMessageSendingRepository,
        ],
        providers = []
    }
}
