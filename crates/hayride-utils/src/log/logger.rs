use env_logger::{Builder, Env, Logger};
use log::{Level, LevelFilter};
use log_reload::{ReloadHandle, ReloadLog};
use std::sync::OnceLock;
use std::{env, fs};
use toml::Value;

static LOG_HANDLE: OnceLock<ReloadHandle<log_reload::LevelFilter<Logger>>> = OnceLock::new();

/// Initializes the logger with a specific log level for the workspace crates.
/// Will only initialize once, even if called multiple times to prevent multiple env logger initialization
pub fn init_logger(log_level: String) -> anyhow::Result<()> {
    let workspace_crates = get_workspace_crates();

    // Get the log level from the string
    let level = match log_level.to_lowercase().as_str() {
        "error" => Level::Error,
        "warn" => Level::Warn,
        "info" => Level::Info,
        "debug" => Level::Debug,
        "trace" => Level::Trace,
        _ => Level::Info, // Default to Info if invalid level is provided
    };

    let level_filter = match level {
        log::Level::Error => LevelFilter::Error,
        log::Level::Warn => LevelFilter::Warn,
        log::Level::Info => LevelFilter::Info,
        log::Level::Debug => LevelFilter::Debug,
        log::Level::Trace => LevelFilter::Trace,
    };

    // Get or init the log handle with the specified log level
    let log_handle = LOG_HANDLE.get_or_init(|| {
        let mut builder = Builder::from_env(Env::default());

        // Set default level for dependencies
        builder.filter_level(LevelFilter::Warn);

        // Apply log level to the workspace crates
        for crate_name in workspace_crates.clone() {
            builder.filter_module(crate_name.as_str(), level_filter);
        }

        let logger = builder.build();
        log::set_max_level(level_filter);

        // Create a new logger that will filter the logs based on the max level
        let level_filter_logger = log_reload::LevelFilter::new(level, logger);

        let reload_log = ReloadLog::new(level_filter_logger);
        let handle = reload_log.handle();

        // Register the logger to be used by the log crate
        if let Err(err) = log::set_boxed_logger(Box::new(reload_log)) {
            log::warn!("Failed to set the logger: {}", err);
        }

        return handle;
    });

    // Otherwise update the log level
    let mut builder = Builder::from_env(Env::default());

    // Set default level for dependencies
    builder.filter_level(LevelFilter::Warn);

    // Apply log level to the workspace crates
    for crate_name in workspace_crates {
        builder.filter_module(crate_name.as_str(), level_filter);
    }

    let logger = builder.build();
    log::set_max_level(level_filter);

    // Create a new logger that will filter the logs based on the max level
    let level_filter_logger = log_reload::LevelFilter::new(level, logger);

    return log_handle
        .replace(level_filter_logger)
        .map_err(|e| anyhow::anyhow!(e));
}

// Helper to get the module names of all crates in the workspace
// Uses the root Cargo.toml to find workspace members
// Then reads each member's Cargo.toml to get the actual package name if possible
// If not, it falls back to the directory name
fn get_workspace_crates() -> Vec<String> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let cargo_toml_path = format!("{}/Cargo.toml", manifest_dir);

    // Read and parse Cargo.toml
    if let Ok(contents) = fs::read_to_string(&cargo_toml_path) {
        if let Ok(cargo_toml) = toml::from_str::<Value>(&contents) {
            if let Some(workspace) = cargo_toml
                .get("workspace")
                .and_then(|w| w.as_table())
                .and_then(|t| t.get("members"))
                .and_then(|m| m.as_array())
            {
                let mut module_names = Vec::new();

                for member in workspace {
                    if let Some(path_str) = member.as_str() {
                        // Try to read the member's Cargo.toml to get the actual package name
                        let member_cargo_path = format!("{}/{}/Cargo.toml", manifest_dir, path_str);
                        if let Ok(member_contents) = fs::read_to_string(&member_cargo_path) {
                            if let Ok(member_toml) = toml::from_str::<Value>(&member_contents) {
                                if let Some(package) = member_toml
                                    .get("package")
                                    .and_then(|p| p.as_table())
                                    .and_then(|t| t.get("name"))
                                    .and_then(|n| n.as_str())
                                {
                                    // Convert hyphens to underscores for Rust module compatibility
                                    let module_name = package.replace('-', "_");
                                    module_names.push(module_name);
                                    continue;
                                }
                            }
                        }

                        // Fallback: if we can't get the actual name, use the directory name
                        if let Some(name) = path_str.split('/').last() {
                            let module_name = name.replace('-', "_");
                            module_names.push(module_name);
                        }
                    }
                }

                return module_names;
            }
        }
    }

    Vec::new() // Return empty if parsing fails
}
