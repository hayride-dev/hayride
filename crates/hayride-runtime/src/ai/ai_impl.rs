use super::ai::{AiImpl, AiView};
use super::bindings::graph::{ExecutionTarget, GraphBuilder, GraphEncoding};
use super::bindings::graph_stream::GraphStream;
use super::bindings::inference_stream::TensorStream;
use super::bindings::{
    errors, graph, graph_stream, inference, inference_stream, model_repository, rag, tensor,
    tensor_stream, transformer,
};
use hayride_host_traits::ai::model::ErrorCode as ModelErrorCode;
use hayride_host_traits::ai::rag::{
    Connection, Error as RagError, ErrorCode as RagErrorCode, RagOption, Transformer,
};
use hayride_host_traits::ai::{Error, ErrorCode, ExecutionContext, Graph, Tensor};

use anyhow::anyhow;
use wasmtime::component::Resource;
use wasmtime::Result;
use wasmtime_wasi::p2::InputStream;

// Construct an error resource and return it
macro_rules! bail {
    ($self:ident, $code:expr, $data:expr) => {
        let e = Error {
            code: $code,
            data: $data.into(),
        };
        let r = $self.table().push(e)?;
        return Ok(Err(r));
    };
}

macro_rules! rag_bail {
    ($self:ident, $code:expr, $data:expr) => {
        let e = rag::Error {
            code: $code,
            data: $data.into(),
        };
        let r = $self.table().push(e)?;
        return Ok(Err(r));
    };
}

macro_rules! model_bail {
    ($self:ident, $code:expr, $data:expr) => {
        let e = model_repository::Error {
            code: $code,
            data: $data.into(),
        };
        let r = $self.table().push(e)?;
        return Ok(Err(r));
    };
}

impl<T> tensor::Host for AiImpl<T> where T: AiView {}

impl<T> tensor::HostTensor for AiImpl<T>
where
    T: AiView,
{
    fn new(
        &mut self,
        dimensions: tensor::TensorDimensions,
        ty: tensor::TensorType,
        data: tensor::TensorData,
    ) -> Result<Resource<Tensor>> {
        let tensor = Tensor {
            dimensions: dimensions.into(),
            ty: ty.into(),
            data: data.into(),
        };

        let id: Resource<Tensor> = self.table().push(tensor)?;
        Ok(id)
    }

    fn data(&mut self, tensor: Resource<Tensor>) -> Result<tensor::TensorData> {
        let tensor = self.table().get(&tensor)?;
        Ok(tensor.data.clone())
    }

    fn dimensions(&mut self, tensor: Resource<Tensor>) -> Result<tensor::TensorDimensions> {
        let tensor = self.table().get(&tensor)?;
        Ok(tensor.dimensions.clone())
    }

    fn drop(&mut self, tensor: Resource<Tensor>) -> Result<()> {
        self.table().delete(tensor)?;
        Ok(())
    }

    fn ty(&mut self, tensor: Resource<Tensor>) -> Result<tensor::TensorType> {
        let tensor = self.table().get(&tensor)?;
        match tensor.ty {
            hayride_host_traits::ai::TensorType::FP32 => Ok(tensor::TensorType::Fp32),
            hayride_host_traits::ai::TensorType::FP16 => Ok(tensor::TensorType::Fp16),
            hayride_host_traits::ai::TensorType::FP64 => Ok(tensor::TensorType::Fp64),
            hayride_host_traits::ai::TensorType::I32 => Ok(tensor::TensorType::I32),
            hayride_host_traits::ai::TensorType::I64 => Ok(tensor::TensorType::I64),
            hayride_host_traits::ai::TensorType::U8 => Ok(tensor::TensorType::U8),
            hayride_host_traits::ai::TensorType::BF16 => Ok(tensor::TensorType::Bf16),
        }
    }
}

impl<T> graph::Host for AiImpl<T>
where
    T: AiView,
{
    fn load_by_name(
        &mut self,
        path: String,
    ) -> Result<Result<Resource<Graph>, Resource<errors::Error>>> {
        match self.ctx().backend.load(path) {
            Ok(graph) => {
                let id = self.table().push(graph)?;
                return Ok(Ok(id));
            }
            Err(error) => {
                bail!(self, ErrorCode::RuntimeError, error);
            }
        }
    }

    fn load(
        &mut self,
        _builder: Vec<GraphBuilder>,
        _encoding: GraphEncoding,
        _target: ExecutionTarget,
    ) -> Result<Result<Resource<Graph>, Resource<errors::Error>>> {
        bail!(
            self,
            ErrorCode::UnsupportedOperation,
            anyhow!("Load not implemented, use load_by_name")
        );
    }
}

