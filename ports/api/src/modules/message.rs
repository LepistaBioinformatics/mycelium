use myc_notifier::repositories::MessageSendingQueueRepository;
use shaku::module;

module! {
    pub MessageSendingQueueModule {
        components = [MessageSendingQueueRepository],
        providers = []
    }
}
