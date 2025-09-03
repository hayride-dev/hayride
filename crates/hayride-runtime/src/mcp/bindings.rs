mod generated {
    wasmtime::component::bindgen!({
        path: "../../wit",
        world: "hayride-mcp",
        // Indicates that the `T` in `Store<T>` should be send even if async is not
        // enabled.
        //
        // This is helpful when sync bindings depend on generated functions from
        // async bindings as is the case with WASI in-tree.
        require_store_data_send: true,

        // Wrap functions returns with a result with error
        trappable_imports: true,
        with: {
            "hayride:mcp/tools/tools": hayride_host_traits::mcp::tools::Tools,
            "hayride:mcp/tools/error": hayride_host_traits::mcp::tools::Error,
            "hayride:mcp/auth/provider": hayride_host_traits::mcp::auth::Provider,
            "hayride:mcp/auth/error": hayride_host_traits::mcp::auth::Error,
        },
    });
}

pub use self::generated::hayride::*;
