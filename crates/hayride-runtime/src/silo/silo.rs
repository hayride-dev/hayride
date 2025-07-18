use hayride_host_traits::silo::{Thread, ThreadStatus};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use uuid::Uuid;
use wasmtime::component::ResourceTable;
use wasmtime::Result;

use tokio::task::JoinHandle;

pub struct ThreadData {
    handle: Option<JoinHandle<()>>,
    metadata: Thread,
}

#[derive(Clone)]
pub struct SiloCtx {
    // The output directory for the runtime.
    pub out_dir: Option<String>,

    pub model_path: Option<String>,

    // A concurrent safe map of spawned threads by id.
    pub threads: Arc<dashmap::DashMap<Uuid, ThreadData>>,
    thread_id: Arc<AtomicI32>,
    pub registry_path: String,
}

impl SiloCtx {
    pub fn new(out_dir: Option<String>, registry_path: String, model_path: Option<String>) -> Self {
        let thread_id = Arc::new(AtomicI32::new(0));
        Self {
            out_dir,
            model_path,
            threads: Arc::new(dashmap::DashMap::new()),
            thread_id,
            registry_path: registry_path,
        }
    }

    pub fn next_thread_id(&self) -> Option<i32> {
        match self
            .thread_id
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| match v {
                ..=0x1ffffffe => Some(v + 1),
                _ => None,
            }) {
            Ok(v) => Some(v + 1),
            Err(_) => None,
        }
    }

    pub fn insert_thread(&self, id: Uuid, handle: Option<JoinHandle<()>>, metadata: Thread) {
        self.threads.insert(id, ThreadData { handle, metadata });
    }

    pub fn metadata(&self, thread_id: Uuid) -> Result<Thread, ErrNo> {
        self.threads
            .get(&thread_id)
            .map(|data| data.metadata.clone())
            .ok_or(ErrNo::ThreadNotFound)
    }

    pub fn threads(&self) -> Vec<Thread> {
        self.threads
            .iter()
            .map(|entry| entry.value().metadata.clone())
            .collect()
    }

    /// Waits for the task with the given ID to complete.
    pub async fn wait_for_thread(&self, thread_id: Uuid) -> Result<(), ErrNo> {
        if let Some(mut entry) = self.threads.get_mut(&thread_id) {
            // Take the handle out so we can await it
            if let Some(handle) = entry.handle.take() {
                match handle.await {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        log::warn!("thread {} failed: {:?}", thread_id, err);
                        Err(ErrNo::ThreadFailed)
                    }
                }
            } else {
                log::warn!("thread {} already awaited or never started", thread_id);
                Err(ErrNo::ThreadNotFound)
            }
        } else {
            Err(ErrNo::ThreadNotFound)
        }
    }

    /// Kills the task with the given ID.
    pub fn kill_thread(&self, thread_id: Uuid) -> Result<(), ErrNo> {
        if let Some(mut data) = self.threads.get_mut(&thread_id) {
            if let Some(handle) = data.handle.take() {
                handle.abort(); // Correctly call abort on the JoinHandle.
                data.metadata.status = ThreadStatus::Killed; // Update the status to Killed.
                log::debug!("thread {} has been aborted", thread_id);
                Ok(())
            } else {
                log::warn!("thread {} has no active handle to abort", thread_id);
                Err(ErrNo::ThreadNotFound)
            }
        } else {
            Err(ErrNo::ThreadNotFound)
        }
    }

    pub fn update_status(&self, thread_id: Uuid, status: ThreadStatus) -> Result<()> {
        if let Some(mut data) = self.threads.get_mut(&thread_id) {
            data.metadata.status = status;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Thread not found"))
        }
    }

    pub fn update_output(&self, thread_id: Uuid, output: Vec<u8>) -> Result<()> {
        if let Some(mut data) = self.threads.get_mut(&thread_id) {
            data.metadata.output = output;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Thread not found"))
        }
    }
}

pub trait SiloView: Send {
    /// Returns a mutable reference to the silo context.
    fn ctx(&mut self) -> &mut SiloCtx;

    /// Returns a mutable reference to the silo resource table.
    fn table(&mut self) -> &mut ResourceTable;
}

impl<T: ?Sized + SiloView> SiloView for &mut T {
    fn ctx(&mut self) -> &mut SiloCtx {
        T::ctx(self)
    }

    fn table(&mut self) -> &mut ResourceTable {
        T::table(self)
    }
}

impl<T: ?Sized + SiloView> SiloView for Box<T> {
    fn ctx(&mut self) -> &mut SiloCtx {
        T::ctx(self)
    }

    fn table(&mut self) -> &mut ResourceTable {
        T::table(self)
    }
}

/// A concrete structure that all generated `Host` traits are implemented for.
///
/// This type serves as a small newtype wrapper to implement all of the `Host`
/// traits for `hayride:silo`. This type is internally used and is only needed if
/// you're interacting with `add_to_linker` functions generated by bindings
/// themselves (or `add_to_linker_get_host`).
///
/// This type is automatically used when using
/// [`add_to_linker_async`](crate::add_to_linker_async)
/// or
/// [`add_to_linker_sync`](crate::add_to_linker_sync)
/// and doesn't need to be manually configured.
#[repr(transparent)]
pub struct SiloImpl<T>(pub T);

impl<T: SiloView> SiloView for SiloImpl<T> {
    fn ctx(&mut self) -> &mut SiloCtx {
        self.0.ctx()
    }

    fn table(&mut self) -> &mut ResourceTable {
        self.0.table()
    }
}

pub enum ErrNo {
    UnknownErrno = 0,
    MissingHomedir = 1,
    MorphNotFound = 2,
    InvalidThreadId = 3,
    ThreadNotFound = 4,
    ThreadFailed = 5,
    EngineError = 6,
    FailedToFindRegistry = 7,
    FailedToCreateLogDir = 8,
    FailedToCreateLogFile = 9,
    FailedToSpawnProcess = 10,
    FailedToCreateThreadResource = 11,
    Failed,
}

impl From<ErrNo> for u32 {
    fn from(code: ErrNo) -> u32 {
        code as u32
    }
}
