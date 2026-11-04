use super::client::AzureDevOpsClient;
use super::types::*;
use crate::providers::traits::ProviderError;

const API_VERSION: &str = "api-version=7.1-preview";

impl AzureDevOpsClient {
    /// Get the current user's identity via the connection data endpoint
    pub async fn get_connection_data(&self) -> Result<AdoConnectionData, ProviderError> {
        self.get(&format!("/_apis/connectionData?{API_VERSION}"))
            .await
    }

    /// List active pull requests across the entire org, filtered by reviewer or creator.
    /// No project scope needed — returns PRs from all projects.
    pub async fn list_pull_requests_org(
        &self,
        reviewer_id: Option<&str>,
        creator_id: Option<&str>,
    ) -> Result<Vec<AdoPullRequest>, ProviderError> {
        let mut query = format!(
            "/_apis/git/pullrequests?searchCriteria.status=active&$top=200&{API_VERSION}"
        );

        if let Some(rid) = reviewer_id {
            query.push_str(&format!("&searchCriteria.reviewerId={rid}"));
        }

        if let Some(cid) = creator_id {
            query.push_str(&format!("&searchCriteria.creatorId={cid}"));
        }

        let response: ListResponse<AdoPullRequest> = self.get(&query).await?;
        Ok(response.value)
    }

    /// List active pull requests for a project, optionally filtered by reviewer
    pub async fn list_pull_requests_by_project(
        &self,
        project: &str,
        reviewer_id: Option<&str>,
        creator_id: Option<&str>,
    ) -> Result<Vec<AdoPullRequest>, ProviderError> {
        let mut query = format!(
            "/{project}/_apis/git/pullrequests?searchCriteria.status=active&$top=100&{API_VERSION}"
        );

        if let Some(rid) = reviewer_id {
            query.push_str(&format!("&searchCriteria.reviewerId={rid}"));
        }

        if let Some(cid) = creator_id {
            query.push_str(&format!("&searchCriteria.creatorId={cid}"));
        }

        let response: ListResponse<AdoPullRequest> = self.get(&query).await?;
        Ok(response.value)
    }

    /// List active pull requests for a specific repo
    pub async fn list_pull_requests_by_repo(
        &self,
        project: &str,
        repo: &str,
        reviewer_id: Option<&str>,
        creator_id: Option<&str>,
    ) -> Result<Vec<AdoPullRequest>, ProviderError> {
        let mut query = format!(
            "/{project}/_apis/git/repositories/{repo}/pullrequests?searchCriteria.status=active&$top=100&{API_VERSION}"
        );

        if let Some(rid) = reviewer_id {
            query.push_str(&format!("&searchCriteria.reviewerId={rid}"));
        }

        if let Some(cid) = creator_id {
            query.push_str(&format!("&searchCriteria.creatorId={cid}"));
        }

        let response: ListResponse<AdoPullRequest> = self.get(&query).await?;
        Ok(response.value)
    }

    /// List all projects in the organization
    pub async fn list_projects(&self) -> Result<Vec<String>, ProviderError> {
        let path = format!("/_apis/projects?{API_VERSION}&$top=1000&stateFilter=wellFormed");
        let response: ListResponse<AdoProject> = self.get(&path).await?;
        let mut names: Vec<String> = response.value.into_iter().map(|p| p.name).collect();
        names.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        Ok(names)
    }

    /// Get a single pull request by ID (returns full description, not truncated)
    pub async fn get_pull_request(
        &self,
        project: &str,
        repo: &str,
        pr_id: u64,
    ) -> Result<AdoPullRequest, ProviderError> {
        let path = format!(
            "/{project}/_apis/git/repositories/{repo}/pullrequests/{pr_id}?{API_VERSION}"
        );
        self.get(&path).await
    }

    /// Get policy evaluations for a PR
    pub async fn get_policy_evaluations(
        &self,
        project: &str,
        pr_id: u64,
    ) -> Result<Vec<AdoPolicyEvaluation>, ProviderError> {
        let path = format!(
            "/{project}/_apis/policy/evaluations?artifactId=vstfs:///CodeReview/CodeReviewId/{project}/{pr_id}&{API_VERSION}"
        );
        let response: ListResponse<AdoPolicyEvaluation> = self.get(&path).await?;
        Ok(response.value)
    }

    /// Get threads (comments) for a PR
    pub async fn get_threads(
        &self,
        project: &str,
        repo_id: &str,
        pr_id: u64,
    ) -> Result<Vec<AdoThread>, ProviderError> {
        let path = format!(
            "/{project}/_apis/git/repositories/{repo_id}/pullrequests/{pr_id}/threads?{API_VERSION}"
        );
        let response: ListResponse<AdoThread> = self.get(&path).await?;
        Ok(response.value)
    }
}
