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

pub async fn init_providers(
    config: &config::model::Config,
) -> Vec<(Arc<dyn PrProvider>, UserId, String)> {
    let init_futs: Vec<_> = config
        .providers
        .iter()
        .map(|provider_config| async {
            match provider_config {
                ProviderConfig::AzureDevOps(ado_config) => {
                    let provider = match ado_config.auth {
                        AuthMethod::Pat => {
                            let pat = config::credentials::get_token(&ado_config.name)
                                .map_err(|e| {
                                    eprintln!("skipping provider {}: {}", ado_config.name, e);
                                })
                                .ok()?;
                            AzureDevOpsProvider::new(ado_config, &pat)
                        }
                        AuthMethod::AzureCli => {
                            let credential = Arc::new(auth::AzureCliCredential);
                            AzureDevOpsProvider::with_credential(ado_config, credential)
                        }
                        AuthMethod::DeviceCode => {
                            // Device code needs an interactive flow first;
                            // if no cached token exists, skip with a message.
                            eprintln!(
                                "provider {}: device-code auth requires browser sign-in from settings",
                                ado_config.name
                            );
                            return None;
                        }
                    };
                    let user = provider
                        .get_current_user()
                        .await
                        .map_err(|e| {
                            eprintln!("skipping provider {}: {}", ado_config.name, e);
                        })
                        .ok()?;
                    let auth_label = provider.credential_type();
                    let name = format!("{} ({}, {})", ado_config.name, user.display_name, auth_label);
                    Some((Arc::new(provider) as Arc<dyn PrProvider>, user, name))
                }
            }
        })
        .collect();

    futures::future::join_all(init_futs)
        .await
        .into_iter()
        .flatten()
        .collect()
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
}

pub struct CachedBuildStatus {
    pub source_commit_id: String,
    pub status: BuildStatus,
}
