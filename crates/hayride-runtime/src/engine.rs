use super::create_wasi_ctx;
use crate::ai::AiCtx;
use crate::bindings::hayride_cli::HayrideCliPre;
use crate::bindings::hayride_server::HayrideServerPre;
use crate::bindings::hayride_ws::HayrideWsPre;
use crate::core::CoreCtx;
use crate::server::Server;
use crate::silo::SiloCtx;
use crate::wac::WacCtx;
use crate::db::DBCtx;
use crate::websocket::WebsocketServer;
use crate::Host;

use hayride_utils::wit::parser::WitParser;

use wasmtime::component::types::ComponentItem;
use wasmtime::{
    component::{Component, ComponentExportIndex, Linker, ResourceTable},
    Result,
};
use wasmtime_wasi_http::io::TokioIo;
use wasmtime_wasi_http::WasiHttpCtx;

use hyper::server::conn::http1;
use std::fs::{self, File};
use std::path::Path;
use std::sync::Arc;
use std::{path::PathBuf, vec};
use tokio::net::TcpListener;
use url::Url;
use uuid::Uuid;

pub struct EngineBuilder {
    engine: wasmtime::Engine,
    // If out_dir is not set, will inherit stdio for wasmtime execution
    out_dir: Option<String>,
    registry_path: String,
    model_path: Option<String>,
    log_level: String,
    inherit_stdio: bool,
    envs: Vec<(String, String)>,

    ai_enabled: bool,
    silo_enabled: bool,
    wac_enabled: bool,
    wasi_enabled: bool,
    core_enabled: bool,
    db_enabled: bool,
}

impl EngineBuilder {
    pub fn new(engine: wasmtime::Engine, registry_path: String) -> Self {
        Self {
            engine,
            out_dir: None,
            registry_path,
            model_path: None,
            log_level: "info".to_string(),
            inherit_stdio: false,
            envs: vec![],

            ai_enabled: false,
            silo_enabled: false,
            wac_enabled: false,
            wasi_enabled: true,
            core_enabled: true,
            db_enabled: true,
        }
    }

    pub fn out_dir(mut self, out_dir: Option<String>) -> Self {
        self.out_dir = out_dir;
        self
    }

    pub fn registry_path(mut self, registry_path: String) -> Self {
        self.registry_path = registry_path;
        self
    }

    pub fn model_path(mut self, model_path: Option<String>) -> Self {
        self.model_path = model_path;
        self
    }

    pub fn log_level(mut self, log_level: String) -> Self {
        self.log_level = log_level;
        self
    }

    pub fn inherit_stdio(mut self, inherit_stdio: bool) -> Self {
        self.inherit_stdio = inherit_stdio;
        self
    }

    pub fn envs(mut self, envs: Vec<(String, String)>) -> Self {
        self.envs = envs;
        self
    }

    pub fn ai_enabled(mut self, ai_enabled: bool) -> Self {
        self.ai_enabled = ai_enabled;
        self
    }

    pub fn silo_enabled(mut self, silo_enabled: bool) -> Self {
        self.silo_enabled = silo_enabled;
        self
    }

    pub fn wac_enabled(mut self, wac_enabled: bool) -> Self {
        self.wac_enabled = wac_enabled;
        self
    }

    pub fn wasi_enabled(mut self, wasi_enabled: bool) -> Self {
        self.wasi_enabled = wasi_enabled;
        self
    }

    pub fn core_enabled(mut self, core_enabled: bool) -> Self {
        self.core_enabled = core_enabled;
        self
    }

    pub fn db_enabled(mut self, db_enabled: bool) -> Self {
        self.db_enabled = db_enabled;
        self
    }

    pub fn build(self) -> Result<WasmtimeEngine> {
        let id = Uuid::new_v4();

        // Check if out_dir is set, if so create the output and input files
        if let Some(ref out_dir) = self.out_dir {
            if !self.inherit_stdio {
                let base_dir = Path::new(out_dir).join(id.to_string());

                // Create dir if it does not exist
                fs::create_dir_all(&base_dir)?;

                let output_path = base_dir.join("out");
                let error_path = base_dir.join("err");
                let input_path = base_dir.join("in");

                File::create(output_path.clone())?;
                File::create(error_path.clone())?;
                File::create(input_path.clone())?;
            }
        }

        Ok(WasmtimeEngine {
            id: id,
            engine: self.engine,
            out_dir: self.out_dir,
            registry_path: self.registry_path,
            model_path: self.model_path,
            log_level: self.log_level,
            inherit_stdio: self.inherit_stdio,
            envs: self.envs,
            ai_enabled: self.ai_enabled,
            silo_enabled: self.silo_enabled,
            wac_enabled: self.wac_enabled,
            wasi_enabled: self.wasi_enabled,
            core_enabled: self.core_enabled,
            db_enabled: self.db_enabled,
        })
    }
}

