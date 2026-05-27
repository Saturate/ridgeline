use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::providers::types::{PrId, PullRequest, Vote};

#[derive(Clone, Serialize, Deserialize)]
struct PrMeta {
    title: String,
    repo: String,
    web_url: String,
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
        let state_path = dirs::config_dir().map(|d| d.join("ridgeline").join(".tracker_state.json"));

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
        let mut changes = Vec::new();

        let current_keys: HashSet<String> = prs.iter().map(|pr| pr_key(&pr.id)).collect();

        // Detect completed PRs (were known, now gone)
        for key in &self.known_prs {
            if !current_keys.contains(key) {
                if let Some(meta) = self.pr_meta.get(key) {
                    changes.push(Change::Completed {
                        title: meta.title.clone(),
                        repo: meta.repo.clone(),
                        web_url: meta.web_url.clone(),
                    });
                }
                self.vote_states.remove(key);
                self.pr_meta.remove(key);
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
}

impl Change {
    pub fn web_url(&self) -> &str {
        match self {
            Change::NewPr { web_url, .. }
            | Change::VoteChanged { web_url, .. }
            | Change::Completed { web_url, .. } => web_url,
        }
    }

    pub fn notification_title(&self) -> String {
        match self {
            Change::NewPr { .. } => "New Pull Request".to_string(),
            Change::VoteChanged { .. } => "Vote Changed".to_string(),
            Change::Completed { .. } => "PR Completed".to_string(),
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
                format!(
                    "{reviewer} voted {} on \"{pr_title}\"",
                    new_vote.symbol()
                )
            }
            Change::Completed { title, repo, .. } => {
                format!("\"{title}\" in {repo} was completed")
            }
        }
    }
}