impl<T> graph::HostGraph for AiImpl<T>
where
    T: AiView,
{
    fn init_execution_context(
        &mut self,
        graph: Resource<Graph>,
    ) -> Result<Result<Resource<ExecutionContext>, Resource<graph::Error>>> {
        let graph = self.table().get(&graph)?;
        match graph.init_execution_context() {
            Ok(exec_context) => {
                let id = self.table().push(exec_context)?;
                return Ok(Ok(id));
            }
            Err(error) => {
                bail!(self, ErrorCode::RuntimeError, error);
            }
        }
    }

    fn drop(&mut self, id: Resource<Graph>) -> Result<(), wasmtime::Error> {
        self.table().delete(id)?;
        Ok(())
    }
}

impl<T> inference::Host for AiImpl<T> where T: AiView {}

impl<T> inference::HostGraphExecutionContext for AiImpl<T>
where
    T: AiView,
{
    fn compute(
        &mut self,
        exec_context: Resource<inference::GraphExecutionContext>,
        inputs: Vec<(String, Resource<Tensor>)>,
    ) -> Result<Result<Vec<(String, Resource<Tensor>)>, Resource<errors::Error>>, wasmtime::Error>
    {
        // Convert tensor resources to tensors
        let converted_inputs: Vec<(String, Tensor)> = inputs
            .into_iter()
            .map(|(name, tensor)| {
                let tensor = self.table().get(&tensor)?;
                Ok((name, tensor.clone()))
            })
            .collect::<Result<Vec<(String, Tensor)>>>()?;

        // Compute
        let context = self.table().get_mut(&exec_context)?;
        match context.compute(converted_inputs) {
            Ok(tensor) => {
                let mut results: Vec<(String, Resource<Tensor>)> = Vec::new();
                let id = self.table().push(tensor)?;
                results.push(("Output".to_string(), id));

                return Ok(Ok(results));
            }
            Err(error) => {
                bail!(self, ErrorCode::RuntimeError, error);
            }
        }
    }

    fn drop(&mut self, id: Resource<inference::GraphExecutionContext>) -> Result<()> {
        self.table().delete(id)?;
        Ok(())
    }
}

impl<T> errors::Host for AiImpl<T> where T: AiView {}

impl<T> errors::HostError for AiImpl<T>
where
    T: AiView,
{
    fn code(&mut self, error: Resource<errors::Error>) -> Result<errors::ErrorCode> {
        let error = self.table().get(&error)?;
        match error.code {
            ErrorCode::InvalidArgument => Ok(errors::ErrorCode::InvalidArgument),
            ErrorCode::InvalidEncoding => Ok(errors::ErrorCode::InvalidEncoding),
            ErrorCode::Timeout => Ok(errors::ErrorCode::Timeout),
            ErrorCode::RuntimeError => Ok(errors::ErrorCode::RuntimeError),
            ErrorCode::UnsupportedOperation => Ok(errors::ErrorCode::UnsupportedOperation),
            ErrorCode::TooLarge => Ok(errors::ErrorCode::TooLarge),
            ErrorCode::NotFound => Ok(errors::ErrorCode::NotFound),
        }
    }

    fn data(&mut self, error: Resource<errors::Error>) -> Result<String> {
        let error = self.table().get(&error)?;
        return Ok(error.data.to_string());
    }

    fn drop(&mut self, error: Resource<errors::Error>) -> Result<()> {
        self.table().delete(error)?;
        return Ok(());
    }
}

impl<T> tensor_stream::Host for AiImpl<T> where T: AiView {}

