pub mod generated {
    wasmtime::component::bindgen!({
        path: "../../wit",
        world: "hayride-db",
        trappable_imports: true,
        with: {
            "hayride:db/db/error": hayride_host_traits::db::Error,
            "hayride:db/db/connection": hayride_host_traits::db::Connection,
        },
    });
}

pub use self::generated::hayride::db::*;
