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
        anyhow::bail!(
            "config file not found at {}\n\
             Create it with your provider settings. See --help for details.",
            path.display()
        );
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read config from {}", path.display()))?;

    let config: Config =
        toml::from_str(&content).with_context(|| format!("invalid TOML in {}", path.display()))?;

    Ok(config)
}

/// Ensure the config directory exists (used by `auth add`)
pub fn ensure_config_dir() -> Result<PathBuf> {
    let dir = config_dir()?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create config directory {}", dir.display()))?;
    Ok(dir)
}

/// Write a new config file for the given Azure DevOps provider and projects
pub fn save_config(org_url: &str, provider_name: &str, projects: &[String]) -> Result<()> {
    ensure_config_dir()?;
    let path = config_path()?;

    let mut content = String::new();
    content.push_str("[general]\n");
    content.push_str("refresh_interval_secs = 60\n");
    content.push_str("stale_threshold_hours = 48\n");
    content.push_str("notifications_enabled = true\n\n");

    content.push_str("[[providers]]\n");
    content.push_str("type = \"azure-devops\"\n");
    content.push_str(&format!("name = \"{provider_name}\"\n"));
    content.push_str(&format!("url = \"{org_url}\"\n\n"));

    for project in projects {
        content.push_str("  [[providers.projects]]\n");
        content.push_str(&format!("  name = \"{project}\"\n"));
        content.push_str("  repos = []\n\n");
    }

    std::fs::write(&path, &content)
        .with_context(|| format!("failed to write config to {}", path.display()))?;

    Ok(())
}
