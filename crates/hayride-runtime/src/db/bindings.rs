pub mod generated {
    wasmtime::component::bindgen!({
        path: "../../wit",
        world: "hayride-db",
        trappable_imports: true,
        with: {
            "hayride:db/db/error": hayride_host_traits::db::Error,
            "hayride:db/db/connection": hayride_host_traits::db::Connection,
            "hayride:db/db/statement": hayride_host_traits::db::Statement,
            "hayride:db/db/transaction": hayride_host_traits::db::Transaction,
            "hayride:db/db/rows": hayride_host_traits::db::Rows,
        },
    });
}

pub use self::generated::hayride::db::*;
