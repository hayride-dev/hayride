// The morph runtime world
pub mod generated {
    wasmtime::component::bindgen!({
        path: "../../wit",
        world: "hayride-silo",
        with: {
            "hayride:silo/threads/thread": hayride_host_traits::silo::Thread,
        },
    });
}

pub use self::generated::hayride::silo::*;
