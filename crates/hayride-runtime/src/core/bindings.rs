pub mod generated {
    wasmtime::component::bindgen!({
        path: "../../wit",
        world: "hayride-core",
        trappable_imports: true,
        with: {
            "hayride:core/version/error": hayride_host_traits::core::version::Error,
        },
    });
}

pub use self::generated::hayride::core::*;
