use super::create_wasi_ctx;
use crate::bindings::hayride_ws::{HayrideWs, HayrideWsPre};
use crate::core::CoreCtx;
use crate::silo::SiloCtx;
use crate::Host;

use anyhow::bail;

use hyper_tungstenite::tungstenite::Utf8Bytes;
use wasmtime_wasi::p2::StreamError;
use wasmtime_wasi_http::{body::HyperOutgoingBody, WasiHttpCtx};

use bytes::{Buf, Bytes};
use hyper::body::Body;
use hyper::upgrade::Upgraded;
use hyper_tungstenite::WebSocketStream;
use hyper_tungstenite::{tungstenite, HyperWebsocket};
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;
use tungstenite::Message;
use uuid::Uuid;

use crate::ai::AiCtx;
use crate::db::DBCtx;
use crate::mcp::McpCtx;
use crate::wac::WacCtx;
use wasmtime::{component::ResourceTable, Result};
use wasmtime_wasi::cli::{IsTerminal, StdoutStream};

// Trait extensions
use futures::sink::SinkExt;
use futures::stream::{SplitSink, SplitStream, Stream, StreamExt};
use http_body_util::BodyExt;

pub struct WebsocketServer {
    id: Uuid,
    out_dir: Option<String>,
    ws_pre: HayrideWsPre<Host>,
    silo_ctx: SiloCtx,
    core_ctx: CoreCtx,
    registry_path: String,
    model_path: Option<String>,
    args: Vec<String>,
    envs: Vec<(String, String)>,
}

impl WebsocketServer {
    pub fn new(
        id: Uuid,
        out_dir: Option<String>,
        ws_pre: HayrideWsPre<Host>,
        silo_ctx: SiloCtx,
        core_ctx: CoreCtx,
        registry_path: String,
        model_path: Option<String>,
        args: Vec<String>,
        envs: Vec<(String, String)>,
    ) -> Self {
        Self {
            id,
            out_dir,
            ws_pre,
            silo_ctx,
            core_ctx,
            registry_path,
            model_path,
            args,
            envs,
        }
    }

    pub async fn handle_request(
        &self,
        mut req: hyper::Request<hyper::body::Incoming>,
    ) -> Result<hyper::Response<HyperOutgoingBody>> {
        // Check if this is a websocket request and handle it
        if hyper_tungstenite::is_upgrade_request(&req) {
            let wasi_ctx =
                create_wasi_ctx(&self.args, self.out_dir.clone(), self.id, false, &self.envs)?;
            let mut store: wasmtime::Store<Host> = wasmtime::Store::new(
                &self.ws_pre.engine(),
                Host {
                    ctx: wasi_ctx,
                    http_ctx: WasiHttpCtx::new(),
                    core_ctx: self.core_ctx.clone(),
                    ai_ctx: AiCtx::new(self.out_dir.clone(), self.model_path.clone())?,
                    mcp_ctx: McpCtx::new(),
                    silo_ctx: self.silo_ctx.clone(),
                    wac_ctx: WacCtx::new(self.registry_path.clone()),
                    db_ctx: DBCtx::new(),
                    table: ResourceTable::default(),
                },
            );

            // Instantiate the server
            let pre = self.ws_pre.clone();
            let server: HayrideWs = pre.instantiate_async(&mut store).await?;

            let (response, websocket) = hyper_tungstenite::upgrade(&mut req, None)?;

            tokio::spawn(async move {
                if let Err(e) = serve_websocket(websocket, server, store, req).await {
                    eprintln!("websocket error: {:?}", e);
                }
            });

            // Convert and return response so spawned future can continue.
            let response = response.map(|body| {
                let boxed = body.map_err(|never| match never {}).boxed();
                HyperOutgoingBody::new(boxed)
            });
            return Ok(response); // 101 Switching Protocols
        }

        bail!("Request not handled, was not a websocket upgrade request");
    }
}

