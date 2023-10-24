use myc_smtp::repositories::MessageSendingSmtpRepository;
use shaku::module;

module! {
    pub MessageSendingModule {
        components = [MessageSendingSmtpRepository],
        providers = []
    }
}
