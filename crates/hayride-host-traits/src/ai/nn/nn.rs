use super::errors::BackendError;
use anyhow::anyhow;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use wasmtime_wasi::StreamError;

pub trait BackendInner: Send + Sync {
    fn load(&mut self, name: String) -> Result<Graph, BackendError>;
}

pub trait BackendGraph: Send + Sync {
    fn init_execution_context(&self) -> Result<ExecutionContext, BackendError>;
}

pub trait BackendExecutionContext: Send {
    //fn set_input(&mut self, id: String, tensor: &Tensor) -> Result<(), BackendError>;
    fn compute(&mut self, tensors: Vec<(String, Tensor)>) -> Result<Tensor, BackendError>;
    //fn get_output(&mut self, id: String) -> Result<Tensor, BackendError>;
    fn compute_stream(
        &mut self,
        tensors: Vec<(String, Tensor)>,
    ) -> Result<TensorStream, BackendError>;
}

/// A backend-defined execution context.
pub struct ExecutionContext(Box<dyn BackendExecutionContext>);
impl From<Box<dyn BackendExecutionContext>> for ExecutionContext {
    fn from(value: Box<dyn BackendExecutionContext>) -> Self {
        Self(value)
    }
}
impl std::ops::Deref for ExecutionContext {
    type Target = dyn BackendExecutionContext;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for ExecutionContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}

/// A backend-defined graph (i.e., ML model).
#[derive(Clone)]
pub struct Graph(Arc<dyn BackendGraph>);
impl From<Box<dyn BackendGraph>> for Graph {
    fn from(value: Box<dyn BackendGraph>) -> Self {
        Self(value.into())
    }
}
impl std::ops::Deref for Graph {
    type Target = dyn BackendGraph;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

/// A host-side tensor.
#[derive(Clone, PartialEq)]
pub struct Tensor {
    pub dimensions: Vec<u32>,
    pub ty: TensorType,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct PromptOptions {
    temperature: f32,
    num_context: i32,
    num_batch: i32,
    max_predict: i32,
    top_k: i32,
    top_p: f32,
    seed: u32,
}

/// A host-side tensor-stream.
pub struct TensorStream {
    pub dimensions: Vec<u32>,
    pub ty: TensorType,

