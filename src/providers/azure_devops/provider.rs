use async_trait::async_trait;

use super::client::AzureDevOpsClient;
use super::mapper;
use crate::config::model::{AzureDevOpsConfig, ProjectFilter};
use crate::providers::traits::{PrProvider, ProviderError};
use crate::providers::types::*;

pub struct AzureDevOpsProvider {
    name: String,
    client: AzureDevOpsClient,
    projects: Vec<ProjectFilter>,
}

impl AzureDevOpsProvider {
    pub fn new(config: &AzureDevOpsConfig, pat: &str) -> Self {
        Self {
            name: config.name.clone(),
            client: AzureDevOpsClient::new(&config.url, pat),
            projects: config.projects.clone(),
        }
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
        // Fetch the PR itself
        let ado_prs = self
            .client
            .list_pull_requests_by_repo(
                &pr_id.project,
                &pr_id.repository,
                None,
                None,
            )
            .await?;

        let ado_pr = ado_prs
            .iter()
            .find(|p| p.pull_request_id == pr_id.number)
            .ok_or_else(|| ProviderError::Other(format!("PR {} not found", pr_id.number)))?;

        let pr = mapper::map_pull_request(ado_pr, &self.name, self.client.base_url());

        // Fetch policy evaluations
        let policies = match self
            .client
            .get_policy_evaluations(&pr_id.project, pr_id.number)
            .await
        {
            Ok(evals) => evals.iter().map(mapper::map_policy_evaluation).collect(),
            Err(_) => Vec::new(),
        };

        Ok(PrDetail {
            pr,
            diff_stats: None,
            build_status: None,
            policies,
        })
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