impl<T> tensor_stream::HostTensorStream for AiImpl<T>
where
    T: AiView,
{
    fn new(
        &mut self,
        dimensions: tensor::TensorDimensions,
        ty: tensor::TensorType,
        data: tensor::TensorData,
    ) -> Result<Resource<TensorStream>> {
        let buffer = std::io::Cursor::new(data.clone().as_slice().to_vec());
        let tensor = tensor_stream::TensorStream::new(dimensions.clone(), ty.into(), buffer);

        let id: Resource<TensorStream> = self.table().push(tensor)?;

        Ok(id)
    }

    fn read(
        &mut self,
        tensor: Resource<TensorStream>,
        len: u64,
    ) -> Result<Result<tensor_stream::TensorData, tensor_stream::StreamError>> {
        let tensor = self.table().get_mut(&tensor)?;
        let len = len as usize;
        let data: Result<bytes::Bytes, wasmtime_wasi::p2::StreamError> = tensor.read(len);

        let data = data.map(|bytes| bytes.to_vec()).map_err(|_err| {
            // Convert wasmtime_wasi::StreamError to tensor_stream::StreamError
            // TODO: Support other error types
            tensor_stream::StreamError::Closed
        });

        Ok(data)
    }

    fn subscribe(
        &mut self,
        tensor: Resource<TensorStream>,
    ) -> wasmtime::Result<Resource<tensor_stream::Pollable>> {
        wasmtime_wasi::p2::subscribe(self.table(), tensor)
    }

    fn dimensions(&mut self, tensor: Resource<TensorStream>) -> Result<tensor::TensorDimensions> {
        let tensor: &tensor_stream::TensorStream = self.table().get(&tensor)?;
        Ok(tensor.dimensions.clone())
    }

    fn ty(&mut self, tensor: Resource<TensorStream>) -> Result<tensor::TensorType> {
        let tensor = self.table().get(&tensor)?;
        match tensor.ty {
            hayride_host_traits::ai::TensorType::FP32 => Ok(tensor::TensorType::Fp32),
            hayride_host_traits::ai::TensorType::FP16 => Ok(tensor::TensorType::Fp16),
            hayride_host_traits::ai::TensorType::FP64 => Ok(tensor::TensorType::Fp64),
            hayride_host_traits::ai::TensorType::I32 => Ok(tensor::TensorType::I32),
            hayride_host_traits::ai::TensorType::I64 => Ok(tensor::TensorType::I64),
            hayride_host_traits::ai::TensorType::U8 => Ok(tensor::TensorType::U8),
            hayride_host_traits::ai::TensorType::BF16 => Ok(tensor::TensorType::Bf16),
        }
    }

    fn drop(&mut self, tensor: Resource<TensorStream>) -> Result<()> {
        self.table().delete(tensor)?;
        Ok(())
    }
}

impl<T> graph_stream::Host for AiImpl<T>
where
    T: AiView,
{
    fn load_by_name(
        &mut self,
        path: String,
    ) -> Result<Result<Resource<GraphStream>, Resource<errors::Error>>> {
        match self.ctx().backend.load(path) {
            Ok(graph) => {
                let id = self.table().push(graph)?;
                return Ok(Ok(id));
            }
            Err(error) => {
                bail!(self, ErrorCode::RuntimeError, error);
            }
        }
    }
}

impl<T> graph_stream::HostGraphStream for AiImpl<T>
where
    T: AiView,
{
    fn init_execution_context_stream(
        &mut self,
        graph: Resource<GraphStream>,
    ) -> Result<Result<Resource<ExecutionContext>, Resource<graph::Error>>> {
        let graph = self.table().get(&graph)?;
        match graph.init_execution_context() {
            Ok(exec_context) => {
                let id = self.table().push(exec_context)?;
                return Ok(Ok(id));
            }
            Err(error) => {
                bail!(self, ErrorCode::RuntimeError, error);
            }
        }
    }

    fn drop(&mut self, id: Resource<Graph>) -> Result<(), wasmtime::Error> {
        self.table().delete(id)?;
        Ok(())
    }
}

impl<T> inference_stream::Host for AiImpl<T> where T: AiView {}

impl<T> inference_stream::HostGraphExecutionContextStream for AiImpl<T>
where
    T: AiView,
{
    fn compute(
        &mut self,
        exec_context: Resource<ExecutionContext>,
        inputs: Vec<inference_stream::NamedTensor>,
    ) -> Result<Result<inference_stream::NamedTensorStream, Resource<inference_stream::Error>>>
    {
        // Convert tensor resources to tensors
        let inputs: Vec<(String, Tensor)> = inputs
            .into_iter()
            .map(|(name, tensor)| {
                let tensor = self.table().get(&tensor)?;
                Ok((name, tensor.clone()))
            })
            .collect::<Result<Vec<(String, Tensor)>>>()?;

        // Get the compute stream from the execution context
        let context = self.table().get_mut(&exec_context)?;
        match context.compute_stream(inputs) {
            Ok(tensor_stream) => {
                let id = self.table().push(tensor_stream)?;

                // TODO: How to get a valid output name?
                let named_tensor_stream = ("Output".to_string(), id);

                return Ok(Ok(named_tensor_stream));
            }
            Err(error) => {
                bail!(self, ErrorCode::RuntimeError, error);
            }
        }
    }

    fn drop(&mut self, id: Resource<inference::GraphExecutionContext>) -> Result<()> {
        self.table().delete(id)?;
        Ok(())
    }
}