    // Based on wasmtime AsyncReadStream
    closed: bool,
    buffer: Option<Result<Bytes, StreamError>>,
    receiver: mpsc::Receiver<Result<Bytes, StreamError>>,
    _join_handle: Option<wasmtime_wasi::runtime::AbortOnDropJoinHandle<()>>,
}

impl TensorStream {
    pub fn new<T: tokio::io::AsyncRead + Send + Unpin + 'static>(
        dimensions: Vec<u32>,
        ty: TensorType,
        mut reader: T,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(1);
        let join_handle = wasmtime_wasi::runtime::spawn(async move {
            loop {
                use tokio::io::AsyncReadExt;
                let mut buf = bytes::BytesMut::with_capacity(4096);
                let sent = match reader.read_buf(&mut buf).await {
                    Ok(nbytes) if nbytes == 0 => sender.send(Err(StreamError::Closed)).await,
                    Ok(_) => sender.send(Ok(buf.freeze())).await,
                    Err(e) => {
                        sender
                            .send(Err(StreamError::LastOperationFailed(e.into())))
                            .await
                    }
                };
                if sent.is_err() {
                    // no more receiver - stop trying to read
                    break;
                }
            }
        });
        Self {
            dimensions,
            ty,

            closed: false,
            buffer: None,
            receiver,
            _join_handle: Some(join_handle),
        }
    }
}

#[async_trait::async_trait]
impl wasmtime_wasi::HostInputStream for TensorStream {
    fn read(&mut self, size: usize) -> wasmtime_wasi::StreamResult<Bytes> {
        use mpsc::error::TryRecvError;

        match self.buffer.take() {
            Some(Ok(mut bytes)) => {
                // TODO: de-duplicate the buffer management with the case below
                let len = bytes.len().min(size);
                let rest = bytes.split_off(len);
                if !rest.is_empty() {
                    self.buffer = Some(Ok(rest));
                }
                return Ok(bytes);
            }
            Some(Err(e)) => {
                self.closed = true;
                return Err(e);
            }
            None => {}
        }

        match self.receiver.try_recv() {
            Ok(Ok(mut bytes)) => {
                let len = bytes.len().min(size);
                let rest = bytes.split_off(len);
                if !rest.is_empty() {
                    self.buffer = Some(Ok(rest));
                }

                Ok(bytes)
            }
            Ok(Err(e)) => {
                self.closed = true;
                Err(e)
            }
            Err(TryRecvError::Empty) => Ok(Bytes::new()),
            Err(TryRecvError::Disconnected) => Err(StreamError::Trap(anyhow!(
                "AsyncReadStream sender died - should be impossible"
            ))),
        }
    }
}

#[async_trait::async_trait]
impl wasmtime_wasi::Subscribe for TensorStream {
    async fn ready(&mut self) {
        if self.buffer.is_some() || self.closed {
            return;
        }
        match self.receiver.recv().await {
            Some(res) => self.buffer = Some(res),
            None => {
                panic!("no more sender for an open AsyncReadStream - should be impossible")
            }
        }
    }
}

/// The tensor type options
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TensorType {
    FP16,
    FP32,
    FP64,
    BF16,
    U8,
    I32,
    I64,
}

pub struct FutureResult {
    // Based on wasmtime AsyncReadStream
    closed: bool,
    buffer: Option<Result<Bytes, StreamError>>,
    receiver: mpsc::Receiver<Result<Bytes, StreamError>>,
    _join_handle: Option<wasmtime_wasi::runtime::AbortOnDropJoinHandle<()>>,
}

impl FutureResult {
    pub fn new<T: tokio::io::AsyncRead + Send + Unpin + 'static>(mut reader: T) -> Self {
        let (sender, receiver) = mpsc::channel(1);
        let join_handle = wasmtime_wasi::runtime::spawn(async move {
            loop {
                use tokio::io::AsyncReadExt;
                let mut buf = bytes::BytesMut::with_capacity(4096);
                let sent = match reader.read_buf(&mut buf).await {
                    Ok(nbytes) if nbytes == 0 => sender.send(Err(StreamError::Closed)).await,
                    Ok(_) => sender.send(Ok(buf.freeze())).await,
                    Err(e) => {
                        sender
                            .send(Err(StreamError::LastOperationFailed(e.into())))
                            .await
                    }
                };
                if sent.is_err() {
                    // no more receiver - stop trying to read
                    break;
                }
            }
        });
        Self {
            closed: false,
            buffer: None,
            receiver,
            _join_handle: Some(join_handle),
        }
    }

    pub fn get(&mut self) -> wasmtime_wasi::StreamResult<Bytes> {
        use mpsc::error::TryRecvError;

        match self.buffer.take() {
            Some(Ok(bytes)) => {
                return Ok(bytes);
            }
            Some(Err(e)) => {
                self.closed = true;
                return Err(e);
            }
            None => {}
        }

        match self.receiver.try_recv() {
            Ok(Ok(bytes)) => Ok(bytes),
            Ok(Err(e)) => {
                self.closed = true;
                Err(e)
            }
            Err(TryRecvError::Empty) => Ok(Bytes::new()),
            Err(TryRecvError::Disconnected) => Err(StreamError::Trap(anyhow!(
                "AsyncReadStream sender died - should be impossible"
            ))),
        }
    }
}

#[async_trait::async_trait]
impl wasmtime_wasi::Subscribe for FutureResult {
    async fn ready(&mut self) {
        if self.buffer.is_some() || self.closed {
            return;
        }
        match self.receiver.recv().await {
            Some(res) => self.buffer = Some(res),
            None => {
                panic!("no more sender for an open AsyncReadStream - should be impossible")
            }
        }
    }
}
