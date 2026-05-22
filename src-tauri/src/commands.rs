use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;

use crate::config::model::{Config, NotificationConfig, ProviderConfig};
use crate::config::{credentials, loader};
use crate::notifications::tracker::Change;
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
pub fn get_version(app: AppHandle) -> String {
    app.config().version.clone().unwrap_or_default()
}

#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_polling(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    // Cancel existing poll task if any
    {
        let mut task = state.poll_task().write().await;
        if let Some(handle) = task.take() {
            handle.abort();
        }
    }

    let state = state.inner().clone();
    let handle = app.clone();

    let task = tokio::spawn(async move {
        let mut first_poll = true;

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

            {
                let all_prs: Vec<_> = result
                    .reviewing
                    .iter()
                    .chain(result.authored.iter())
                    .cloned()
                    .collect();
                let mut tracker = state.change_tracker().write().await;
                let changes = tracker.detect_changes(&all_prs);

                if !first_poll {
                    let notif_config = {
                        let config = state.config().read().await;
                        (config.general.notifications_enabled, config.general.notifications.clone())
                    };

                    for change in &changes {
                        let _ = handle.emit("pr-change", change);

                        if notif_config.0 && should_notify(change, &notif_config.1) {
                            crate::notifications::sender::send_with_url(
                                &change.notification_title(),
                                &change.notification_body(),
                                change.web_url(),
                            );
                        }
                    }
                }
            }

            let result = enrich_authored_build_status(result, &state).await;

            let _ = handle.emit("poll-update", &result);
            first_poll = false;

            let interval = {
                let config = state.config().read().await;
                config.general.refresh_interval_secs
            };
            tokio::time::sleep(Duration::from_secs(interval)).await;
        }
    });

    // Store task handle so we can cancel it later
    let app_state = app.state::<AppState>();
    let mut poll_task = app_state.poll_task().write().await;
    *poll_task = Some(task);

    Ok(())
}

async fn enrich_authored_build_status(
    mut result: PollResult,
    state: &AppState,
) -> PollResult {
    use crate::providers::types::BuildStatus;
    use crate::state::CachedBuildStatus;

    let providers = state.providers().read().await;
    let mut cache = state.build_cache().write().await;

    let all_prs = result
        .authored
        .iter_mut()
        .chain(result.reviewing.iter_mut());

    for pr in all_prs {
        let pr_key = format!("{}:{}", pr.id, pr.source_commit_id.as_deref().unwrap_or(""));
        let commit_id = pr.source_commit_id.clone().unwrap_or_default();

        // Check cache — skip fetch if source commit hasn't changed
        if let Some(cached) = cache.get(&pr.id.to_string()) {
            if cached.source_commit_id == commit_id {
                pr.build_status = Some(cached.status.clone());
                continue;
            }
        }

        // Fetch policy evaluations for this PR
        let provider = providers
            .iter()
            .find(|(p, _)| p.name() == pr.id.provider);

        if let Some((provider, _)) = provider {
            if let Ok(detail) = provider.get_detail(&pr.id).await {
                let build_status = derive_build_status(&detail.policies);
                if !commit_id.is_empty() {
                    cache.insert(
                        pr.id.to_string(),
                        CachedBuildStatus {
                            source_commit_id: commit_id,
                            status: build_status.clone(),
                        },
                    );
                }
                pr.build_status = Some(build_status);
            }
        }
    }

    result
}

fn derive_build_status(
    policies: &[crate::providers::types::PolicyStatus],
) -> crate::providers::types::BuildStatus {
    use crate::providers::types::{BuildStatus, PolicyEvaluation};

    let build_policies: Vec<_> = policies
        .iter()
        .filter(|p| {
            let name = p.name.to_lowercase();
            name.contains("build") || name.contains("pipeline") || name.contains("ci")
        })
        .collect();

    if build_policies.is_empty() {
        return BuildStatus::NotStarted;
    }

    if build_policies.iter().any(|p| p.status == PolicyEvaluation::Rejected) {
        return BuildStatus::Failed;
    }
    if build_policies
        .iter()
        .any(|p| p.status == PolicyEvaluation::Running || p.status == PolicyEvaluation::Queued)
    {
        return BuildStatus::InProgress;
    }
    BuildStatus::Succeeded
}

fn should_notify(change: &Change, config: &NotificationConfig) -> bool {
    match change {
        Change::NewPr { .. } => config.new_pr,
        Change::VoteChanged { new_vote, .. } => {
            if *new_vote == crate::providers::types::Vote::WaitingForAuthor {
                config.waiting_for_author
            } else {
                config.vote_changed
            }
        }
        Change::Completed { .. } => config.completed,
    }
}

#[tauri::command]
pub async fn test_notification() -> Result<(), String> {
    crate::notifications::sender::send_with_url(
        "Ridgeline",
        "Notifications are working! Click to open the project.",
        "https://github.com/Saturate/ridgeline",
    );
    Ok(())
}