impl<T> rag::Host for AiImpl<T>
where
    T: AiView,
{
    fn connect(
        &mut self,
        dsn: String,
    ) -> Result<Result<Resource<Connection>, Resource<rag::Error>>> {
        match self.ctx().rag.connect(dsn.clone()) {
            Ok(conn) => {
                let id = self.table().push(conn)?;
                return Ok(Ok(id));
            }
            Err(error) => {
                rag_bail!(self, error, anyhow!("Failed to connect to Rag: {}", dsn));
            }
        }
    }
}

impl<T> rag::HostConnection for AiImpl<T>
where
    T: AiView,
{
    fn register(
        &mut self,
        conn: Resource<rag::Connection>,
        transformer: Resource<Transformer>,
    ) -> Result<Result<(), Resource<rag::Error>>> {
        let table = self.table();
        let transformer = {
            let transformer_ref = table.get(&transformer)?;
            transformer_ref.clone()
        };
        let conn = table.get_mut(&conn)?;

        match conn.register(transformer.clone()) {
            Ok(()) => {
                return Ok(Ok(()));
            }
            Err(error) => {
                rag_bail!(
                    self,
                    error,
                    anyhow!("Register failed for transformer: {:?}", transformer)
                );
            }
        }
    }

    fn embed(
        &mut self,
        conn: Resource<rag::Connection>,
        table: String,
        data: String,
    ) -> Result<Result<(), Resource<RagError>>> {
        let conn = self.table().get(&conn)?;
        match conn.embed(table.clone(), data.clone()) {
            Ok(()) => {
                return Ok(Ok(()));
            }
            Err(error) => {
                rag_bail!(
                    self,
                    error,
                    anyhow!("Embed failed for table: {}, data: {}", table, data)
                );
            }
        }
    }

    fn query(
        &mut self,
        conn: Resource<rag::Connection>,
        table: String,
        data: String,
        options: Vec<rag::RagOption>,
    ) -> Result<Result<Vec<String>, Resource<RagError>>> {
        let conn = self.table().get(&conn)?;

        // Convert RagOption to hayride_rag::RagOption
        let options: Vec<RagOption> = options
            .into_iter()
            .map(|option| RagOption {
                name: option.0,
                value: option.1,
            })
            .collect();

        match conn.query(table.clone(), data.clone(), options) {
            Ok(results) => {
                return Ok(Ok(results));
            }
            Err(error) => {
                rag_bail!(
                    self,
                    error,
                    anyhow!("Query failed for table: {}, data: {}", table, data)
                );
            }
        }
    }

    fn drop(&mut self, id: Resource<rag::Connection>) -> Result<()> {
        self.table().delete(id)?;
        return Ok(());
    }
}

impl<T> rag::HostError for AiImpl<T>
where
    T: AiView,
{
    fn code(&mut self, error: Resource<rag::Error>) -> Result<rag::ErrorCode> {
        let error = self.table().get(&error)?;
        match error.code {
            RagErrorCode::ConnectionFailed => Ok(rag::ErrorCode::ConnectionFailed),
            RagErrorCode::CreateTableFailed => Ok(rag::ErrorCode::CreateTableFailed),
            RagErrorCode::QueryFailed => Ok(rag::ErrorCode::QueryFailed),
            RagErrorCode::EmbedFailed => Ok(rag::ErrorCode::EmbedFailed),
            RagErrorCode::RegisterFailed => Ok(rag::ErrorCode::RegisterFailed),
            RagErrorCode::MissingTable => Ok(rag::ErrorCode::MissingTable),
            RagErrorCode::InvalidOption => Ok(rag::ErrorCode::InvalidOption),
            RagErrorCode::NotEnabled => Ok(rag::ErrorCode::NotEnabled),
            RagErrorCode::Unknown => Ok(rag::ErrorCode::Unknown),
        }
    }

    fn data(&mut self, error: Resource<rag::Error>) -> Result<String> {
        let error = self.table().get(&error)?;
        return Ok(error.data.to_string());
    }

    fn drop(&mut self, error: Resource<rag::Error>) -> Result<()> {
        self.table().delete(error)?;
        return Ok(());
    }
}

impl<T> transformer::Host for AiImpl<T> where T: AiView {}

