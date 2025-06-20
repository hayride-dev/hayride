use super::create_wasi_ctx;
use crate::bindings::hayride_server::{HayrideServer, HayrideServerPre};
use crate::silo::SiloCtx;
use crate::wac::WacCtx;
use crate::Host;

use anyhow::bail;

use uuid::Uuid;
use wasmtime_wasi_http::bindings::http::types::Scheme;
use wasmtime_wasi_http::{body::HyperOutgoingBody, WasiHttpCtx, WasiHttpView};

use crate::ai::AiCtx;
use wasmtime::{component::ResourceTable, Result};

pub struct Server {
    id: Uuid,
    out_dir: Option<String>,

    pre: HayrideServerPre<Host>,
    silo_ctx: SiloCtx,
    registry_path: String,
    model_path: Option<String>,
    args: Vec<String>,
}

impl Server {
    pub fn new(
        id: Uuid,
        out_dir: Option<String>,
        pre: HayrideServerPre<Host>,
        silo_ctx: SiloCtx,
        registry_path: String,
        model_path: Option<String>,
        args: Vec<String>,
    ) -> Self {
        Self {
            id,
            out_dir,
            pre,
            silo_ctx,
            registry_path,
            model_path,
            args,
        }
    }

    pub async fn handle_request(
        &self,
        req: hyper::Request<hyper::body::Incoming>,
    ) -> Result<hyper::Response<HyperOutgoingBody>> {
        let wasi_ctx = create_wasi_ctx(
            &self.args,
            self.out_dir.clone(),
            self.id,
            false,
            &[] as &[(&str, &str)],
        )?;
        let mut store: wasmtime::Store<Host> = wasmtime::Store::new(
            &self.pre.engine(),
            Host {
                ctx: wasi_ctx,
                http_ctx: WasiHttpCtx::new(),
                ai_ctx: AiCtx::new(self.out_dir.clone(), self.model_path.clone())?,
                silo_ctx: self.silo_ctx.clone(),
                wac_ctx: WacCtx::new(self.registry_path.clone()),
                table: ResourceTable::default(),
            },
        );

        // Instantiate the server
        let pre: HayrideServerPre<Host> = self.pre.clone();
        let proxy: HayrideServer = pre.instantiate_async(&mut store).await?;

        // Create a new incoming request and response outparam
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let req = store.data_mut().new_incoming_request(Scheme::Http, req)?;
        let out = store.data_mut().new_response_outparam(sender)?;

        // run the http request in separate task
        let task = tokio::task::spawn(async move {
            if let Err(e) = proxy
                .wasi_http_incoming_handler()
                .call_handle(&mut store, req, out)
                .await
            {
                return Err(e);
            }

            Ok(())
        });

        match receiver.await {
            Ok(Ok(mut resp)) => {
                // Add CORS headers to the response
                let headers = resp.headers_mut();
                if let Ok(origin) = "*".parse() {
                    headers.insert("Access-Control-Allow-Origin", origin);
                }
                if let Ok(methods) = "GET, POST, OPTIONS".parse() {
                    headers.insert("Access-Control-Allow-Methods", methods);
                }
                if let Ok(allowed_headers) = "*".parse() {
                    headers.insert("Access-Control-Allow-Headers", allowed_headers);
                }

                Ok(resp)
            }
            Ok(Err(e)) => Err(e.into()),

            // Otherwise the `sender` will get dropped along with the `Store`
            // meaning that the oneshot will get disconnected and here we can
            // inspect the `task` result to see what happened
            Err(_) => {
                let e = match task.await {
                    Ok(r) => r.unwrap_err(),
                    Err(e) => e.into(),
                };
                bail!("guest never invoked `response-outparam::set` method: {e:?}")
            }
        }
    }
}
