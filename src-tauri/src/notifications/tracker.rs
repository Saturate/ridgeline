use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::providers::types::{PrId, PullRequest, Vote};

pub struct ChangeTracker {
    known_prs: HashSet<String>,
    vote_states: HashMap<String, Vec<(String, Vote)>>,
}

impl ChangeTracker {
    pub fn new() -> Self {
        Self {
            known_prs: HashSet::new(),
            vote_states: HashMap::new(),
        }
    }

    pub fn detect_changes(&mut self, prs: &[PullRequest]) -> Vec<Change> {
        let mut changes = Vec::new();

        let current_keys: HashSet<String> = prs.iter().map(|pr| pr_key(&pr.id)).collect();

        for pr in prs {
            let key = pr_key(&pr.id);
            if !self.known_prs.contains(&key) {
                changes.push(Change::NewPr {
                    title: pr.title.clone(),
                    author: pr.author.display_name.clone(),
                    repo: format!("{}/{}", pr.repository.project, pr.repository.name),
                });
            }

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
                            });
                        }
                        None if *vote != Vote::NoVote => {
                            changes.push(Change::VoteChanged {
                                pr_title: pr.title.clone(),
                                reviewer: name.clone(),
                                new_vote: vote.clone(),
                            });
                        }
                        _ => {}
                    }
                }
            }

            self.vote_states.insert(key, current_votes);
        }

        self.known_prs = current_keys;

        changes
    }
}

fn pr_key(id: &PrId) -> String {
    format!(
        "{}:{}:{}:{}",
        id.provider, id.project, id.repository, id.number
    )
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Change {
    #[serde(rename = "newPr")]
    NewPr {
        title: String,
        author: String,
        repo: String,
    },
    #[serde(rename = "voteChanged")]
    VoteChanged {
        pr_title: String,
        reviewer: String,
        new_vote: Vote,
    },
}

impl Change {
    pub fn notification_title(&self) -> String {
        match self {
            Change::NewPr { .. } => "New Pull Request".to_string(),
            Change::VoteChanged { .. } => "Vote Changed".to_string(),
        }
    }

    pub fn notification_body(&self) -> String {
        match self {
            Change::NewPr {
                title,
                author,
                repo,
            } => {
                format!("{author} opened \"{title}\" in {repo}")
            }
            Change::VoteChanged {
                pr_title,
                reviewer,
                new_vote,
            } => {
                format!(
                    "{reviewer} voted {} on \"{pr_title}\"",
                    new_vote.symbol()
                )
            }
        }
    }
}
