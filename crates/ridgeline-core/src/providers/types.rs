use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrId {
    pub provider: String,
    pub project: String,
    pub repository: String,
    pub number: u64,
}

impl fmt::Display for PrId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}/{}/#{}",
            self.provider, self.project, self.repository, self.number
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserId {
    pub provider: String,
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub id: PrId,
    pub title: String,
    pub description: Option<String>,
    pub status: PrStatus,
    pub is_draft: bool,
    pub created_at: DateTime<Utc>,
    pub author: User,
    pub source_branch: String,
    pub target_branch: String,
    pub reviewers: Vec<Reviewer>,
    pub repository: Repository,
    pub labels: Vec<String>,
    pub merge_status: Option<MergeStatus>,
    pub build_status: Option<BuildStatus>,
    pub source_commit_id: Option<String>,
    pub web_url: String,
    /// Unresolved comment threads started by someone other than the author.
    /// Only filled in for authored PRs.
    #[serde(default)]
    pub open_threads: u32,
}

impl PullRequest {
    /// Whether `user_id` is a reviewer on this PR and has approved it.
    pub fn is_approved_by(&self, user_id: &str) -> bool {
        self.reviewers.iter().any(|r| {
            r.user.id == user_id && matches!(r.vote, Vote::Approved | Vote::ApprovedWithSuggestions)
        })
    }

    /// True when the author has something to fix: merge conflicts, a failing
    /// build, unresolved reviewer comments, or a reviewer pushed back.
    pub fn is_waiting_on_author(&self) -> bool {
        let pushed_back = self
            .reviewers
            .iter()
            .any(|r| matches!(r.vote, Vote::Rejected | Vote::WaitingForAuthor));

        pushed_back
            || self.open_threads > 0
            || self.merge_status == Some(MergeStatus::Conflicts)
            || self.build_status == Some(BuildStatus::Failed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrDetail {
    pub pr: PullRequest,
    pub diff_stats: Option<DiffStats>,
    pub build_status: Option<BuildStatus>,
    pub policies: Vec<PolicyStatus>,
    /// False when the policy evaluations couldn't be fetched, so an empty
    /// `policies` means "unknown" rather than "this repo has none".
    #[serde(default)]
    pub policies_loaded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrStatus {
    Active,
    Completed,
    Abandoned,
}

impl fmt::Display for PrStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrStatus::Active => write!(f, "Active"),
            PrStatus::Completed => write!(f, "Completed"),
            PrStatus::Abandoned => write!(f, "Abandoned"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub display_name: String,
    pub unique_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reviewer {
    pub user: User,
    pub vote: Vote,
    pub is_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vote {
    Approved,
    ApprovedWithSuggestions,
    NoVote,
    WaitingForAuthor,
    Rejected,
}

impl Vote {
    pub fn symbol(&self) -> &'static str {
        match self {
            Vote::Approved => "✓",
            Vote::ApprovedWithSuggestions => "✓~",
            Vote::NoVote => "·",
            Vote::WaitingForAuthor => "⏳",
            Vote::Rejected => "✗",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub project: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeStatus {
    Succeeded,
    Conflicts,
    RejectedByPolicy,
    NotSet,
    Queued,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffStats {
    pub files_changed: u32,
    pub additions: u32,
    pub deletions: u32,
}

impl fmt::Display for DiffStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "+{}/-{}", self.additions, self.deletions)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildStatus {
    Succeeded,
    Failed,
    InProgress,
    NotStarted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyStatus {
    pub name: String,
    pub is_blocking: bool,
    pub status: PolicyEvaluation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyEvaluation {
    Approved,
    Rejected,
    Running,
    Queued,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderType {
    AzureDevOps,
    GitHub,
    GitLab,
    Bitbucket,
}

impl fmt::Display for ProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderType::AzureDevOps => write!(f, "Azure DevOps"),
            ProviderType::GitHub => write!(f, "GitHub"),
            ProviderType::GitLab => write!(f, "GitLab"),
            ProviderType::Bitbucket => write!(f, "Bitbucket"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reviewer(vote: Vote) -> Reviewer {
        Reviewer {
            user: User {
                id: "r".into(),
                display_name: "Reviewer".into(),
                unique_name: None,
            },
            vote,
            is_required: false,
        }
    }

    fn pr(reviewers: Vec<Reviewer>) -> PullRequest {
        PullRequest {
            id: PrId {
                provider: "p".into(),
                project: "proj".into(),
                repository: "repo".into(),
                number: 1,
            },
            title: "t".into(),
            description: None,
            status: PrStatus::Active,
            is_draft: false,
            created_at: Utc::now(),
            author: User {
                id: "a".into(),
                display_name: "Author".into(),
                unique_name: None,
            },
            source_branch: "refs/heads/feature".into(),
            target_branch: "refs/heads/main".into(),
            reviewers,
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

    #[test]
    fn pending_or_approved_waits_on_reviewers() {
        assert!(!pr(vec![reviewer(Vote::Approved), reviewer(Vote::NoVote)]).is_waiting_on_author());
        assert!(!pr(vec![reviewer(Vote::Approved)]).is_waiting_on_author());
        assert!(!pr(Vec::new()).is_waiting_on_author());
    }

    fn voted(id: &str, vote: Vote) -> Reviewer {
        let mut r = reviewer(vote);
        r.user.id = id.into();
        r
    }

    #[test]
    fn approved_by_matches_only_your_own_approval() {
        let p = pr(vec![
            voted("me", Vote::ApprovedWithSuggestions),
            voted("other", Vote::NoVote),
        ]);
        assert!(p.is_approved_by("me"));
        assert!(!p.is_approved_by("other"));
        assert!(!p.is_approved_by("nobody"));

        let waiting = pr(vec![voted("me", Vote::WaitingForAuthor)]);
        assert!(!waiting.is_approved_by("me"));
    }

    #[test]
    fn pushback_waits_on_author() {
        assert!(pr(vec![reviewer(Vote::WaitingForAuthor)]).is_waiting_on_author());
        assert!(pr(vec![reviewer(Vote::Rejected), reviewer(Vote::NoVote)]).is_waiting_on_author());
    }

    #[test]
    fn conflicts_failed_builds_and_comments_wait_on_author() {
        let mut conflicts = pr(vec![reviewer(Vote::NoVote)]);
        conflicts.merge_status = Some(MergeStatus::Conflicts);
        assert!(conflicts.is_waiting_on_author());

        let mut failed = pr(vec![reviewer(Vote::NoVote)]);
        failed.build_status = Some(BuildStatus::Failed);
        assert!(failed.is_waiting_on_author());

        let mut commented = pr(vec![reviewer(Vote::NoVote)]);
        commented.open_threads = 2;
        assert!(commented.is_waiting_on_author());
    }
}