pub struct WasmtimeEngine {
    pub id: Uuid,
    engine: wasmtime::Engine,
    out_dir: Option<String>,

    registry_path: String,
    model_path: Option<String>,
    log_level: String,

    inherit_stdio: bool,
    envs: Vec<(String, String)>,

    ai_enabled: bool,
    silo_enabled: bool,
    wac_enabled: bool,
    wasi_enabled: bool,
    core_enabled: bool,
    db_enabled: bool,
}

#[derive(Debug)]
enum ComponentType {
    Server,
    WebsocketServer,
    Cli,
    Reactor,
}

impl WasmtimeEngine {
    fn create_store(
        &self,
        args: &[impl AsRef<str> + std::marker::Sync],
        silo_ctx: SiloCtx,
        core_ctx: CoreCtx,
        mut stdin: bool,
    ) -> wasmtime::Result<wasmtime::Store<Host>> {
        let mut outdir = self.out_dir.clone();
        if self.inherit_stdio {
            // If inheriting stdio, don't create out dir or stdin files
            stdin = false;
            outdir = None;
        }

        let wasi_ctx = create_wasi_ctx(args, outdir, self.id, stdin, &self.envs)?;
        let store = wasmtime::Store::new(
            &self.engine,
            Host {
                ctx: wasi_ctx,
                http_ctx: WasiHttpCtx::new(),
                core_ctx: core_ctx.clone(),
                ai_ctx: AiCtx::new(self.out_dir.clone(), self.model_path.clone())?,
                silo_ctx: silo_ctx.clone(),
                wac_ctx: WacCtx::new(self.registry_path.clone()),
                db_ctx: DBCtx::new(),
                table: ResourceTable::default(),
            },
        );

        Ok(store)
    }

    // link imports will add the enabled interfaces to the linker
    // TODO: config to determine which interfaces are allowed
    fn link_imports(&self, wit: WitParser) -> wasmtime::Result<Linker<Host>> {
        // Create the linker and add enabled interfaces
        let mut linker: Linker<Host> = Linker::<Host>::new(&self.engine);

        let mut wasi: bool = false;
        let mut ai: bool = false;
        let mut silo: bool = false;
        let mut wac: bool = false;
        let mut core: bool = false;
        let mut db: bool = false;
        wit.imports().iter().for_each(|i| {
            match i.name.namespace.as_str() {
                "hayride" => match i.name.name.as_str() {
                    "silo" => silo = true,
                    "ai" => ai = true,
                    "wac" => wac = true,
                    "core" => core = true,
                    "db" => db = true,
                    _ => {
                        log::debug!("unknown import Found: {}", i.name.name);
                    }
                },
                "wasi" => {
                    wasi = true;
                    if i.name.name == "nn" {
                        // AI is required through wasi:nn or hayride:ai
                        ai = true;
                    }
                }
                _ => {
                    log::debug!("unknown import namespace: {}", i.name.namespace);
                }
            }
        });

        // Debug
        log::debug!("wasi import enabled: {:?}", wasi);
        log::debug!("ai import enabled: {:?}", ai);
        log::debug!("silo import enabled: {:?}", silo);
        log::debug!("wac import enabled: {:?}", wac);
        log::debug!("core import enabled: {:?}", core);

        if wasi {
            if !self.wasi_enabled {
                return Err(anyhow::anyhow!("WASI is not enabled").into());
            }

            wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
            // TODO: Look for http import separately
            wasmtime_wasi_http::add_only_http_to_linker_async(&mut linker)?;
        }

        if ai {
            if !self.ai_enabled {
                return Err(anyhow::anyhow!("AI is not enabled").into());
            }

            crate::ai::add_to_linker_sync(&mut linker)?;
        }

        if silo {
            if !self.silo_enabled {
                return Err(anyhow::anyhow!("Silo is not enabled").into());
            }

            crate::silo::add_to_linker_sync(&mut linker)?;
        }

        if wac {
            if !self.wac_enabled {
                return Err(anyhow::anyhow!("WAC is not enabled").into());
            }

            crate::wac::add_to_linker_sync(&mut linker)?;
        }

        if core {
            if !self.core_enabled {
                return Err(anyhow::anyhow!("Core is not enabled").into());
            }

            crate::core::add_to_linker_sync(&mut linker)?;
        }

        if db {
            if !self.db_enabled {
                return Err(anyhow::anyhow!("DB is not enabled").into());
            }

            crate::db::add_to_linker_sync(&mut linker)?;
        }

        return Ok(linker);
    }

