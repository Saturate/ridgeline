use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::providers::types::{PrId, PrStatus, PullRequest, Vote};

#[derive(Clone, Serialize, Deserialize)]
struct PrMeta {
    title: String,
    repo: String,
    web_url: String,
    #[serde(default)]
    pr_id: Option<PrId>,
}

#[derive(Serialize, Deserialize, Default)]
struct TrackerState {
    known_prs: HashSet<String>,
    #[serde(default)]
    pr_meta: HashMap<String, PrMeta>,
    vote_states: HashMap<String, Vec<(String, String)>>,
}

pub struct ChangeTracker {
    known_prs: HashSet<String>,
    pr_meta: HashMap<String, PrMeta>,
    vote_states: HashMap<String, Vec<(String, Vote)>>,
    state_path: Option<PathBuf>,
}

impl ChangeTracker {
    pub fn new() -> Self {
        let state_path =
            dirs::config_dir().map(|d| d.join("ridgeline").join(".tracker_state.json"));

        let mut tracker = Self {
            known_prs: HashSet::new(),
            pr_meta: HashMap::new(),
            vote_states: HashMap::new(),
            state_path: state_path.clone(),
        };

        if let Some(path) = &state_path {
            if let Ok(data) = fs::read_to_string(path) {
                if let Ok(state) = serde_json::from_str::<TrackerState>(&data) {
                    tracker.known_prs = state.known_prs;
                    tracker.pr_meta = state.pr_meta;
                    tracker.vote_states = state
                        .vote_states
                        .into_iter()
                        .map(|(k, v)| {
                            (
                                k,
                                v.into_iter()
                                    .map(|(name, vote_str)| (name, deserialize_vote(&vote_str)))
                                    .collect(),
                            )
                        })
                        .collect();
                }
            }
        }

        tracker
    }

    pub fn detect_changes(&mut self, prs: &[PullRequest]) -> Vec<Change> {
        self.detect_changes_partial(prs, &HashSet::new())
    }

    /// Like `detect_changes`, but PRs from `unreachable` providers that are
    /// missing from `prs` are kept as known instead of reported gone. A failed
    /// query says nothing about whether those PRs still exist, and forgetting
    /// them would re-announce each one as new once the provider answers again.
    pub fn detect_changes_partial(
        &mut self,
        prs: &[PullRequest],
        unreachable: &HashSet<String>,
    ) -> Vec<Change> {
        let mut changes = Vec::new();

        let mut current_keys: HashSet<String> = prs.iter().map(|pr| pr_key(&pr.id)).collect();
        let held: Vec<String> = self
            .known_prs
            .iter()
            .filter(|key| !current_keys.contains(*key))
            .filter(|key| {
                self.pr_meta
                    .get(*key)
                    .and_then(|m| m.pr_id.as_ref())
                    .is_some_and(|id| unreachable.contains(&id.provider))
            })
            .cloned()
            .collect();
        current_keys.extend(held);

        // Detect disappeared PRs (were known, now gone)
        for key in &self.known_prs {
            if !current_keys.contains(key) {
                if let Some(meta) = self.pr_meta.remove(key) {
                    if let Some(pr_id) = meta.pr_id.clone() {
                        changes.push(Change::Disappeared {
                            title: meta.title.clone(),
                            repo: meta.repo.clone(),
                            web_url: meta.web_url.clone(),
                            pr_id,
                        });
                    } else {
                        changes.push(Change::Completed {
                            title: meta.title.clone(),
                            repo: meta.repo.clone(),
                            web_url: meta.web_url.clone(),
                        });
                    }
                }
                self.vote_states.remove(key);
            }
        }

        for pr in prs {
            let key = pr_key(&pr.id);
            if !self.known_prs.contains(&key) {
                changes.push(Change::NewPr {
                    title: pr.title.clone(),
                    author: pr.author.display_name.clone(),
                    repo: format!("{}/{}", pr.repository.project, pr.repository.name),
                    web_url: pr.web_url.clone(),
                });
            }

            self.pr_meta.insert(
                key.clone(),
                PrMeta {
                    title: pr.title.clone(),
                    repo: format!("{}/{}", pr.repository.project, pr.repository.name),
                    web_url: pr.web_url.clone(),
                    pr_id: Some(pr.id.clone()),
                },
            );

            let current_votes: Vec<(String, Vote)> = pr
                .reviewers
                .iter()
                .map(|r| (r.user.display_name.clone(), r.vote.clone()))
                .collect();

            if let Some(prev_votes) = self.vote_states.get(&key) {
                for (name, vote) in &current_votes {
                    let prev = prev_votes.iter().find(|(n, _)| n == name);
                    match prev {
                        Some((_, prev_vote)) if prev_vote != vote => {
                            changes.push(Change::VoteChanged {
                                pr_title: pr.title.clone(),
                                reviewer: name.clone(),
                                new_vote: vote.clone(),
                                web_url: pr.web_url.clone(),
                            });
                        }
                        None if *vote != Vote::NoVote => {
                            changes.push(Change::VoteChanged {
                                pr_title: pr.title.clone(),
                                reviewer: name.clone(),
                                new_vote: vote.clone(),
                                web_url: pr.web_url.clone(),
                            });
                        }
                        _ => {}
                    }
                }
            }

            self.vote_states.insert(key, current_votes);
        }

        self.known_prs = current_keys;
        self.save();

        changes
    }

