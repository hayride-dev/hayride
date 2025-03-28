// The hayride cli world (used to execute a command line component)
pub mod hayride_cli {
    wasmtime::component::bindgen!({
        path: "../../wit",
        world: "hayride-cli",
        // Indicates that the `T` in `Store<T>` should be send even if async is not
        // enabled.
        //
        // This is helpful when sync bindings depend on generated functions from
        // async bindings as is the case with WASI in-tree.
        require_store_data_send: true,
        async: true,
    });
}

// The hayride server world (used by morph to run the server)
pub mod hayride_server {
    wasmtime::component::bindgen!({
        path: "../../wit",
        world: "hayride-server",
        // Indicates that the `T` in `Store<T>` should be send even if async is not
        // enabled.
        //
        // This is helpful when sync bindings depend on generated functions from
        // async bindings as is the case with WASI in-tree.
        require_store_data_send: true,
        async: true,
        with: {
            // Upstream package dependencies
            "wasi:io": wasmtime_wasi::bindings::io,

            // Configure all WIT http resources to be defined types in this
            // crate to use the `ResourceTable` helper methods.
            "wasi:http/types/outgoing-body": wasmtime_wasi_http::body::HostOutgoingBody,
            "wasi:http/types/future-incoming-response": wasmtime_wasi_http::types::HostFutureIncomingResponse,
            "wasi:http/types/outgoing-response": wasmtime_wasi_http::types::HostOutgoingResponse,
            "wasi:http/types/future-trailers": wasmtime_wasi_http::body::HostFutureTrailers,
            "wasi:http/types/incoming-body": wasmtime_wasi_http::body::HostIncomingBody,
            "wasi:http/types/incoming-response": wasmtime_wasi_http::types::HostIncomingResponse,
            "wasi:http/types/response-outparam": wasmtime_wasi_http::types::HostResponseOutparam,
            "wasi:http/types/outgoing-request": wasmtime_wasi_http::types::HostOutgoingRequest,
            "wasi:http/types/incoming-request": wasmtime_wasi_http::types::HostIncomingRequest,
        },
    });
}

// The hayride server world (used by morph to run the server)
pub mod hayride_ws {
    wasmtime::component::bindgen!({
        path: "../../wit",
        world: "hayride-ws",
        // Indicates that the `T` in `Store<T>` should be send even if async is not
        // enabled.
        //
        // This is helpful when sync bindings depend on generated functions from
        // async bindings as is the case with WASI in-tree.
        require_store_data_send: true,
        async: true,
        with: {
            // Upstream package dependencies
            "wasi:io": wasmtime_wasi::bindings::io,
        },
    });
}
