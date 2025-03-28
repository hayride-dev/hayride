use super::{
    BackendError, BackendExecutionContext, BackendGraph, BackendInner, ExecutionContext, Graph,
    Tensor, TensorType,
};

#[derive(Default)]
pub struct MockBackend {}

impl BackendInner for MockBackend {
    fn load(&mut self, _name: String) -> Result<Graph, BackendError> {
        let graph: Box<dyn BackendGraph> = Box::new(MockGraph {});
        return Ok(graph.into());
    }
}

struct MockGraph {}

impl BackendGraph for MockGraph {
    fn init_execution_context(&self) -> Result<ExecutionContext, BackendError> {
        let context: Box<dyn BackendExecutionContext> = Box::new(MockExecutionContext {});
        return Ok(context.into());
    }
}

struct MockExecutionContext {}

impl BackendExecutionContext for MockExecutionContext {
    fn compute(&mut self, _tensors: Vec<(String, Tensor)>) -> Result<Tensor, BackendError> {
        let tensor = Tensor {
            dimensions: vec![1],
            ty: TensorType::U8,
            data: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        };
        return Ok(tensor);
    }

    fn compute_stream(
        &mut self,
        _tensors: Vec<(String, Tensor)>,
    ) -> Result<super::TensorStream, BackendError> {
        let buffer = std::io::Cursor::new(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let tensor = super::TensorStream::new(vec![1], TensorType::U8, buffer);

        Ok(tensor)
    }
}
