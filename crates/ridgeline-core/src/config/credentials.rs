use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

/// Token file path: ~/.config/ridgeline/.token_<provider>
fn token_file_path(provider_name: &str) -> Result<PathBuf> {
    let base = dirs::config_dir().context("could not determine config directory")?;
    Ok(base.join("ridgeline").join(format!(".token_{provider_name}")))
}

pub fn store_token(provider_name: &str, token: &str) -> Result<()> {
    let path = token_file_path(provider_name)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, token)
        .with_context(|| format!("failed to write token to {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

pub fn get_token(provider_name: &str) -> Result<String> {
    let path = token_file_path(provider_name)?;
    if path.exists() {
        let token = fs::read_to_string(&path)
            .with_context(|| format!("failed to read token from {}", path.display()))?;
        return Ok(token.trim().to_string());
    }

    anyhow::bail!(
        "no token found for provider '{}'. Run: ridgeline auth add {}",
        provider_name,
        provider_name
    )
}

pub fn delete_token(provider_name: &str) -> Result<()> {
    let path = token_file_path(provider_name)?;
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("failed to delete token file {}", path.display()))?;
    }
    Ok(())
}
