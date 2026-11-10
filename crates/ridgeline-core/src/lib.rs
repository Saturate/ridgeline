pub mod auth;
pub mod config;
pub mod mcp;
pub mod notifications;
pub mod polling;
pub mod providers;
pub mod util;

use std::sync::Arc;

use config::model::{AuthMethod, NotificationConfig, ProviderConfig};
use notifications::tracker::Change;
use providers::azure_devops::AzureDevOpsProvider;
use providers::traits::PrProvider;
use providers::types::{BuildStatus, PolicyEvaluation, PolicyStatus, PullRequest, UserId, Vote};

/// Outcome of connecting one configured provider, kept per provider so the UI
/// can say which one failed and why.
pub struct ProviderInit {
    pub name: String,
    pub result: Result<(Arc<dyn PrProvider>, UserId), String>,
}

pub async fn init_providers(config: &config::model::Config) -> Vec<ProviderInit> {
    let init_futs: Vec<_> = config
        .providers
        .iter()
        .map(|provider_config| async {
            let ProviderConfig::AzureDevOps(ado_config) = provider_config;
            let result = connect_azure_devops(ado_config).await;
            if let Err(e) = &result {
                eprintln!("provider {}: {e}", ado_config.name);
            }
            ProviderInit {
                name: ado_config.name.clone(),
                result,
            }
        })
        .collect();

    futures::future::join_all(init_futs).await
}

async fn connect_azure_devops(
    ado_config: &config::model::AzureDevOpsConfig,
) -> Result<(Arc<dyn PrProvider>, UserId), String> {
    let provider = match ado_config.auth {
        AuthMethod::Pat => {
            let pat = config::credentials::get_token(&ado_config.name)
                .map_err(|_| "No token stored".to_string())?;
            AzureDevOpsProvider::new(ado_config, &pat)
        }
        AuthMethod::AzureCli => {
            AzureDevOpsProvider::with_credential(ado_config, Arc::new(auth::AzureCliCredential))
        }
        AuthMethod::DeviceCode => {
            return Err("Browser sign-in isn't supported yet".into());
        }
    };
    let user = provider
        .get_current_user()
        .await
        .map_err(|e| e.to_string())?;
    Ok((Arc::new(provider) as Arc<dyn PrProvider>, user))
}

pub fn derive_build_status(policies: &[PolicyStatus]) -> BuildStatus {
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

    if build_policies
        .iter()
        .any(|p| p.status == PolicyEvaluation::Rejected)
    {
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

pub fn should_notify(change: &Change, config: &NotificationConfig) -> bool {
    match change {
        Change::NewPr { .. } => config.new_pr,
        Change::VoteChanged { new_vote, .. } => {
            if *new_vote == Vote::WaitingForAuthor {
                config.waiting_for_author
            } else {
                config.vote_changed
            }
        }
        Change::Completed { .. } | Change::Abandoned { .. } => config.completed,
        Change::Disappeared { .. } => false,
    }
}

pub async fn enrich_build_status(
    prs: &mut [PullRequest],
    providers: &[(Arc<dyn PrProvider>, UserId)],
    cache: &mut std::collections::HashMap<String, CachedBuildStatus>,
) {
    for pr in prs.iter_mut() {
        let commit_id = pr.source_commit_id.clone().unwrap_or_default();

        if let Some(cached) = cache.get(&pr.id.to_string()) {
            if cached.source_commit_id == commit_id {
                pr.build_status = Some(cached.status.clone());
                continue;
            }
        }

        let provider = providers.iter().find(|(p, _)| p.name() == pr.id.provider);

        if let Some((provider, _)) = provider {
            if let Ok(detail) = provider.get_detail(&pr.id).await {
                let build_status = derive_build_status(&detail.policies);
                // A running build finishes without a new commit, so only
                // settled results are safe to reuse.
                if !commit_id.is_empty() && build_status != BuildStatus::InProgress {
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
}

/// Threads get resolved and added without new commits, so this is fetched
/// every poll rather than cached.
pub async fn enrich_open_threads(
    prs: &mut [PullRequest],
    providers: &[(Arc<dyn PrProvider>, UserId)],
) {
    let counts = futures::future::join_all(prs.iter().map(|pr| async move {
        let (provider, _) = providers.iter().find(|(p, _)| p.name() == pr.id.provider)?;
        provider.count_open_threads(pr).await.ok()
    }))
    .await;

    for (pr, count) in prs.iter_mut().zip(counts) {
        if let Some(count) = count {
            pr.open_threads = count;
        }
    }
}

pub struct CachedBuildStatus {
    pub source_commit_id: String,
    pub status: BuildStatus,
}
