pub mod errors;
pub mod mock;
pub mod nn;

pub use nn::{
    BackendExecutionContext, BackendGraph, BackendInner, ExecutionContext, FutureResult, Graph,
    Tensor, TensorStream, TensorType,
};

pub use errors::{BackendError, Error, ErrorCode};