impl<T> transformer::HostTransformer for AiImpl<T>
where
    T: AiView,
{
    fn new(
        &mut self,
        embedding: super::bindings::transformer::EmbeddingType,
        model: String,
        data_column: String,
        vector_column: String,
    ) -> Result<Resource<Transformer>> {
        let embedding = match embedding {
            transformer::EmbeddingType::Sentence => {
                hayride_host_traits::ai::rag::Embedding::Sentence
            }
        };

        let transformer = Transformer {
            embedding: embedding,
            model: model,
            data_column: data_column,
            vector_column: vector_column,
        };

        let id = self.table().push(transformer)?;
        Ok(id)
    }

    fn drop(&mut self, id: Resource<Transformer>) -> Result<()> {
        self.table().delete(id)?;
        Ok(())
    }

    fn embedding(
        &mut self,
        transformer: Resource<Transformer>,
    ) -> Result<transformer::EmbeddingType> {
        let transformer = self.table().get(&transformer)?;

        match transformer.embedding {
            hayride_host_traits::ai::rag::Embedding::Sentence => {
                Ok(transformer::EmbeddingType::Sentence)
            }
        }
    }

    fn model(&mut self, transformer: Resource<Transformer>) -> Result<String> {
        let transformer = self.table().get(&transformer)?;
        Ok(transformer.model.clone())
    }

    fn data_column(&mut self, transformer: Resource<Transformer>) -> Result<String> {
        let transformer = self.table().get(&transformer)?;
        Ok(transformer.data_column.clone())
    }

    fn vector_column(&mut self, transformer: Resource<Transformer>) -> Result<String> {
        let transformer = self.table().get(&transformer)?;
        Ok(transformer.vector_column.clone())
    }
}

impl<T> model_repository::Host for AiImpl<T>
where
    T: AiView,
{
    fn download_model(
        &mut self,
        name: String,
    ) -> Result<Result<String, Resource<model_repository::Error>>> {
        match self.ctx().model_repository.download(name.clone()) {
            Ok(path) => {
                return Ok(Ok(path));
            }
            Err(error) => {
                model_bail!(self, error.clone(), anyhow!("download model failed with '{}'", error));
            }
        }
    }

    fn get_model(&mut self,name:wasmtime::component::__internal::String,) -> wasmtime::Result<std::result::Result<wasmtime::component::__internal::String,wasmtime::component::Resource<hayride_host_traits::ai::model::Error>>> {
        match self.ctx().model_repository.get(name.clone()) {
            Ok(path) => {
                return Ok(Ok(path));
            }
            Err(error) => {
                model_bail!(self, error.clone(), anyhow!("get model failed with '{}'", error));
            }
        }
    }

    fn delete_model(&mut self,name:wasmtime::component::__internal::String,) -> wasmtime::Result<std::result::Result<(),wasmtime::component::Resource<hayride_host_traits::ai::model::Error>>> {
        match self.ctx().model_repository.delete(name.clone()) {
            Ok(()) => {
                return Ok(Ok(()));
            }
            Err(error) => {
                model_bail!(self, error.clone(), anyhow!("delete model failed with '{}'", error));
            }
        }
    }

    fn list_models(&mut self,) -> wasmtime::Result<std::result::Result<wasmtime::component::__internal::Vec<wasmtime::component::__internal::String>,wasmtime::component::Resource<hayride_host_traits::ai::model::Error>>> {
        match self.ctx().model_repository.list() {
            Ok(models) => Ok(Ok(models)),
            Err(error) => {
                model_bail!(self, error.clone(), anyhow!("list models failed with '{}'", error));
            }
        }
    }
}

impl<T> model_repository::HostError for AiImpl<T>
where
    T: AiView,
{
    fn code(
        &mut self,
        error: Resource<model_repository::Error>,
    ) -> Result<model_repository::ErrorCode> {
        let error = self.table().get(&error)?;
        match error.code {
            ModelErrorCode::ModelNotFound => Ok(model_repository::ErrorCode::ModelNotFound),
            ModelErrorCode::InvalidModelName => Ok(model_repository::ErrorCode::InvalidModelName),
            ModelErrorCode::RuntimeError => Ok(model_repository::ErrorCode::RuntimeError),
            ModelErrorCode::NotEnabled => Ok(model_repository::ErrorCode::NotEnabled),
            ModelErrorCode::Unknown => Ok(model_repository::ErrorCode::Unknown),
        }
    }

    fn data(&mut self, error: Resource<model_repository::Error>) -> Result<String> {
        let error = self.table().get(&error)?;
        return Ok(error.data.to_string());
    }

    fn drop(&mut self, error: Resource<model_repository::Error>) -> Result<()> {
        self.table().delete(error)?;
        return Ok(());
    }
}
