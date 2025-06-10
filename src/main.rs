use hayride_runtime::engine::EngineBuilder;
use std::env;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let hayride_dir = hayride_utils::paths::hayride::default_hayride_dir()?;
    let morphs_dir: String = "registry/morphs".to_string();
    let model_dir: String = "ai/models".to_string();

    // Setup logging
    // The ENV "HAYRIDE_LOG" can be used to set the log file path
    // otherwise fallback to $HOME/.hayride/logs/hayride.log
    let log_file: String = env::var("HAYRIDE_LOG").unwrap_or("hayride.log".to_string());
    // Put log in the hayride logs directory
    let mut log_dir = hayride_dir.clone();
    log_dir.push("logs");
    log_dir.push(log_file);
    let log_path = log_dir
        .to_str()
        .ok_or(anyhow::anyhow!("Failed to convert path to string"))?
        .to_string();

    hayride_utils::log::logger::set_log_path(log_path)?;

    let bin_path = env::var("HAYRIDE_BIN").unwrap_or("hayride-core:cli".to_string());
    let entrypoint = env::var("HAYRIDE_ENTRYPOINT").unwrap_or("run".to_string());
    let log_level = env::var("HAYRIDE_LOG_LEVEL").unwrap_or("info".to_string());

    // Output directory
    let mut out_dir = hayride_dir.clone();
    out_dir.push("sessions");
    let out_dir = out_dir
        .to_str()
        .ok_or(anyhow::anyhow!("Failed to convert path to string"))?
        .to_string();

    let wasmtime_engine = wasmtime::Engine::new(
        wasmtime::Config::new()
            .wasm_component_model(true)
            .async_support(true),
    )?;
    let engine = EngineBuilder::new(
        wasmtime_engine,
        hayride_core::CoreBackend::new(None),
        morphs_dir.clone(),
    )
    .log_level(log_level.clone())
    .out_dir(Some(out_dir)) // outdir set in context for spawned components
    .inherit_stdio(true) // Inherit stdio for the cli component
    .model_path(Some(model_dir))
    .core_enabled(true)
    .silo_enabled(true)
    .wac_enabled(true)
    .wasi_enabled(true)
    .ai_enabled(true)
    .envs(vec![
        ("HAYRIDE_LOG_LEVEL".to_string(), log_level.clone()),
        ("HAYRIDE_BIN".to_string(), bin_path.clone()),
        ("HAYRIDE_ENTRYPOINT".to_string(), entrypoint.clone()),
    ])
    .build()?;

    // Parse args to pass to the component
    let args: Vec<String> = env::args().collect();

    let mut morph_path = hayride_dir.clone();
    morph_path.push("registry");
    morph_path.push("morphs");
    let path_str = morph_path
        .to_str()
        .ok_or(anyhow::anyhow!("Failed to convert path to string"))?
        .to_string();

    // TODO: ENV for the cli morph name
    let wasm_file = hayride_utils::paths::registry::find_morph_path(path_str, &bin_path)?;

    if let Err(e) = engine.run(wasm_file, entrypoint.to_string(), &args).await {
        log::error!("Error running component: {:?}", e);
    }

    Ok(())
}
