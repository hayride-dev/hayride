mod generated {
    wasmtime::component::bindgen!({
        path: "../../wit",
        world: "hayride-ai",
        // Indicates that the `T` in `Store<T>` should be send even if async is not
        // enabled.
        //
        // This is helpful when sync bindings depend on generated functions from
        // async bindings as is the case with WASI in-tree.
        require_store_data_send: true,

        // Wrap functions returns with a result with error
        trappable_imports: true,
        with: {
            // Upstream package dependencies
            "wasi:io": wasmtime_wasi::p2::bindings::io,

            "wasi:nn/tensor/tensor": hayride_host_traits::ai::Tensor,
            "wasi:nn/errors/error": hayride_host_traits::ai::Error,
            "wasi:nn/graph/graph" : hayride_host_traits::ai::Graph,
            "wasi:nn/inference/graph-execution-context": hayride_host_traits::ai::ExecutionContext,
            "hayride:ai/tensor-stream/tensor-stream": hayride_host_traits::ai::TensorStream,
            "hayride:ai/graph-stream/graph-stream": hayride_host_traits::ai::Graph, // Reuse Graph for graph stream
            "hayride:ai/inference-stream/graph-execution-context-stream": hayride_host_traits::ai::ExecutionContext, // Reuse ExecutionContext for graph execution context
            "hayride:ai/rag/connection": hayride_host_traits::ai::rag::Connection,
            "hayride:ai/transformer/transformer": hayride_host_traits::ai::rag::Transformer,
            "hayride:ai/rag/error": hayride_host_traits::ai::rag::Error,
            "hayride:ai/model-repository/error": hayride_host_traits::ai::model::Error,
        },
    });
}

pub use self::generated::hayride::ai::*;
pub use self::generated::wasi::nn::*;

// Convert from generated types to hayride_host_traits types
impl Into<hayride_host_traits::ai::TensorType> for self::generated::wasi::nn::tensor::TensorType {
    fn into(self) -> hayride_host_traits::ai::TensorType {
        match self {
            self::generated::wasi::nn::tensor::TensorType::Fp16 => {
                hayride_host_traits::ai::TensorType::FP16
            }
            self::generated::wasi::nn::tensor::TensorType::Fp32 => {
                hayride_host_traits::ai::TensorType::FP32
            }
            self::generated::wasi::nn::tensor::TensorType::Fp64 => {
                hayride_host_traits::ai::TensorType::FP64
            }
            self::generated::wasi::nn::tensor::TensorType::Bf16 => {
                hayride_host_traits::ai::TensorType::BF16
            }
            self::generated::wasi::nn::tensor::TensorType::U8 => {
                hayride_host_traits::ai::TensorType::U8
            }
            self::generated::wasi::nn::tensor::TensorType::I32 => {
                hayride_host_traits::ai::TensorType::I32
            }
            self::generated::wasi::nn::tensor::TensorType::I64 => {
                hayride_host_traits::ai::TensorType::I64
            }
        }
    }
}
