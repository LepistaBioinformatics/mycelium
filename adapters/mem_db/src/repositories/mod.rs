use shaku::module;

mod config;
mod routes_read;
mod service_read;
mod service_write;
mod shared;

use routes_read::*;
use service_read::*;
use service_write::*;

pub use config::*;

module! {
    pub MemDbModule {
        components = [
            //
            // Provide the database pool
            //
            MemDbPoolProvider,
            //
            // Provide repositories
            //
            RoutesReadMemDbRepo,
            ServiceReadMemDbRepo,
            ServiceWriteMemDbRepo,
        ],
        providers = []
    }
}
