use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, State};

use crate::config::model::{Config, ProviderConfig};
use crate::config::{credentials, loader};
use crate::polling::error::PollResult;
use crate::polling::poller::Poller;
use crate::providers::azure_devops::AzureDevOpsProvider;
use crate::providers::traits::PrProvider;
use crate::providers::types::{PrDetail, PrId};
use crate::state::AppState;

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    let config = state.config().read().await;
    serde_json::to_value(&*config)
        .and_then(serde_json::from_value)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_config(state: State<'_, AppState>, config: Config) -> Result<(), String> {
    loader::save_config(&config).map_err(|e| e.to_string())?;
    let mut current = state.config().write().await;
    *current = config;
    Ok(())
}

#[tauri::command]
pub async fn store_token(provider_name: String, token: String) -> Result<(), String> {
    credentials::store_token(&provider_name, &token).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_token(provider_name: String) -> Result<(), String> {
    credentials::delete_token(&provider_name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_connection(
    provider_name: String,
    url: String,
    token: String,
) -> Result<String, String> {
    let provider = AzureDevOpsProvider::new_from_params(&provider_name, &url, &token);
    let user = provider
        .get_current_user()
        .await
        .map_err(|e| e.to_string())?;
    Ok(user.display_name)
}

#[tauri::command]
pub async fn init_providers(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let config = state.config().read().await;
    let mut instances: Vec<(Arc<dyn PrProvider>, crate::providers::types::UserId)> = Vec::new();
    let mut names = Vec::new();

    for provider_config in &config.providers {
        match provider_config {
            ProviderConfig::AzureDevOps(ado_config) => {
                let pat = credentials::get_token(&ado_config.name).map_err(|e| e.to_string())?;
                let provider = AzureDevOpsProvider::new(ado_config, &pat);
                let user = provider
                    .get_current_user()
                    .await
                    .map_err(|e| e.to_string())?;
                names.push(format!("{} ({})", ado_config.name, user.display_name));
                instances.push((Arc::new(provider), user));
            }
        }
    }

    let mut providers = state.providers().write().await;
    *providers = instances.clone();

    let mut poller = state.poller().write().await;
    *poller = Some(Poller::new(instances));

    Ok(names)
}

#[tauri::command]
pub async fn poll_all(state: State<'_, AppState>) -> Result<PollResult, String> {
    let poller = state.poller().read().await;
    let poller = poller.as_ref().ok_or("providers not initialized")?;
    Ok(poller.poll_once().await)
}

#[tauri::command]
pub async fn get_pr_detail(state: State<'_, AppState>, pr_id: PrId) -> Result<PrDetail, String> {
    let providers = state.providers().read().await;
    let (provider, _) = providers
        .iter()
        .find(|(p, _)| p.name() == pr_id.provider)
        .ok_or_else(|| format!("provider '{}' not found", pr_id.provider))?;

    provider
        .get_detail(&pr_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_projects(
    provider_name: String,
    url: String,
    token: String,
) -> Result<Vec<String>, String> {
    let provider = AzureDevOpsProvider::new_from_params(&provider_name, &url, &token);
    provider
        .list_projects()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_polling(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let state = state.inner().clone();
    let handle = app.clone();

    tokio::spawn(async move {
        loop {
            let result = {
                let poller = state.poller().read().await;
                match poller.as_ref() {
                    Some(p) => p.poll_once().await,
                    None => {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                }
            };

            let changes = {
                let all_prs: Vec<_> = result
                    .reviewing
                    .iter()
                    .chain(result.authored.iter())
                    .cloned()
                    .collect();
                let mut tracker = state.change_tracker().write().await;
                tracker.detect_changes(&all_prs)
            };

            let _ = handle.emit("poll-update", &result);

            for change in &changes {
                let _ = handle.emit("pr-change", change);
            }

            let interval = {
                let config = state.config().read().await;
                config.general.refresh_interval_secs
            };
            tokio::time::sleep(Duration::from_secs(interval)).await;
        }
    });

    Ok(())
}
