use chrono::{DateTime, Utc};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId {
    pub provider: String,
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
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
    pub web_url: String,
}

#[derive(Debug, Clone)]
pub struct PrDetail {
    pub pr: PullRequest,
    pub diff_stats: Option<DiffStats>,
    pub build_status: Option<BuildStatus>,
    pub policies: Vec<PolicyStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub display_name: String,
    pub unique_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Reviewer {
    pub user: User,
    pub vote: Vote,
    pub is_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub fn initials_display(&self, user: &User) -> String {
        let initials = user
            .display_name
            .split_whitespace()
            .filter_map(|w| w.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase();
        format!("{}{}", self.symbol(), initials)
    }
}

#[derive(Debug, Clone)]
pub struct Repository {
    pub name: String,
    pub project: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeStatus {
    Succeeded,
    Conflicts,
    RejectedByPolicy,
    NotSet,
    Queued,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildStatus {
    Succeeded,
    Failed,
    InProgress,
    NotStarted,
}

#[derive(Debug, Clone)]
pub struct PolicyStatus {
    pub name: String,
    pub is_blocking: bool,
    pub status: PolicyEvaluation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyEvaluation {
    Approved,
    Rejected,
    Running,
    Queued,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