    pub async fn run(
        self,
        wasm_file: PathBuf,
        function: String,
        args: &[impl AsRef<str> + std::marker::Sync],
    ) -> Result<Vec<u8>> {
        // Set initial logger based on builder
        hayride_utils::log::init_logger(self.log_level.clone())?;

        let bytes: Vec<u8> = std::fs::read(wasm_file)?;
        let component: Component = Component::from_binary(&self.engine, &bytes)?;

        // Use wit_component to decode into a wit definition
        let wit_parsed = WitParser::new(bytes)?;
        let linker = self.link_imports(wit_parsed.clone())?;

        // Default assume that a component is a reactor unless we find a handle or run function
        let mut component_type: ComponentType = ComponentType::Reactor;
        wit_parsed.function_exports().iter().for_each(|f| {
            match f.function.name.as_str() {
                "run" => {
                    component_type = ComponentType::Cli;
                }
                "handle" => {
                    // Check if interface name is "websocket"
                    if f.interface.as_ref().and_then(|i| i.name.as_deref()) == Some("websocket") {
                        component_type = ComponentType::WebsocketServer;
                    } else {
                        component_type = ComponentType::Server;
                    }
                }
                _ => {}
            }
        });

        let silo_ctx = SiloCtx::new(
            self.out_dir.clone(),
            self.registry_path.clone(),
            self.model_path.clone(),
        );

        let core_ctx = CoreCtx::new();

        // Handle component based on its type
        match component_type {
            ComponentType::Cli => {
                let mut store = self.create_store(args, silo_ctx.clone(), core_ctx, true)?;

                // TODO: Configuration for which bindings to use
                let pre: HayrideCliPre<Host> =
                    HayrideCliPre::new(linker.instantiate_pre(&component)?)?;
                let instance = pre.instantiate_async(&mut store).await?;

                // Execute the cli run function
                let result = instance.wasi_cli_run().call_run(&mut store).await?;
                log::info!("runtime executed: {result:?}");

                return Ok(vec![]);
            }
            ComponentType::Reactor => {
                let mut store = self.create_store(args, silo_ctx.clone(), core_ctx, true)?;

                // For Reactor, lookup the function to call and call it
                let pre: wasmtime::component::InstancePre<Host> =
                    linker.instantiate_pre(&component)?;
                let instance = pre.instantiate_async(&mut store).await?;

                // Look up the exported function
                let func_index = get_func_export(store.engine(), &component, function);
                let func_index = match func_index {
                    Some(i) => i,
                    None => {
                        return Err(anyhow::Error::msg("No Function Export Found"));
                    }
                };

                // Execute the exported function
                match instance.get_func(&mut store, func_index) {
                    Some(f) => {
                        // Ensure that the number of arguments match the function signature
                        if f.params(&mut store).len() != args.len() - 1 {
                            return Err(anyhow::Error::msg("Incorrect number of arguments"));
                        }

                        // Build the params using the args
                        // skipping first arg as it will be the function name (matching OS Args)
                        let mut index = 1;
                        let mut params = Vec::new();
                        for p in f.params(&mut store).iter() {
                            match p.1 {
                                wasmtime::component::Type::String => {
                                    params.push(wasmtime::component::Val::String(
                                        args[index].as_ref().to_string(),
                                    ));
                                }
                                wasmtime::component::Type::S32 => {
                                    params.push(wasmtime::component::Val::S32(
                                        args[index].as_ref().parse::<i32>()?,
                                    ));
                                }
                                wasmtime::component::Type::S64 => {
                                    params.push(wasmtime::component::Val::S64(
                                        args[index].as_ref().parse::<i64>()?,
                                    ));
                                }
                                wasmtime::component::Type::U32 => {
                                    params.push(wasmtime::component::Val::U32(
                                        args[index].as_ref().parse::<u32>()?,
                                    ));
                                }
                                wasmtime::component::Type::U64 => {
                                    params.push(wasmtime::component::Val::U64(
                                        args[index].as_ref().parse::<u64>()?,
                                    ));
                                }
                                wasmtime::component::Type::Bool => {
                                    params.push(wasmtime::component::Val::Bool(
                                        args[index].as_ref().parse::<bool>()?,
                                    ));
                                }
                                _ => {
                                    // TODO: Return error
                                    return Err(anyhow::Error::msg("Unknown Param Type"));
                                }
                            }
                            index += 1;
                        }

                        // Set results based on function signature
                        let mut results = Vec::new();
                        for r in f.results(&mut store) {
                            match r {
                                wasmtime::component::Type::String => {
                                    results.push(wasmtime::component::Val::String("".to_string()));
                                }
                                wasmtime::component::Type::S32 => {
                                    results.push(wasmtime::component::Val::S32(0));
                                }
                                wasmtime::component::Type::S64 => {
                                    results.push(wasmtime::component::Val::S64(0));
                                }
                                wasmtime::component::Type::U32 => {
                                    results.push(wasmtime::component::Val::U32(0));
                                }
                                wasmtime::component::Type::U64 => {
                                    results.push(wasmtime::component::Val::U64(0));
                                }
                                wasmtime::component::Type::Bool => {
                                    results.push(wasmtime::component::Val::Bool(false));
                                }
                                _ => {
                                    return Err(anyhow::Error::msg("Unknown Result Type"));
                                }
                            }
                        }

                        f.call_async(&mut store, &params, &mut results[..]).await?;

                        log::info!(
                            "function executed with args {:?} and got results: {:?}",
                            params,
                            results
                        );

                        // Return the results as Vec<u8>
                        for f in results {
                            match f {
                                wasmtime::component::Val::String(s) => {
                                    return Ok(s.into_bytes());
                                }
                                wasmtime::component::Val::S32(result) => {
                                    return Ok(result.to_string().into_bytes());
                                }
                                wasmtime::component::Val::S64(result) => {
                                    return Ok(result.to_string().into_bytes());
                                }
                                wasmtime::component::Val::U32(result) => {
                                    return Ok(result.to_string().into_bytes());
                                }
                                wasmtime::component::Val::U64(result) => {
                                    return Ok(result.to_string().into_bytes());
                                }
                                wasmtime::component::Val::Bool(result) => {
                                    return Ok(result.to_string().into_bytes());
                                }
                                _ => {
                                    return Err(anyhow::Error::msg("Unknown Result Type"));
                                }
                            }
                        }
                    }
                    None => {
                        log::warn!("no function found for export index {:?}", func_index);
                    }
                }

                return Ok(vec![]);
            }
            ComponentType::Server => {
                // For server, instantiate as server and start listening using component to handle requests
                let pre: HayrideServerPre<Host> =
                    HayrideServerPre::new(linker.instantiate_pre(&component)?)?;

                // Get config from server instance
                let mut store =
                    self.create_store(args, silo_ctx.clone(), core_ctx.clone(), false)?;
                let server = pre.instantiate_async(&mut store).await?;
                let config = match server.hayride_http_config().call_get(store).await? {
                    Ok(c) => {
                        log::debug!("server config: {:?}", c);
                        c
                    }
                    Err(e) => {
                        log::error!("failed to get server config: {:?}", e);
                        return Err(anyhow::Error::msg("failed to get server config"));
                    }
                };

                // Ensure the input has a scheme
                let address_with_scheme = if config.address.contains("://") {
                    config.address.clone()
                } else {
                    format!("http://{}", config.address)
                };

                let url = Url::parse(&address_with_scheme)
                    .map_err(|e| anyhow::anyhow!("Failed to parse URL: {}", e))?;

                // Parse url or use default values
                let host = url.host_str().unwrap_or("127.0.0.1");
                let port = url.port_or_known_default().unwrap_or(80);
                let address = format!("{}:{}", host, port);

                log::debug!("starting server with address: {}", address);

                // Prepare our server state and start listening for connections.
                let server = Arc::new(Server::new(
                    self.id,
                    self.out_dir.clone(),
                    pre,
                    silo_ctx,
                    core_ctx,
                    self.registry_path.clone(),
                    self.model_path.clone(),
                    args.iter().map(|s| s.as_ref().to_string()).collect(),
                    self.envs.clone(),
                ));
                let listener = TcpListener::bind(address).await?;

                // Start long running process
                loop {
                    let (client, addr) = listener.accept().await?;
                    log::debug!("accepted client from: {}", addr);

                    let server = server.clone();

                    // TODO: Set configured read/write timeouts and header limit

                    tokio::task::spawn(async move {
                        if let Err(e) = http1::Builder::new()
                            .keep_alive(true)
                            .serve_connection(
                                TokioIo::new(client),
                                hyper::service::service_fn(move |req| {
                                    let server = server.clone();
                                    async move { server.handle_request(req).await }
                                }),
                            )
                            .with_upgrades()
                            .await
                        {
                            log::error!("server error: {}", e);
                        }
                    });
                }
            }
            ComponentType::WebsocketServer => {
                let ws_pre: HayrideWsPre<Host> =
                    HayrideWsPre::new(linker.instantiate_pre(&component)?)?;

                // TODO: Add instance export for ws config
                let address = "127.0.0.1:8082".to_string(); // Default address

                log::debug!("starting websocket server with address: {}", address);

                // Prepare our server state and start listening for connections.
                let server = Arc::new(WebsocketServer::new(
                    self.id,
                    self.out_dir.clone(),
                    ws_pre,
                    silo_ctx,
                    core_ctx,
                    self.registry_path.clone(),
                    self.model_path.clone(),
                    args.iter().map(|s| s.as_ref().to_string()).collect(),
                    self.envs.clone(),
                ));
                let listener = TcpListener::bind(address).await?;

                // Start long running process
                loop {
                    let (client, addr) = listener.accept().await?;
                    log::debug!("accepted client from: {}", addr);

                    let server = server.clone();
                    tokio::task::spawn(async move {
                        if let Err(e) = http1::Builder::new()
                            .keep_alive(true)
                            .serve_connection(
                                TokioIo::new(client),
                                hyper::service::service_fn(move |req| {
                                    let server = server.clone();
                                    async move { server.handle_request(req).await }
                                }),
                            )
                            .with_upgrades()
                            .await
                        {
                            eprintln!("server error: {}", e);
                        }
                    });
                }
            }
        }
    }
}

