pub mod ai;
pub mod bindings;
pub mod core;
pub mod db;
pub mod engine;
pub mod mcp;
pub mod server;
pub mod silo;
pub mod wac;
pub mod websocket;

use crate::ai::{AiCtx, AiView};
use crate::core::{CoreCtx, CoreView};
use crate::db::{DBCtx, DBView};
use crate::mcp::{McpCtx, McpView};
use crate::silo::{SiloCtx, SiloView};
use crate::wac::{WacCtx, WacView};

use uuid::Uuid;
use wasmtime::component::ResourceTable;
use wasmtime_wasi::cli::{InputFile, OutputFile};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

pub struct Host {
    ctx: WasiCtx,
    http_ctx: WasiHttpCtx,
    core_ctx: CoreCtx,
    ai_ctx: AiCtx,
    mcp_ctx: McpCtx,
    silo_ctx: SiloCtx,
    wac_ctx: WacCtx,
    db_ctx: DBCtx,
    table: ResourceTable,
}

impl WasiView for Host {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

impl WasiHttpView for Host {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http_ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl CoreView for Host {
    fn ctx(&mut self) -> &mut CoreCtx {
        &mut self.core_ctx
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl AiView for Host {
    fn ctx(&mut self) -> &mut AiCtx {
        &mut self.ai_ctx
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl McpView for Host {
    fn ctx(&mut self) -> &mut McpCtx {
        &mut self.mcp_ctx
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl SiloView for Host {
    fn ctx(&mut self) -> &mut SiloCtx {
        &mut self.silo_ctx
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl WacView for Host {
    fn ctx(&mut self) -> &mut WacCtx {
        &mut self.wac_ctx
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl DBView for Host {
    fn ctx(&mut self) -> &mut DBCtx {
        &mut self.db_ctx
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

fn create_wasi_ctx(
    args: &[impl AsRef<str> + std::marker::Sync],
    out_dir: Option<String>,
    id: Uuid,
    stdin: bool,
    envs: &[(impl AsRef<str>, impl AsRef<str>)],
) -> wasmtime::Result<WasiCtx> {
    let hayride_dir = hayride_utils::paths::hayride::default_hayride_dir()?;
    let hayride_dir_str = hayride_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert hayride dir to string"))?;

    let mut binding = WasiCtxBuilder::new();
    let mut wasi_ctx_builder = binding
        .args(args)
        .inherit_stderr()
        .inherit_stdio() // Default inherit stdout
        .env("PWD", ".") // Set the current working directory
        .envs(envs) // append custom envs
        .preopened_dir(
            ".",
            ".",
            wasmtime_wasi::DirPerms::all(),
            wasmtime_wasi::FilePerms::all(),
        )?
        .preopened_dir(
            hayride_dir_str,
            "/.hayride",
            wasmtime_wasi::DirPerms::all(),
            wasmtime_wasi::FilePerms::all(),
        )?;

    if let Some(out_dir) = out_dir {
        let output_path = out_dir.clone() + "/" + &id.to_string() + "/out";
        let error_path = out_dir.clone() + "/" + &id.to_string() + "/err";

        // Create the dir if it doesn't exist
        std::fs::create_dir_all(out_dir.clone() + "/" + &id.to_string())
            .expect("Failed to create output directory for thread");

        let out_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(output_path.clone())
            .expect("Failed to open output file stdout");

        let err_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(error_path.clone())
            .expect("Failed to open error file for stderr");

        let output_file = OutputFile::new(
            out_file
                .try_clone()
                .map_err(|e| anyhow::anyhow!("Failed to clone output file: {:?}", e))?,
        );
        wasi_ctx_builder = wasi_ctx_builder.stdout(output_file);

        let error_file = OutputFile::new(
            err_file
                .try_clone()
                .map_err(|e| anyhow::anyhow!("Failed to clone error file: {:?}", e))?,
        );
        wasi_ctx_builder = wasi_ctx_builder.stderr(error_file);

        if stdin {
            let input_path = out_dir.clone() + "/" + &id.to_string() + "/in";
            // Create the input file to be used for stdin
            let _in_file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .truncate(false)
                .open(input_path.clone())
                .expect("Failed to open input file");

            let file = std::fs::File::open(&input_path)?;
            let file_stdin = InputFile::new(file);

            wasi_ctx_builder = wasi_ctx_builder.stdin(file_stdin);
        }
    }

    let wasi_ctx: WasiCtx = wasi_ctx_builder.build();

    Ok(wasi_ctx)
}
