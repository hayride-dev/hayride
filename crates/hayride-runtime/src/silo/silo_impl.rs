use super::silo::ErrNo;
use crate::silo::bindings::{process, threads};
use crate::silo::{SiloImpl, SiloView};

use hayride_host_traits::silo::{Thread, ThreadStatus};

use std::fs::{self, File};
use std::io::{Read, Write};
use std::process::Command;
use uuid::Uuid;

use wasmtime::component::Resource;

#[cfg(unix)]
use nix::sys::signal::Signal;
#[cfg(unix)]
use nix::unistd::Pid;

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::{
        GetExitCodeProcess, OpenProcess, TerminateProcess, WaitForSingleObject,
        PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
    },
};

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
        wait_impl(pid)
    }

    fn status(&mut self, pid: u32) -> Result<bool, process::ErrNo> {
        status_impl(pid)
    }

    fn kill(&mut self, pid: u32, sig: i32) -> Result<i32, process::ErrNo> {
        kill_impl(pid, sig)
    }
}

#[cfg(unix)]
fn wait_impl(pid: u32) -> Result<i32, process::ErrNo> {
    let pid = Pid::from_raw(pid as i32);
    match nix::sys::wait::waitpid(pid, None) {
        Ok(status) => {
            log::debug!("process {:?} finished: {:?}", pid, status);
            Ok(pid.as_raw())
        }
        Err(e) => Err(e as u32),
    }
}

#[cfg(unix)]
fn status_impl(pid: u32) -> Result<bool, process::ErrNo> {
    let pid = Pid::from_raw(pid as i32);
    match nix::sys::signal::kill(pid, None) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[cfg(unix)]
fn kill_impl(pid: u32, sig: i32) -> Result<i32, process::ErrNo> {
    let pid = Pid::from_raw(pid as i32);
    let signal = Signal::try_from(sig).map_err(|e| e as u32)?;

    nix::sys::signal::kill(pid, signal).map_err(|e| e as u32)?;

    Ok(pid.as_raw())
}

#[cfg(windows)]
fn wait_impl(pid: u32) -> Result<i32, process::ErrNo> {
    unsafe {
        let handle: HANDLE = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            return Err(1);
        }

        match WaitForSingleObject(handle, u32::MAX) {
            WAIT_OBJECT_0 => Ok(pid as i32),
            _ => Err(2),
        }
    }
}

#[cfg(windows)]
fn status_impl(pid: u32) -> Result<bool, process::ErrNo> {
    unsafe {
        let handle: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return Ok(false); // process likely exited
        }

        let mut exit_code: u32 = 0;
        let success = GetExitCodeProcess(handle, &mut exit_code as *mut u32);
        if success == 0 {
            return Err(3);
        }

        // STILL_ACTIVE = 259; means the process is still running
        Ok(exit_code == 259)
    }
}

#[cfg(windows)]
fn kill_impl(pid: u32, _sig: i32) -> Result<i32, process::ErrNo> {
    unsafe {
        let handle: HANDLE = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if handle.is_null() {
            return Err(1);
        }

        let success = TerminateProcess(handle, 1);
        if success == 0 {
            return Err(2);
        }

        Ok(pid as i32)
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
                        let output_path = out_dir.clone() + "/" + &id.to_string() + "/out";
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
        mut args: Vec<String>,
    ) -> Result<Resource<Thread>, threads::ErrNo> {
        log::debug!(
            "executing spawn: {} with function: {}, and args: {:?}",
            morph,
            function,
            args
        );

        // add the morph as the first argument
        args.insert(0, morph.clone());

        let mut path = hayride_utils::paths::hayride::default_hayride_dir().map_err(|_err| {
            return ErrNo::MissingHomedir;
        })?;
        path.push(self.ctx().registry_path.clone());
        let path = hayride_utils::paths::registry::find_morph_path(
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
        .out_dir(out_dir.clone())
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
            match engine
                .run(path.clone(), function.clone(), &args.clone())
                .await
            {
                Ok(result) => {
                    // If out_dir is set, write a result file
                    if let Some(out_dir) = &out_dir {
                        // Create the output directory if it doesn't exist
                        let output_path =
                            out_dir.clone() + "/" + &thread_id.to_string() + "/result";
                        match File::create(output_path) {
                            Ok(mut file) => {
                                // Write the result to the file
                                if let Err(e) = file.write_all(&result) {
                                    log::warn!("Failed to write to output file: {:?}", e);
                                }
                            }
                            Err(e) => {
                                log::warn!("Failed to create output file: {:?}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    // If the engine fails, log the error
                    log::warn!(
                        "error running component {:?} with function: {:?} and args: {:?}: {:?}",
                        path,
                        function,
                        args,
                        e
                    );
                }
            }

            // Update the thread status to Exited
            ctx.update_status(thread_id, ThreadStatus::Exited)
                .map_err(|err| {
                    log::warn!("error updating thread status after exiting: {:?}", err);
                })
                .unwrap_or_default();
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
