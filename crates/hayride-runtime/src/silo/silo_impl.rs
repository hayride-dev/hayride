use super::silo::ErrNo;
use crate::silo::bindings::{process, threads};
use crate::silo::{SiloImpl, SiloView};

use hayride_host_traits::silo::{Thread, ThreadStatus};

use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::fs::{self, File};
use std::io::Read;
use std::process::Command;
use uuid::Uuid;

use wasmtime::component::Resource;

impl<T> process::Host for SiloImpl<T>
where
    T: SiloView,
{
    fn spawn(
        &mut self,
        name: String,
        args: Vec<String>,
        envs: Vec<(String, String)>,
    ) -> Result<i32, process::ErrNo> {
        let mut cmd = Command::new(name);
        cmd.args(args);

        // Add environment variables to the command
        for (key, value) in envs {
            cmd.env(key, value);
        }

        // Spawn a process and return the pid
        let child = cmd.spawn().map_err(|_| ErrNo::FailedToSpawnProcess)?;

        Ok(child.id() as i32)
    }

    fn wait(&mut self, pid: u32) -> Result<i32, process::ErrNo> {
        // Wait for process with pid to finish
        let pid = Pid::from_raw(pid as i32);
        match nix::sys::wait::waitpid(pid, None) {
            Ok(status) => {
                log::debug!(
                    "process with pid: {:?} finished with status: {:?}",
                    pid,
                    status
                );
                Ok(pid.as_raw() as i32)
            }
            Err(e) => {
                return Err(e as u32);
            }
        }
    }

    fn status(&mut self, pid: u32) -> Result<bool, process::ErrNo> {
        // Check if the process with pid is still running
        let pid = Pid::from_raw(pid as i32);
        match kill(pid, None) {
            Ok(_) => Ok(true),
            Err(_) => {
                // Process is not running
                return Ok(false);
            }
        }
    }

    fn kill(&mut self, pid: u32, sig: i32) -> Result<i32, process::ErrNo> {
        let pid = Pid::from_raw(pid as i32);
        // Send the SIGKILL signal to terminate the process
        let signal = match Signal::try_from(sig) {
            Ok(s) => s,
            Err(e) => {
                return Err(e as u32);
            }
        };

        match kill(pid, signal) {
            Ok(_) => Ok(pid.as_raw() as i32),
            Err(e) => {
                return Err(e as u32);
            }
        }
    }
}

impl<T> threads::HostThread for SiloImpl<T>
where
    T: SiloView,
{
    fn id(&mut self, thread: Resource<Thread>) -> Result<String, threads::ErrNo> {
        let thread = self.table().get(&thread).map_err(|_| {
            return ErrNo::ThreadNotFound;
        })?;

        Ok(thread.id.clone())
    }

    fn wait(&mut self, thread: Resource<Thread>) -> Result<Vec<u8>, threads::ErrNo> {
        let thread = self.table().get(&thread).map_err(|_| {
            return ErrNo::ThreadNotFound;
        })?;

        let id = Uuid::parse_str(&thread.id.clone()).map_err(|_err| {
            return ErrNo::InvalidThreadId;
        })?;

        // Wait for the thread to complete
        tokio::task::block_in_place(|| {
            tokio::runtime::Runtime::new()
                .map_err(|_| ErrNo::EngineError)?
                .block_on(async {
                    let _ = self.ctx().wait_for_thread(id).await?;

                    if let Some(out_dir) = &self.ctx().out_dir {
                        // Read the output file and return the contents as bytes
                        let output_path = out_dir.clone() + "/" + &id.to_string() + "/out.txt";
                        let result = get_file_as_byte_vec(&output_path);

                        return Ok(result);
                    }

                    return Ok(vec![]);
                })
        })
    }

    fn drop(&mut self, thread: Resource<Thread>) -> wasmtime::Result<()> {
        self.table().delete(thread)?;
        Ok(())
    }
}

