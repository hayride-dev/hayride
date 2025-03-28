use super::create_wasi_ctx;
use crate::bindings::hayride_ws::{HayrideWs, HayrideWsPre};
use crate::silo::SiloCtx;
use crate::Host;

use hayride_core::CoreBackend;

use anyhow::bail;

use hyper_tungstenite::tungstenite::Utf8Bytes;
use wasmtime_wasi::StreamError;
use wasmtime_wasi_http::{body::HyperOutgoingBody, WasiHttpCtx, WasiHttpView};

use bytes::Bytes;
use hyper::body::Body;
use hyper::upgrade::Upgraded;
use hyper_tungstenite::WebSocketStream;
use hyper_tungstenite::{tungstenite, HyperWebsocket};
use tungstenite::Message;
use uuid::Uuid;

use crate::ai::AiCtx;
use crate::core::CoreCtx;
use crate::wac::WacCtx;
use wasmtime::{component::ResourceTable, Result};

// Trait extensions
use futures::sink::SinkExt;
use futures::stream::{SplitSink, StreamExt};
use http_body_util::BodyExt;

pub struct WebsocketServer {
    id: Uuid,
    out_dir: Option<String>,
    ws_pre: HayrideWsPre<Host>,
    core_backend: CoreBackend,
    silo_ctx: SiloCtx,
    registry_path: String,
    model_path: Option<String>,
    args: Vec<String>,
}

impl WebsocketServer {
    pub fn new(
        id: Uuid,
        out_dir: Option<String>,
        ws_pre: HayrideWsPre<Host>,
        core_backend: CoreBackend,
        silo_ctx: SiloCtx,
        registry_path: String,
        model_path: Option<String>,
        args: Vec<String>,
    ) -> Self {
        Self {
            id,
            out_dir,
            ws_pre,
            core_backend,
            silo_ctx,
            registry_path,
            model_path,
            args,
        }
    }

    pub async fn handle_request(
        &self,
        mut req: hyper::Request<hyper::body::Incoming>,
    ) -> Result<hyper::Response<HyperOutgoingBody>> {
        // Check if this is a websocket request and handle it
        if hyper_tungstenite::is_upgrade_request(&req) {
            let wasi_ctx = create_wasi_ctx(&self.args, self.out_dir.clone(), self.id)?;
            let mut store: wasmtime::Store<Host> = wasmtime::Store::new(
                &self.ws_pre.engine(),
                Host {
                    ctx: wasi_ctx,
                    http_ctx: WasiHttpCtx::new(),
                    core_ctx: CoreCtx::new(self.core_backend.clone()),
                    ai_ctx: AiCtx::new(self.out_dir.clone(), self.model_path.clone()),
                    silo_ctx: self.silo_ctx.clone(),
                    wac_ctx: WacCtx::new(self.registry_path.clone()),
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
    // Get the parts from the request
    // TODO: Do we care about the initial body?
    // let (parts, body) = req.into_parts();

    let websocket: WebSocketStream<hyper_util::rt::TokioIo<Upgraded>> = websocket.await?;
    let (write, mut read) = websocket.split();
    let out = WebsocketOutputPipe::new(write);

    while let Some(message) = read.next().await {
        match message? {
            Message::Text(msg) => {
                let boxed: Box<dyn wasmtime_wasi::HostOutputStream> = Box::new(out.clone());
                let arg = store.data_mut().table().push(boxed)?;

                if let Err(e) = server
                    .hayride_socket_websocket()
                    .call_handle(&mut store, &msg, arg)
                    .await
                {
                    log::warn!("error handling websocket request: {:?}", e);
                    continue;
                }
            }
            Message::Binary(msg) => {
                log::debug!("received binary message: {msg:02X?}");
                // write.send(Message::binary(b"Thank you, come again.".to_vec())).await?;
            }
            Message::Ping(msg) => {
                // No need to send a reply: tungstenite takes care of this for you.
                log::debug!("received ping message: {msg:02X?}");
            }
            Message::Pong(msg) => {
                log::debug!("received pong message: {msg:02X?}");
            }
            Message::Close(msg) => {
                // No need to send a reply: tungstenite takes care of this for you.
                if let Some(msg) = &msg {
                    log::debug!(
                        "received close message with code {} and message: {}",
                        msg.code,
                        msg.reason
                    );
                } else {
                    log::debug!("received close message");
                }
            }
            Message::Frame(_msg) => {
                unreachable!();
            }
        }
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

#[async_trait::async_trait]
impl wasmtime_wasi::HostOutputStream for WebsocketOutputPipe {
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
impl wasmtime_wasi::Subscribe for WebsocketOutputPipe {
    async fn ready(&mut self) {}
}

impl wasmtime_wasi::StdoutStream for WebsocketOutputPipe {
    fn stream(&self) -> Box<dyn wasmtime_wasi::HostOutputStream> {
        Box::new(self.clone())
    }

    fn isatty(&self) -> bool {
        false
    }
}
