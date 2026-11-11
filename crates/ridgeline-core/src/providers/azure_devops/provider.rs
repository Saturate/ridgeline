use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::OnceCell;

use super::client::AzureDevOpsClient;
use super::mapper;
use super::types::AdoPullRequest;
use crate::auth::Credential;
use crate::config::model::{AzureDevOpsConfig, ProjectFilter};
use crate::providers::traits::{PrProvider, ProviderError};
use crate::providers::types::*;

pub struct AzureDevOpsProvider {
    name: String,
    client: AzureDevOpsClient,
    projects: Vec<ProjectFilter>,
    /// Team membership rarely changes, so it's looked up once per run.
    team_ids: OnceCell<Vec<String>>,
}

impl AzureDevOpsProvider {
    pub fn new(config: &AzureDevOpsConfig, pat: &str) -> Self {
        Self {
            name: config.name.clone(),
            client: AzureDevOpsClient::new(&config.url, pat),
            projects: config.projects.clone(),
            team_ids: OnceCell::new(),
        }
    }

    pub fn with_credential(config: &AzureDevOpsConfig, credential: Arc<dyn Credential>) -> Self {
        Self {
            name: config.name.clone(),
            client: AzureDevOpsClient::with_credential(&config.url, credential),
            projects: config.projects.clone(),
            team_ids: OnceCell::new(),
        }
    }

    pub fn new_from_params(name: &str, url: &str, pat: &str) -> Self {
        Self {
            name: name.to_string(),
            client: AzureDevOpsClient::new(url, pat),
            projects: Vec::new(),
            team_ids: OnceCell::new(),
        }
    }

    pub fn credential_type(&self) -> &str {
        self.client.credential().display_name()
    }

    pub async fn list_projects(&self) -> Result<Vec<String>, ProviderError> {
        self.client.list_projects().await
    }

    /// Check configured projects against the org's actual project list.
    /// Returns names of projects that don't exist.
    pub async fn validate_projects(&self) -> Result<Vec<String>, ProviderError> {
        let available = self.client.list_projects().await?;
        let invalid: Vec<String> = self
            .projects
            .iter()
            .filter(|p| !available.iter().any(|a| a.eq_ignore_ascii_case(&p.name)))
            .map(|p| p.name.clone())
            .collect();
        Ok(invalid)
    }

    /// Active PRs where any of the user's teams is a reviewer.
    async fn team_prs(&self) -> Result<Vec<AdoPullRequest>, ProviderError> {
        let team_ids = self.team_ids().await;
        let results = futures::future::join_all(
            team_ids
                .iter()
                .map(|id| self.client.list_pull_requests_org(Some(id), None)),
        )
        .await;
        let mut prs = Vec::new();
        for result in results {
            prs.extend(result?);
        }
        Ok(prs)
    }

    /// A PAT without the project/team read scope can't list teams. That
    /// shouldn't take down the PRs you're on directly, so it degrades to no
    /// teams and is retried on the next refresh.
    async fn team_ids(&self) -> &[String] {
        let result = self
            .team_ids
            .get_or_try_init(|| async {
                let teams = self.client.list_my_teams().await?;
                Ok::<_, ProviderError>(teams.into_iter().map(|t| t.id).collect())
            })
            .await;
        match result {
            Ok(ids) => ids,
            Err(e) => {
                eprintln!("provider {}: could not list your teams: {e}", self.name);
                &[]
            }
        }
    }

