use std::path::PathBuf;

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
        .map_err(|e| GlimError::config_save_error(config_file.clone(), e))?;

    Ok(())
}
