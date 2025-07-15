use std::path::PathBuf;

use compact_str::ToCompactString;
use directories::BaseDirs;

use crate::{
    glim_app::GlimConfig,
    result::{GlimError, Result},
};

pub fn default_config_path() -> PathBuf {
    if let Some(dirs) = BaseDirs::new() {
        dirs.config_dir().join("glim.toml")
    } else {
        PathBuf::from("glim.toml")
    }
}

pub fn save_config(config_file: &PathBuf, config: GlimConfig) -> Result<()> {
    confy::store_path(config_file, &config)
        .map_err(|e| GlimError::ConfigError(e.to_compact_string()))?;

    Ok(())
}
