use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::Mutex;

use super::client::AzureDevOpsClient;
use super::mapper;
use super::types::AdoPullRequest;
use crate::auth::Credential;
use crate::config::model::{AzureDevOpsConfig, ProjectFilter};
use crate::providers::traits::{PrProvider, ProviderError};
use crate::providers::types::*;

const TEAM_QUERY_CONCURRENCY: usize = 8;
/// Joining a team, or adding Project and Team read to a token, shows up
/// within this long without a restart.
const TEAM_REFRESH: Duration = Duration::from_secs(15 * 60);

struct TeamCache {
    ids: Vec<String>,
    fetched: Instant,
}

pub struct AzureDevOpsProvider {
    name: String,
    client: AzureDevOpsClient,
    projects: Vec<ProjectFilter>,
    teams: Mutex<Option<TeamCache>>,
}

impl AzureDevOpsProvider {
    pub fn new(config: &AzureDevOpsConfig, pat: &str) -> Self {
        Self {
            name: config.name.clone(),
            client: AzureDevOpsClient::new(&config.url, pat),
            projects: config.projects.clone(),
            teams: Mutex::new(None),
        }
    }

    pub fn with_credential(config: &AzureDevOpsConfig, credential: Arc<dyn Credential>) -> Self {
        Self {
            name: config.name.clone(),
            client: AzureDevOpsClient::with_credential(&config.url, credential),
            projects: config.projects.clone(),
            teams: Mutex::new(None),
        }
    }

    pub fn new_from_params(name: &str, url: &str, pat: &str) -> Self {
        Self {
            name: name.to_string(),
            client: AzureDevOpsClient::new(url, pat),
            projects: Vec::new(),
            teams: Mutex::new(None),
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
        use futures::{StreamExt, TryStreamExt};

        // Built up front: a closure in `.map()` trips the Send bound async_trait
        // puts on the returned future.
        let team_ids = self.team_ids().await?;
        let queries: Vec<_> = team_ids
            .iter()
            .map(|id| async move {
                match self.client.list_pull_requests_org(Some(id), None).await {
                    // Not seen in practice (the org query seems to just leave
                    // out repos you can't read), but if one team is refused,
                    // failing here would freeze Reviewing on every poll.
                    Err(ProviderError::Denied { message, .. }) => {
                        eprintln!("provider {}: skipped team {id}: {message}", self.name);
                        Ok(Vec::new())
                    }
                    result => result,
                }
            })
            .collect();
        let per_team: Vec<Vec<AdoPullRequest>> = futures::stream::iter(queries)
            // Some people sit in dozens of teams; with Azure CLI auth every
            // request also spawns `az`, so the fan-out is capped.
            .buffer_unordered(TEAM_QUERY_CONCURRENCY)
            .try_collect()
            .await?;
        Ok(per_team.into_iter().flatten().collect())
    }

    /// A PAT without the Project and Team (Read) scope gets 401/403 here,
    /// while the PR queries before it went through. With no list from
    /// earlier that's a setup choice, not an outage, so it counts as no teams
    /// until the next refresh. Any other failure falls back to the teams from
    /// last time, or, on the first lookup, is returned so the poller keeps the
    /// previous list instead of reporting team PRs as gone.
    async fn team_ids(&self) -> Result<Vec<String>, ProviderError> {
        // Held across the fetch so concurrent polls share one lookup.
        let mut cache = self.teams.lock().await;
        if let Some(fresh) = cache
            .as_ref()
            .filter(|c| c.fetched.elapsed() < TEAM_REFRESH)
        {
            return Ok(fresh.ids.clone());
        }
        let ids = match self.client.list_my_teams().await {
            Ok(teams) => teams.into_iter().map(|t| t.id).collect(),
            // With a list from earlier, a 401/403 is more likely a blip than a
            // scope that was just removed, and emptying it would make every
            // team PR vanish and come back later as "new". Team PR queries
            // only need Code read, so the old list keeps working either way.
            Err(ProviderError::Denied { .. })
                if cache.as_ref().is_some_and(|c| !c.ids.is_empty()) =>
            {
                eprintln!(
                    "provider {}: team refresh was refused, using last list",
                    self.name
                );
                return Ok(cache.as_ref().map_or_else(Vec::new, |c| c.ids.clone()));
            }
            Err(ProviderError::Denied { message, .. }) => {
                eprintln!(
                    "provider {}: can't list your teams, so PRs assigned to them are skipped \
                     (the token needs Project and Team read access): {message}",
                    self.name
                );
                Vec::new()
            }
            Err(e) => {
                let Some(stale) = cache.as_ref() else {
                    return Err(e);
                };
                eprintln!(
                    "provider {}: team refresh failed, using last list: {e}",
                    self.name
                );
                return Ok(stale.ids.clone());
            }
        };
        *cache = Some(TeamCache {
            ids: ids.clone(),
            fetched: Instant::now(),
        });
        Ok(ids)
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
            for ado_pr in new_team_prs(&mut seen_ids, self.team_prs().await?, me) {
                all_prs.push(mapper::map_pull_request(
                    &ado_pr,
                    &self.name,
                    self.client.base_url(),
                ));
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

/// Team PRs not already listed. Azure's team view leaves out your own PRs,
/// even though your team is a reviewer on them.
fn new_team_prs(
    seen_ids: &mut std::collections::HashSet<u64>,
    team_prs: Vec<AdoPullRequest>,
    me: &str,
) -> Vec<AdoPullRequest> {
    team_prs
        .into_iter()
        .filter(|pr| pr.created_by.id != me && seen_ids.insert(pr.pull_request_id))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn pr(id: u64, author: &str) -> AdoPullRequest {
        serde_json::from_value(serde_json::json!({
            "pullRequestId": id,
            "title": "t",
            "status": "active",
            "creationDate": "2026-11-01T00:00:00Z",
            "sourceRefName": "refs/heads/a",
            "targetRefName": "refs/heads/master",
            "createdBy": { "id": author, "displayName": author },
            "repository": { "id": "r", "name": "r", "project": { "id": "p", "name": "p" } },
        }))
        .unwrap()
    }

    fn ids(prs: &[AdoPullRequest]) -> Vec<u64> {
        prs.iter().map(|p| p.pull_request_id).collect()
    }

    #[test]
    fn team_prs_skip_already_listed_and_own() {
        let mut seen = HashSet::from([1]);
        let team = vec![pr(1, "bob"), pr(2, "me"), pr(3, "bob")];
        assert_eq!(ids(&new_team_prs(&mut seen, team, "me")), vec![3]);
    }

    #[test]
    fn pr_on_several_teams_is_listed_once() {
        let mut seen = HashSet::new();
        let team = vec![pr(5, "bob"), pr(6, "bob"), pr(5, "bob")];
        assert_eq!(ids(&new_team_prs(&mut seen, team, "me")), vec![5, 6]);
    }

    #[test]
    fn no_teams_adds_nothing() {
        let mut seen = HashSet::from([1]);
        assert!(new_team_prs(&mut seen, Vec::new(), "me").is_empty());
        assert_eq!(seen, HashSet::from([1]));
    }
}
