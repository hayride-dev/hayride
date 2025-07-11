pub mod generated {
    wasmtime::component::bindgen!({
        path: "../../wit",
        world: "hayride-wac",
        trappable_imports: true,
        with: {
            "hayride:wac/wac/error": hayride_host_traits::wac::Error,
        },
    });
}

pub use self::generated::hayride::wac::*;