/// Handle a websocket connection.
async fn serve_websocket<B>(
    websocket: HyperWebsocket,
    server: HayrideWs,
    mut store: wasmtime::Store<Host>,
    _req: hyper::Request<B>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
where
    B: Body<Data = Bytes, Error = hyper::Error> + Send + Sync + 'static,
{
    let websocket: WebSocketStream<hyper_util::rt::TokioIo<Upgraded>> = websocket.await?;
    let (write, read) = websocket.split();
    let out = WebsocketOutputPipe::new(write);

    let boxed_output: Box<dyn wasmtime_wasi::p2::OutputStream> = Box::new(out.clone());
    let output_arg = store.data_mut().table.push(boxed_output)?;

    let reader = WebSocketReader::new(read);
    let input = WebsocketInputPipe::new(reader);

    let boxed_input: Box<dyn wasmtime_wasi::p2::InputStream> = Box::new(input);
    let input_arg = store.data_mut().table.push(boxed_input)?;

    if let Err(e) = server
        .hayride_socket_websocket()
        .call_handle(&mut store, input_arg, output_arg)
        .await
    {
        log::warn!("error handling websocket request: {:?}", e);
        return Err(e.into());
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub struct WebsocketOutputPipe {
    // websocket: Arc<Mutex<SplitSink<WebSocketStream<hyper_util::rt::TokioIo<Upgraded>>, Message>>>,
    sender: tokio::sync::mpsc::Sender<Utf8Bytes>,
}

impl WebsocketOutputPipe {
    pub fn new(
        mut write: SplitSink<WebSocketStream<hyper_util::rt::TokioIo<Upgraded>>, Message>,
    ) -> Self {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(2048);

        // Spawn a task to handle sending messages
        tokio::spawn(async move {
            while let Some(bytes) = receiver.recv().await {
                if let Err(e) = write.send(Message::Text(bytes)).await {
                    eprintln!("Error sending websocket message: {:?}", e);
                }
            }
        });

        WebsocketOutputPipe {
            // websocket: Arc::new(Mutex::new(websocket)),
            sender,
        }
    }
}

impl IsTerminal for WebsocketOutputPipe {
    fn is_terminal(&self) -> bool {
        false
    }
}

#[async_trait::async_trait]
impl wasmtime_wasi::p2::OutputStream for WebsocketOutputPipe {
    fn write(&mut self, bytes: Bytes) -> Result<(), StreamError> {
        // Convert Bytes to string
        let data = std::str::from_utf8(&bytes).map_err(|e| {
            log::warn!("error converting bytes to string: {:?}", e);
            return StreamError::Closed; // TODO: Update error
        })?;

        // Send the bytes to the channel
        // NOTE: If the buffer is full, this will fail and skip sending the bytes
        // TODO: How to handle this gracefully?
        if let Err(e) = self.sender.try_send(data.into()) {
            log::warn!("error sending bytes to channel: {:?}", e);
            return Err(StreamError::Closed);
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), StreamError> {
        // TODO: Trigger the Message write here?
        // This stream is always flushed
        Ok(())
    }

    // TODO: Implement this
    fn check_write(&mut self) -> Result<usize, StreamError> {
        Ok(0)
    }
}

#[async_trait::async_trait]
impl wasmtime_wasi::p2::Pollable for WebsocketOutputPipe {
    async fn ready(&mut self) {}
}

impl AsyncWrite for WebsocketOutputPipe {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        // Convert bytes to string
        let data = std::str::from_utf8(buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Send the bytes to the channel
        match self.sender.try_send(data.into()) {
            Ok(()) => Poll::Ready(Ok(buf.len())),
            Err(mpsc::error::TrySendError::Full(_)) => {
                // Channel is full, would block
                Poll::Pending
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "websocket channel closed",
            ))),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        // WebSocket messages are immediately sent when written
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        // Nothing special needed for shutdown
        Poll::Ready(Ok(()))
    }
}

impl StdoutStream for WebsocketOutputPipe {
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(self.clone())
    }

    fn p2_stream(&self) -> Box<dyn wasmtime_wasi::p2::OutputStream> {
        Box::new(self.clone())
    }
}

#[derive(Debug)]
pub struct WebsocketInputPipe {
    closed: bool,
    buffer: Option<Result<Bytes, StreamError>>,
    receiver: mpsc::Receiver<Result<Bytes, StreamError>>,
    _join_handle: Option<wasmtime_wasi::runtime::AbortOnDropJoinHandle<()>>,
}

impl WebsocketInputPipe {
    pub fn new<T: tokio::io::AsyncRead + Send + Unpin + 'static>(mut reader: T) -> Self {
        // let (sender, receiver) = mpsc::channel(2048);
        let (sender, receiver) = mpsc::channel(2048);
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
}

#[async_trait::async_trait]
impl wasmtime_wasi::p2::InputStream for WebsocketInputPipe {
    fn read(&mut self, size: usize) -> wasmtime_wasi::p2::StreamResult<Bytes> {
        use mpsc::error::TryRecvError;

        match self.buffer.take() {
            Some(Ok(mut bytes)) => {
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
            Err(TryRecvError::Disconnected) => Err(StreamError::Trap(anyhow::anyhow!(
                "AsyncReadStream sender died - should be impossible"
            ))),
        }
    }
}

#[async_trait::async_trait]
impl wasmtime_wasi::p2::Pollable for WebsocketInputPipe {
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

pub struct WebSocketReader {
    stream: SplitStream<WebSocketStream<hyper_util::rt::TokioIo<Upgraded>>>,
    buffer: Bytes,
}

impl WebSocketReader {
    pub fn new(stream: SplitStream<WebSocketStream<hyper_util::rt::TokioIo<Upgraded>>>) -> Self {
        Self {
            stream,
            buffer: Bytes::new(),
        }
    }
}

impl AsyncRead for WebSocketReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        // If we have leftover data in buffer, copy that first
        if self.buffer.has_remaining() {
            let to_copy = std::cmp::min(self.buffer.len(), buf.remaining());
            let bytes = self.buffer.split_to(to_copy);
            buf.put_slice(&bytes);
            return Poll::Ready(Ok(()));
        }

        // Otherwise, poll the stream for the next message
        match Pin::new(&mut self.stream).poll_next(cx) {
            Poll::Ready(Some(Ok(Message::Binary(data)))) => {
                self.buffer = data;
                self.poll_read(cx, buf)
            }
            Poll::Ready(Some(Ok(Message::Text(text)))) => {
                let bytes = Bytes::copy_from_slice(text.as_bytes());
                self.buffer = bytes;
                self.poll_read(cx, buf)
            }
            Poll::Ready(Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_)))) => {
                // Skip control frames and keep polling
                self.poll_read(cx, buf)
            }
            Poll::Ready(Some(Ok(Message::Close(_)))) | Poll::Ready(None) => {
                // End of stream
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, e)))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
