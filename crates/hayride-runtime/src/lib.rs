pub mod ai;
pub mod bindings;
pub mod engine;
pub mod server;
pub mod silo;
pub mod wac;
pub mod websocket;

use crate::ai::{AiCtx, AiView};
use crate::silo::{SiloCtx, SiloView};
use crate::wac::{WacCtx, WacView};

use async_trait::async_trait;
use bytes::Bytes;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;
use wasmtime::component::ResourceTable;
use wasmtime_wasi::{
    HostInputStream, OutputFile, StdinStream, StreamError, StreamResult, WasiCtxBuilder,
};
use wasmtime_wasi::{WasiCtx, WasiView};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

pub struct Host {
    ctx: WasiCtx,
    http_ctx: WasiHttpCtx,
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
    envs: &[(impl AsRef<str>, impl AsRef<str>)],
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

            let file_stdin = FileStdin::new(std::path::PathBuf::from(&input_path));

            wasi_ctx_builder = wasi_ctx_builder.stdin(file_stdin);
        }
    }

    let wasi_ctx: WasiCtx = wasi_ctx_builder.build();

    Ok(wasi_ctx)
}

/// Represents the StdinStream (a factory for producing input streams)
struct FileStdin {
    path: PathBuf, // Path to reopen file if needed
}

impl FileStdin {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl StdinStream for FileStdin {
    fn stream(&self) -> Box<dyn HostInputStream> {
        Box::new(FileHostInputStream {
            path: self.path.clone(),
            position: Arc::new(Mutex::new(0)), // Track position across reads
        })
    }

    fn isatty(&self) -> bool {
        false
    }
}

/// Our real file-based input stream
pub struct FileHostInputStream {
    path: PathBuf,
    position: Arc<Mutex<u64>>, // Track position across reads
}

#[async_trait]
impl wasmtime_wasi::Subscribe for FileHostInputStream {
    async fn ready(&mut self) {
        let path = self.path.clone();
        let pos = self.position.clone();

        loop {
            // Try to get metadata
            match std::fs::metadata(&path) {
                Ok(metadata) => {
                    let file_size = metadata.len();

                    // Try to lock position
                    if let Ok(position) = pos.lock() {
                        if *position < file_size {
                            // There is new data available
                            return;
                        }
                    } else {
                        // Couldn't lock position; treat as not ready
                    }
                }
                Err(_) => {
                    // Could not get metadata; treat as not ready yet
                }
            }

            // No data available yet, wait a bit
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }
}

#[async_trait]
impl HostInputStream for FileHostInputStream {
    fn read(&mut self, size: usize) -> StreamResult<Bytes> {
        let pos = self.position.clone();
        let path = self.path.clone();

        let mut position = pos.lock().map_err(|_| StreamError::Closed)?;
        let metadata = std::fs::metadata(&path).map_err(|_| StreamError::Closed)?;
        let file_size = metadata.len();

        if *position >= file_size {
            return Ok(Bytes::new());
        }

        // Reopen the file to read from it
        let mut file = std::fs::File::open(&path).map_err(|_| StreamError::Closed)?;

        file.seek(SeekFrom::Start(*position))
            .map_err(|_| StreamError::Closed)?;

        let bytes_available = (file_size - *position) as usize;
        let to_read = std::cmp::min(size, bytes_available);

        let mut buf = vec![0; to_read];
        let n = file.read(&mut buf).map_err(|_| StreamError::Closed)?;

        if n == 0 {
            return Ok(Bytes::new());
        }

        *position += n as u64;

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
