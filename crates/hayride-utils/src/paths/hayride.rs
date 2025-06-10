use anyhow::{anyhow, Result};
use std::path::PathBuf;

pub fn default_hayride_dir() -> Result<PathBuf> {
    let base_dir = if cfg!(target_os = "windows") {
        dirs::data_dir().ok_or_else(|| anyhow!("Could not find local data directory"))?
    } else {
        dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?
    };

    Ok(base_dir.join(".hayride"))
}