    fn save(&self) {
        let Some(path) = &self.state_path else { return };
        let state = TrackerState {
            known_prs: self.known_prs.clone(),
            pr_meta: self.pr_meta.clone(),
            vote_states: self
                .vote_states
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        v.iter()
                            .map(|(name, vote)| (name.clone(), serialize_vote(vote)))
                            .collect(),
                    )
                })
                .collect(),
        };
        if let Ok(data) = serde_json::to_string(&state) {
            let _ = fs::write(path, data);
        }
    }
}

fn pr_key(id: &PrId) -> String {
    format!(
        "{}:{}:{}:{}",
        id.provider, id.project, id.repository, id.number
    )
}

fn serialize_vote(vote: &Vote) -> String {
    match vote {
        Vote::Approved => "approved",
        Vote::ApprovedWithSuggestions => "approved_with_suggestions",
        Vote::NoVote => "no_vote",
        Vote::WaitingForAuthor => "waiting_for_author",
        Vote::Rejected => "rejected",
    }
    .to_string()
}

fn deserialize_vote(s: &str) -> Vote {
    match s {
        "approved" => Vote::Approved,
        "approved_with_suggestions" => Vote::ApprovedWithSuggestions,
        "waiting_for_author" => Vote::WaitingForAuthor,
        "rejected" => Vote::Rejected,
        _ => Vote::NoVote,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Change {
    #[serde(rename = "newPr")]
    NewPr {
        title: String,
        author: String,
        repo: String,
        web_url: String,
    },
    #[serde(rename = "voteChanged")]
    VoteChanged {
        pr_title: String,
        reviewer: String,
        new_vote: Vote,
        web_url: String,
    },
    #[serde(rename = "completed")]
    Completed {
        title: String,
        repo: String,
        web_url: String,
    },
    #[serde(rename = "abandoned")]
    Abandoned {
        title: String,
        repo: String,
        web_url: String,
    },
    #[serde(skip)]
    Disappeared {
        title: String,
        repo: String,
        web_url: String,
        pr_id: PrId,
    },
}

impl Change {
    pub fn pr_id(&self) -> Option<&PrId> {
        match self {
            Change::Disappeared { pr_id, .. } => Some(pr_id),
            _ => None,
        }
    }

    pub fn resolve_disappeared(self, status: PrStatus) -> Option<Self> {
        match self {
            Change::Disappeared {
                title,
                repo,
                web_url,
                ..
            } => match status {
                PrStatus::Abandoned => Some(Change::Abandoned {
                    title,
                    repo,
                    web_url,
                }),
                PrStatus::Completed => Some(Change::Completed {
                    title,
                    repo,
                    web_url,
                }),
                PrStatus::Active => None,
            },
            other => Some(other),
        }
    }

    pub fn web_url(&self) -> &str {
        match self {
            Change::NewPr { web_url, .. }
            | Change::VoteChanged { web_url, .. }
            | Change::Completed { web_url, .. }
            | Change::Abandoned { web_url, .. }
            | Change::Disappeared { web_url, .. } => web_url,
        }
    }

    pub fn notification_title(&self) -> String {
        match self {
            Change::NewPr { .. } => "New Pull Request".to_string(),
            Change::VoteChanged { .. } => "Vote Changed".to_string(),
            Change::Completed { .. } => "PR Completed".to_string(),
            Change::Abandoned { .. } => "PR Abandoned".to_string(),
            Change::Disappeared { .. } => "PR Closed".to_string(),
        }
    }

    pub fn notification_body(&self) -> String {
        match self {
            Change::NewPr {
                title,
                author,
                repo,
                ..
            } => {
                format!("{author} opened \"{title}\" in {repo}")
            }
            Change::VoteChanged {
                pr_title,
                reviewer,
                new_vote,
                ..
            } => {
                format!("{reviewer} voted {} on \"{pr_title}\"", new_vote.symbol())
            }
            Change::Completed { title, repo, .. } => {
                format!("\"{title}\" in {repo} was completed")
            }
            Change::Abandoned { title, repo, .. } => {
                format!("\"{title}\" in {repo} was abandoned")
            }
            Change::Disappeared { title, repo, .. } => {
                format!("\"{title}\" in {repo} was closed")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::types::{Repository, User};

    fn tracker() -> ChangeTracker {
        // No state_path: tests must not touch the user's tracker file.
        ChangeTracker {
            known_prs: HashSet::new(),
            pr_meta: HashMap::new(),
            vote_states: HashMap::new(),
            state_path: None,
        }
    }

    fn pr(provider: &str, number: u64) -> PullRequest {
        PullRequest {
            id: PrId {
                provider: provider.into(),
                project: "proj".into(),
                repository: "repo".into(),
                number,
            },
            title: format!("PR {number}"),
            description: None,
            status: PrStatus::Active,
            is_draft: false,
            created_at: chrono::Utc::now(),
            author: User {
                id: "a".into(),
                display_name: "Author".into(),
                unique_name: None,
            },
            source_branch: "feature".into(),
            target_branch: "main".into(),
            reviewers: Vec::new(),
            repository: Repository {
                name: "repo".into(),
                project: "proj".into(),
            },
            labels: Vec::new(),
            merge_status: None,
            build_status: None,
            source_commit_id: None,
            web_url: String::new(),
            open_threads: 0,
        }
    }

    fn new_pr_count(changes: &[Change]) -> usize {
        changes
            .iter()
            .filter(|c| matches!(c, Change::NewPr { .. }))
            .count()
    }

    #[test]
    fn unreachable_provider_keeps_its_prs_known() {
        let mut t = tracker();
        t.detect_changes(&[pr("a", 1), pr("b", 2)]);

        // Provider "a" failed this poll: its PR is missing but must not be
        // forgotten or reported gone.
        let unreachable: HashSet<String> = ["a".to_string()].into();
        let changes = t.detect_changes_partial(&[pr("b", 2)], &unreachable);
        assert!(changes.is_empty());

        // When "a" answers again its PR is not announced as new.
        let changes = t.detect_changes(&[pr("a", 1), pr("b", 2)]);
        assert_eq!(new_pr_count(&changes), 0);
    }

    #[test]
    fn reachable_provider_still_reports_disappeared_prs() {
        let mut t = tracker();
        t.detect_changes(&[pr("a", 1), pr("b", 2)]);
        let changes = t.detect_changes_partial(&[pr("b", 2)], &HashSet::new());
        assert!(
            changes
                .iter()
                .any(|c| matches!(c, Change::Disappeared { .. }))
        );
    }
}
