use myc_notifier::repositories::MessageSendingSmtpRepository;
use shaku::module;

module! {
    pub MessageSendingModule {
        components = [MessageSendingSmtpRepository],
        providers = []
    }
}
