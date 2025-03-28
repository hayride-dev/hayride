// The morph runtime world
pub mod generated {
    wasmtime::component::bindgen!({
        path: "../../wit",
        world: "hayride-silo",
    });
}

pub use self::generated::hayride::silo::*;