// Lookup the exported function from the component
// assumes that there will only be one exported function
// TODO: Handle multiple functions AND nested instances
fn get_func_export(
    engine: &wasmtime::Engine,
    component: &Component,
    function: String,
) -> Option<ComponentExportIndex> {
    // Find the exported func index from the provided component
    // TODO: How to handle multiple functions in reactor components?
    // For example, if cli run and another export exist there is not a guarantee which will be returned
    let mut func: Option<ComponentExportIndex> = None;
    component
        .component_type()
        .exports(engine)
        .any(|e: (&str, ComponentItem)| {
            match component.get_export_index(None, e.0) {
                Some(instance_index) => {
                    match e.1 {
                        ComponentItem::ComponentFunc(_f) => {
                            let export = component.get_export_index(None, e.0);
                            match export {
                                Some(i) => {
                                    if e.0 == function {
                                        func = Some(i);
                                        return true;
                                    }
                                }
                                None => {
                                    log::debug!("no export found");
                                }
                            }
                            return false;
                        }
                        ComponentItem::ComponentInstance(i) => {
                            i.exports(engine).any(|e: (&str, ComponentItem)| {
                                match e.1 {
                                    ComponentItem::ComponentFunc(_f) => {
                                        // Lookup the export index for this function
                                        let export =
                                            component.get_export_index(Some(&instance_index), e.0);
                                        match export {
                                            Some(i) => {
                                                if e.0 == function {
                                                    func = Some(i);
                                                    return true;
                                                }
                                            }
                                            None => {
                                                log::debug!("no export found");
                                            }
                                        }
                                        return false;
                                    }
                                    unknown => {
                                        log::debug!("unknown export {:?}", unknown);
                                    }
                                }
                                return false;
                            });
                        }
                        unknown => {
                            log::debug!("unknown export {:?}", unknown);
                        }
                    }
                    return false;
                }
                None => {
                    log::debug!("no export found");
                    return false;
                }
            }
        });

    return func;
}
