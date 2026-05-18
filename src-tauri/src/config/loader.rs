use std::path::PathBuf;

use anyhow::{Context, Result};

use super::model::Config;

fn config_dir() -> Result<PathBuf> {
    let base = dirs::config_dir().context("could not determine config directory")?;
    Ok(base.join("ridgeline"))
}

pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

pub fn load_config() -> Result<Config> {
    let path = config_path()?;

    if !path.exists() {
        return Ok(Config {
            general: Default::default(),
            providers: Vec::new(),
        });
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read config from {}", path.display()))?;

    let config: Config =
        toml::from_str(&content).with_context(|| format!("invalid TOML in {}", path.display()))?;

    Ok(config)
}

pub fn ensure_config_dir() -> Result<PathBuf> {
    let dir = config_dir()?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create config directory {}", dir.display()))?;
    Ok(dir)
}

pub fn save_config(config: &Config) -> Result<()> {
    ensure_config_dir()?;
    let path = config_path()?;
    let content =
        toml::to_string_pretty(config).context("failed to serialize config to TOML")?;
    std::fs::write(&path, content)
        .with_context(|| format!("failed to write config to {}", path.display()))?;
    Ok(())
}