    /// Fetch PRs org-wide for the user, plus any extras from configured projects.
    async fn fetch_prs(
        &self,
        reviewer_id: Option<&str>,
        creator_id: Option<&str>,
    ) -> Result<Vec<PullRequest>, ProviderError> {
        use std::collections::HashSet;

        // Primary: org-level query gets all PRs where user is reviewer/creator
        let org_prs = self
            .client
            .list_pull_requests_org(reviewer_id, creator_id)
            .await?;

        let mut seen_ids = HashSet::new();
        let mut all_prs = Vec::new();

        for ado_pr in &org_prs {
            seen_ids.insert(ado_pr.pull_request_id);
            all_prs.push(mapper::map_pull_request(
                ado_pr,
                &self.name,
                self.client.base_url(),
            ));
        }

        // Azure lists a PR under "Assigned to my teams" when one of your teams
        // is a reviewer, but `reviewerId=<you>` only matches you by name.
        if let Some(me) = reviewer_id {
            for ado_pr in self.team_prs().await? {
                if ado_pr.created_by.id != me && seen_ids.insert(ado_pr.pull_request_id) {
                    all_prs.push(mapper::map_pull_request(
                        &ado_pr,
                        &self.name,
                        self.client.base_url(),
                    ));
                }
            }
        }

        // Extra: configured projects add PRs the user isn't directly on.
        // Only for reviewer queries (team monitoring), not authored queries.
        if reviewer_id.is_some() {
            for project in &self.projects {
                let ado_prs = if project.repos.is_empty() {
                    self.client
                        .list_pull_requests_by_project(&project.name, None, None)
                        .await?
                } else {
                    let mut prs = Vec::new();
                    for repo in &project.repos {
                        let repo_prs = self
                            .client
                            .list_pull_requests_by_repo(&project.name, repo, None, None)
                            .await?;
                        prs.extend(repo_prs);
                    }
                    prs
                };

                for ado_pr in &ado_prs {
                    if seen_ids.insert(ado_pr.pull_request_id) {
                        all_prs.push(mapper::map_pull_request(
                            ado_pr,
                            &self.name,
                            self.client.base_url(),
                        ));
                    }
                }
            }
        }

        Ok(all_prs)
    }
}

#[async_trait]
impl PrProvider for AzureDevOpsProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::AzureDevOps
    }

    async fn get_current_user(&self) -> Result<UserId, ProviderError> {
        let conn = self.client.get_connection_data().await?;
        let display_name = conn
            .authenticated_user
            .properties
            .as_ref()
            .and_then(|p| p.account.as_ref())
            .and_then(|a| a.value.clone())
            .unwrap_or_else(|| conn.authenticated_user.id.clone());

        Ok(UserId {
            provider: self.name.clone(),
            id: conn.authenticated_user.id,
            display_name,
        })
    }

    async fn list_reviewing(&self, user: &UserId) -> Result<Vec<PullRequest>, ProviderError> {
        self.fetch_prs(Some(&user.id), None).await
    }

    async fn list_authored(&self, user: &UserId) -> Result<Vec<PullRequest>, ProviderError> {
        self.fetch_prs(None, Some(&user.id)).await
    }

    async fn get_detail(&self, pr_id: &PrId) -> Result<PrDetail, ProviderError> {
        let ado_pr = self
            .client
            .get_pull_request(&pr_id.project, &pr_id.repository, pr_id.number)
            .await?;

        let pr = mapper::map_pull_request(&ado_pr, &self.name, self.client.base_url());

        // Fetch policy evaluations
        let (policies, policies_loaded) = match self
            .client
            .get_policy_evaluations(&pr_id.project, pr_id.number)
            .await
        {
            Ok(evals) => (
                evals.iter().map(mapper::map_policy_evaluation).collect(),
                true,
            ),
            Err(_) => (Vec::new(), false),
        };

        Ok(PrDetail {
            pr,
            diff_stats: None,
            build_status: None,
            policies,
            policies_loaded,
        })
    }

    async fn count_open_threads(&self, pr: &PullRequest) -> Result<u32, ProviderError> {
        let threads = self
            .client
            .get_threads(&pr.id.project, &pr.id.repository, pr.id.number)
            .await?;
        Ok(threads
            .iter()
            .filter(|t| mapper::is_open_reviewer_thread(t, &pr.author.id))
            .count() as u32)
    }

    fn web_url(&self, pr_id: &PrId) -> String {
        format!(
            "{}/{}/{}/_git/{}/pullrequest/{}",
            self.client.base_url(),
            pr_id.project,
            pr_id.project,
            pr_id.repository,
            pr_id.number
        )
    }
}
