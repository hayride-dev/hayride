use super::silo::ErrNo;
use crate::silo::bindings::{process, threads};
use crate::silo::{SiloImpl, SiloView};

use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::fs::{self, create_dir_all, File};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use uuid::Uuid;

impl<T> process::Host for SiloImpl<T>
where
    T: SiloView,
{
    fn spawn(&mut self, name: String, args: Vec<String>) -> Result<i32, process::ErrNo> {
        // Setup logging
        let log_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".hayride/logs");

        create_dir_all(&log_dir).map_err(|_| ErrNo::FailedToCreateLogDir)?;

        // Optionally make log filename dynamic (e.g., with timestamp or name)
        let stdout_log =
            File::create(log_dir.join("stdout.log")).map_err(|_| ErrNo::FailedToCreateLogFile)?;
        let stderr_log =
            File::create(log_dir.join("stderr.log")).map_err(|_| ErrNo::FailedToCreateLogFile)?;

        // Spawn a process and return the pid
        let child = Command::new(name)
            .args(args)
            // TODO: Rolling log to /.hayride/logs
            .stdout(Stdio::from(stdout_log))
            .stderr(Stdio::from(stderr_log))
            .spawn()
            .map_err(|_| ErrNo::FailedToSpawnProcess)?;

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

impl<T> threads::Host for SiloImpl<T>
where
    T: SiloView,
{
    fn spawn(
        &mut self,
        morph: String,
        function: String,
        args: Vec<String>,
    ) -> Result<String, threads::ErrNo> {
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
        .silo_enabled(true)
        // .wac_enabled(true) // TODO: Should wac be enabled for spawned morphs?
        .wasi_enabled(true)
        .build()
        .map_err(|_err| {
            return ErrNo::EngineError;
        })?;

        log::debug!("Running engine with id: {}", engine.id);
        let thread_id = engine.id;

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
        self.ctx().insert_thread(thread_id, handle);

        // Return Thread ID
        Ok(thread_id.into())
    }

    fn wait(&mut self, thread_id: String) -> Result<Vec<u8>, threads::ErrNo> {
        let id = Uuid::parse_str(&thread_id).map_err(|_err| {
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
                        let output_path =
                            out_dir.clone() + "/" + &thread_id.to_string() + "/out.txt";
                        let result = get_file_as_byte_vec(&output_path);

                        return Ok(result);
                    }

                    return Ok(vec![]);
                })
        })
    }

    fn status(&mut self, thread_id: String) -> Result<bool, threads::ErrNo> {
        let id = Uuid::parse_str(&thread_id).map_err(|_err| {
            return ErrNo::InvalidThreadId;
        })?;

        // Check if the thread is still running
        let is_running = self.ctx().exists(id);

        Ok(is_running)
    }

    fn kill(&mut self, thread_id: String) -> Result<(), threads::ErrNo> {
        let id = Uuid::parse_str(&thread_id).map_err(|_err| {
            return ErrNo::InvalidThreadId;
        })?;

        self.ctx().kill_thread(id)?;

        Ok(())
    }
}

fn get_file_as_byte_vec(filename: &String) -> Vec<u8> {
    let mut f = File::open(&filename).expect("no file found");
    let metadata = fs::metadata(&filename).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    f.read(&mut buffer).expect("buffer overflow");

    buffer
}
