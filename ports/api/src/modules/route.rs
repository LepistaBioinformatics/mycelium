use myc_mem_db::repositories::{
    RoutesFetchingMemDbRepo, ServiceReadMemDbRepo, ServiceWriteMemDbRepo,
};
use shaku::module;

module! {
    pub RoutesFetchingModule {
        components = [RoutesFetchingMemDbRepo],
        providers = []
    }
}

module! {
    pub ServiceReadModule {
        components = [ServiceReadMemDbRepo],
        providers = []
    }
}

module! {
    pub ServiceWriteModule {
        components = [ServiceWriteMemDbRepo],
        providers = []
    }
}
