use myc_mem_db::repositories::RoutesFetchingMemDbRepo;
use shaku::module;

module! {
    pub RoutesFetchingModule {
        components = [RoutesFetchingMemDbRepo],
        providers = []
    }
}