impl<T> threads::Host for SiloImpl<T>
where
    T: SiloView,
{
    fn spawn(
        &mut self,
        morph: String,
        function: String,
        args: Vec<String>,
    ) -> Result<Resource<Thread>, threads::ErrNo> {
        log::debug!(
            "executing spawn: {} with function: {}, and args: {:?}",
            morph,
            function,
            args
        );

        let mut path = dirs::home_dir().ok_or_else(|| ErrNo::MissingHomedir)?;
        path.push(self.ctx().registry_path.clone());
        let path = hayride_utils::morphs::registry::find_morph_path(
            path.to_str()
                .ok_or_else(|| ErrNo::FailedToFindRegistry)?
                .to_string(),
            morph.as_str(),
        )
        .map_err(|_err| {
            return ErrNo::MorphNotFound;
        })?;

        let core_backend = self.ctx().core_backend.clone();
        let out_dir = self.ctx().out_dir.clone();
        let model_path = self.ctx().model_path.clone();

        // Setup the engine
        let wasmtime_engine = wasmtime::Engine::new(
            wasmtime::Config::new()
                .wasm_component_model(true)
                .async_support(true),
        )
        .map_err(|_err| {
            return ErrNo::EngineError;
        })?;
        let engine = crate::engine::EngineBuilder::new(
            wasmtime_engine,
            core_backend,
            self.ctx().registry_path.clone(),
        )
        .out_dir(out_dir)
        .model_path(model_path)
        .core_enabled(true)
        .ai_enabled(true)
        // Disable silo for spawned morphs
        .silo_enabled(false)
        .wac_enabled(true)
        .wasi_enabled(true)
        .build()
        .map_err(|_err| {
            return ErrNo::EngineError;
        })?;

        log::debug!("Running engine with id: {}", engine.id);
        let thread_id = engine.id;

        // Create the Thread resource
        let thread = Thread {
            id: thread_id.to_string(),
            pkg: morph,
            function: function.clone(),
            args: args.clone(),
            status: ThreadStatus::Processing,
        };

        let ctx = self.ctx().clone();
        // run engine in a separate thread
        let handle: tokio::task::JoinHandle<()> = tokio::task::spawn(async move {
            if let Err(e) = engine.run(path, function, &args).await {
                log::warn!("error running component: {:?}", e);
            }

            // Kill thread after completion
            ctx.kill_thread(thread_id).unwrap_or_else(|_| {
                log::warn!("failed to kill thread after run completed {}", thread_id);
            });
        });

        // Insert the thread handle into the thread map
        self.ctx().insert_thread(thread_id, handle, thread.clone());

        // Push the thread resource to the table
        let id = self.table().push(thread).map_err(|_| {
            return ErrNo::FailedToCreateThreadResource;
        })?;

        // Return Thread resource ID
        Ok(id)
    }

    fn status(&mut self, thread_id: String) -> Result<threads::ThreadMetadata, threads::ErrNo> {
        let id = Uuid::parse_str(&thread_id).map_err(|_err| {
            return ErrNo::InvalidThreadId;
        })?;

        // Get the thread metadata
        let thread = self.ctx().metadata(id)?;

        let metadata = threads::ThreadMetadata {
            id: thread.id,
            pkg: thread.pkg,
            function: thread.function,
            args: thread.args,
            status: match thread.status {
                ThreadStatus::Unknown => threads::ThreadStatus::Unknown,
                ThreadStatus::Processing => threads::ThreadStatus::Processing,
                ThreadStatus::Exited => threads::ThreadStatus::Exited,
                ThreadStatus::Killed => threads::ThreadStatus::Killed,
            },
        };

        Ok(metadata)
    }

    fn kill(&mut self, thread_id: String) -> Result<(), threads::ErrNo> {
        let id = Uuid::parse_str(&thread_id).map_err(|_err| {
            return ErrNo::InvalidThreadId;
        })?;

        self.ctx().kill_thread(id)?;

        Ok(())
    }

    fn group(&mut self) -> Result<Vec<threads::ThreadMetadata>, threads::ErrNo> {
        // Get all threads in the silo
        let threads = self.ctx().threads();

        // Map the threads to ThreadMetadata
        let metadata: Vec<threads::ThreadMetadata> = threads
            .iter()
            .map(|thread| threads::ThreadMetadata {
                id: thread.id.clone(),
                pkg: thread.pkg.clone(),
                function: thread.function.clone(),
                args: thread.args.clone(),
                status: match thread.status {
                    ThreadStatus::Unknown => threads::ThreadStatus::Unknown,
                    ThreadStatus::Processing => threads::ThreadStatus::Processing,
                    ThreadStatus::Exited => threads::ThreadStatus::Exited,
                    ThreadStatus::Killed => threads::ThreadStatus::Killed,
                },
            })
            .collect();

        Ok(metadata)
    }
}

fn get_file_as_byte_vec(filename: &String) -> Vec<u8> {
    let mut f = File::open(&filename).expect("no file found");
    let metadata = fs::metadata(&filename).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    f.read(&mut buffer).expect("buffer overflow");

    buffer
}
