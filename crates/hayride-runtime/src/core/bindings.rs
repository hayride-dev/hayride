pub mod generated {
    wasmtime::component::bindgen!({
        path: "../../wit",
        world: "hayride-core",
        imports: {
            default: trappable,
        },
        with: {
            "hayride:core/version/error": hayride_host_traits::core::version::Error,
        },
    });
}

pub use self::generated::hayride::core::*;
