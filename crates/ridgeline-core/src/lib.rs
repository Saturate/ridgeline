pub mod auth;
pub mod config;
pub mod mcp;
pub mod notifications;
pub mod polling;
pub mod providers;
pub mod util;

use std::sync::Arc;

use futures::StreamExt as _;

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
                // Running builds finish, failed ones get re-queued, and
                // NotStarted is also what a failed policy fetch or a fresh
                // push looks like, all without a new commit. Only a pass is
                // final enough to reuse.
                if !commit_id.is_empty() && build_status == BuildStatus::Succeeded {
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
/// every poll rather than cached. When a fetch fails the previous poll's count
/// is kept, so a 429 or timeout doesn't move a PR out of "Your turn".
pub async fn enrich_open_threads(
    prs: &mut [PullRequest],
    providers: &[(Arc<dyn PrProvider>, UserId)],
    previous: &std::collections::HashMap<providers::types::PrId, u32>,
) -> Vec<polling::error::PollError> {
    // Collected up front: a stream over a borrowing iterator trips rustc's
    // higher-ranked Send check once this future is spawned.
    let futs: Vec<_> = prs
        .iter()
        .map(|pr| {
            let provider = providers
                .iter()
                .find(|(p, _)| p.name() == pr.id.provider)
                .map(|(p, _)| p.clone());
            let pr = pr.clone();
            async move {
                match provider {
                    Some(provider) => provider.count_open_threads(&pr).await,
                    None => Ok(0),
                }
            }
        })
        .collect();
    // Capped so a long authored list doesn't fire every request at once.
    let counts = futures::stream::iter(futs)
        .buffered(8)
        .collect::<Vec<_>>()
        .await;

    let mut errors = Vec::new();
    for (pr, count) in prs.iter_mut().zip(counts) {
        match count {
            Ok(count) => pr.open_threads = count,
            Err(e) => {
                pr.open_threads = previous.get(&pr.id).copied().unwrap_or(0);
                errors.push(polling::error::PollError {
                    provider: pr.id.provider.clone(),
                    kind: polling::error::PollErrorKind::Unknown,
                    message: format!("comments for #{}: {e}", pr.id.number),
                });
            }
        }
    }
    errors
}

#[derive(Clone)]
pub struct CachedBuildStatus {
    pub source_commit_id: String,
    pub status: BuildStatus,
}
