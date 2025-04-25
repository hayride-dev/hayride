pub mod ai;
pub mod bindings;
pub mod core;
pub mod engine;
pub mod server;
pub mod silo;
pub mod wac;
pub mod websocket;

use crate::ai::{AiCtx, AiView};
use crate::core::{CoreCtx, CoreView};
use crate::silo::{SiloCtx, SiloView};
use crate::wac::{WacCtx, WacView};

use uuid::Uuid;
use wasmtime::component::ResourceTable;
use wasmtime_wasi::{OutputFile, WasiCtxBuilder, StdinStream, StreamError, HostInputStream, StreamResult};
use wasmtime_wasi::{WasiCtx, WasiView};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::Mutex;
use bytes::Bytes;
use std::io::{Read, Seek, SeekFrom}; 
use std::fs::File;

pub struct Host {
    ctx: WasiCtx,
    http_ctx: WasiHttpCtx,
    core_ctx: CoreCtx,
    ai_ctx: AiCtx,
    silo_ctx: SiloCtx,
    wac_ctx: WacCtx,
    table: ResourceTable,
}

impl WasiView for Host {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
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

fn create_wasi_ctx(
    args: &[impl AsRef<str> + std::marker::Sync],
    out_dir: Option<String>,
    id: Uuid,
    stdin: bool,
) -> wasmtime::Result<WasiCtx> {
    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let hayride_dir = home_dir.join(".hayride");
    let hayride_dir_str = hayride_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert hayride dir to string"))?;

    let mut binding = WasiCtxBuilder::new();
    let mut wasi_ctx_builder = binding
        .args(args)
        .inherit_stderr()
        .inherit_stdio() // Default inherit stdout
        .env("PWD", ".") // Set the current working directory
        .env("HOME", home_dir.to_string_lossy())
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
        let output_path = out_dir.clone() + "/" + &id.to_string() + "/out.txt";
        let error_path = out_dir.clone() + "/" + &id.to_string() + "/err.txt";

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
            let input_path = out_dir.clone() + "/" + &id.to_string() + "/in.txt";
            let input_file = std::fs::OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(true)
                .open(input_path.clone())
                .expect("Failed to open input file for stdin");

            wasi_ctx_builder = wasi_ctx_builder.stdin(
                FileStdin::new(input_file, out_dir.clone() + "/" + &id.to_string() + "/in.txt"),
            );
        }
    }

    let wasi_ctx: WasiCtx = wasi_ctx_builder.build();

    Ok(wasi_ctx)
}


/// Represents the StdinStream (a factory for producing input streams)
struct FileStdin {
    name: String, // Optional name for debugging
    file: Arc<Mutex<File>>,
}

impl FileStdin {
    pub fn new(file: File, name: String) -> Self {
        Self {
            name,
            file: Arc::new(Mutex::new(file)),
        }
    }
}

impl StdinStream for FileStdin {
    fn stream(&self) -> Box<dyn HostInputStream> {
        Box::new(FileHostInputStream {
            name: self.name.clone(),
            file: self.file.clone(),
            position: Arc::new(Mutex::new(0)), // Track position across reads
        })
    }

    fn isatty(&self) -> bool {
        false
    }
}

/// Our real file-based input stream
pub struct FileHostInputStream {
    name: String, // Optional name for debugging
    file: Arc<Mutex<File>>,
    position: Arc<Mutex<u64>>, // Track position across reads
}

#[async_trait]
impl wasmtime_wasi::Subscribe for FileHostInputStream {
    async fn ready(&mut self) {
        // No-op: always "ready" for simplicity
        // simulate non-blocking by returning empty reads)
    }
}

#[async_trait]
impl HostInputStream for FileHostInputStream {
    fn read(&mut self, size: usize) -> StreamResult<Bytes> {
        let file = self.file.clone();
        let pos = self.position.clone();

        println!("{}: attempting to read {} bytes from file at position {}", self.name, size, *pos.lock().map_err(|_| StreamError::Closed)?);
    
        let mut file = file.lock().map_err(|_| StreamError::Closed)?;
        let mut position = pos.lock().map_err(|_| StreamError::Closed)?;
    
        (*file)
            .seek(SeekFrom::Start(*position))
            .map_err(|_| StreamError::Closed)?;

        let mut buf = vec![0; size];
        let n = file.read(&mut buf).map_err(|_| StreamError::Closed)?;
    
        if n == 0 {
            return Ok(Bytes::new());
        }
    
        *position += n as u64;

        println!("Read {} bytes from file at position {}", n, *position);
    
        Ok(Bytes::copy_from_slice(&buf[..n]))
    }

    async fn blocking_read(&mut self, size: usize) -> StreamResult<Bytes> {
        loop {
            let bytes = self.read(size)?;

            if bytes.is_empty() {
                // No data yet, wait a little and retry
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                continue;
            }

            println!("Blocking read got {} bytes", bytes.len());

            return Ok(bytes);
        }
    }

    fn skip(&mut self, nelem: usize) -> StreamResult<usize> {
        self.read(nelem).map(|bytes| bytes.len())
    }

    async fn blocking_skip(&mut self, nelem: usize) -> StreamResult<usize> {
        let bytes = self.blocking_read(nelem).await?;
        Ok(bytes.len())
    }

    async fn cancel(&mut self) {
        // No-op for now: nothing async that needs to be cancelled
    }
}