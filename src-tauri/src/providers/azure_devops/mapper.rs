use chrono::DateTime;

use super::types::{AdoPolicyEvaluation, AdoPullRequest, AdoReviewer};
use crate::providers::types::*;

pub fn map_pull_request(ado_pr: &AdoPullRequest, provider_name: &str, base_url: &str) -> PullRequest {
    let project = &ado_pr.repository.project.name;
    let repo = &ado_pr.repository.name;

    PullRequest {
        id: PrId {
            provider: provider_name.to_string(),
            project: project.clone(),
            repository: repo.clone(),
            number: ado_pr.pull_request_id,
        },
        title: ado_pr.title.clone(),
        description: ado_pr.description.clone(),
        status: map_status(&ado_pr.status),
        is_draft: ado_pr.is_draft.unwrap_or(false),
        created_at: DateTime::parse_from_rfc3339(&ado_pr.creation_date)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        author: User {
            id: ado_pr.created_by.id.clone(),
            display_name: ado_pr.created_by.display_name.clone(),
            unique_name: ado_pr.created_by.unique_name.clone(),
        },
        source_branch: strip_refs_prefix(&ado_pr.source_ref_name),
        target_branch: strip_refs_prefix(&ado_pr.target_ref_name),
        reviewers: ado_pr
            .reviewers
            .as_ref()
            .map(|rs| rs.iter().map(map_reviewer).collect())
            .unwrap_or_default(),
        repository: Repository {
            name: repo.clone(),
            project: project.clone(),
        },
        labels: ado_pr
            .labels
            .as_ref()
            .map(|ls| ls.iter().map(|l| l.name.clone()).collect())
            .unwrap_or_default(),
        merge_status: ado_pr.merge_status.as_deref().map(map_merge_status),
        build_status: None,
        source_commit_id: ado_pr
            .last_merge_source_commit
            .as_ref()
            .map(|c| c.commit_id.clone()),
        web_url: format!(
            "{}/{}/{}/_git/{}/pullrequest/{}",
            base_url, project, project, repo, ado_pr.pull_request_id
        ),
    }
}

fn map_reviewer(r: &AdoReviewer) -> Reviewer {
    Reviewer {
        user: User {
            id: r.id.clone(),
            display_name: r.display_name.clone(),
            unique_name: r.unique_name.clone(),
        },
        vote: map_vote(r.vote),
        is_required: r.is_required.unwrap_or(false),
    }
}

/// Azure DevOps vote values:
///  10 = approved, 5 = approved with suggestions, 0 = no vote,
/// -5 = waiting for author, -10 = rejected
fn map_vote(vote: i32) -> Vote {
    match vote {
        10 => Vote::Approved,
        5 => Vote::ApprovedWithSuggestions,
        0 => Vote::NoVote,
        -5 => Vote::WaitingForAuthor,
        -10 => Vote::Rejected,
        _ => Vote::NoVote,
    }
}

fn map_status(status: &str) -> PrStatus {
    match status {
        "active" => PrStatus::Active,
        "completed" => PrStatus::Completed,
        "abandoned" => PrStatus::Abandoned,
        _ => PrStatus::Active,
    }
}

fn map_merge_status(status: &str) -> MergeStatus {
    match status {
        "succeeded" => MergeStatus::Succeeded,
        "conflicts" => MergeStatus::Conflicts,
        "rejectedByPolicy" => MergeStatus::RejectedByPolicy,
        "queued" => MergeStatus::Queued,
        _ => MergeStatus::NotSet,
    }
}

pub fn map_policy_evaluation(eval: &AdoPolicyEvaluation) -> PolicyStatus {
    PolicyStatus {
        name: eval.configuration.policy_type.display_name.clone(),
        is_blocking: eval.configuration.is_blocking,
        status: match eval.status.as_str() {
            "approved" => PolicyEvaluation::Approved,
            "rejected" => PolicyEvaluation::Rejected,
            "running" => PolicyEvaluation::Running,
            "queued" => PolicyEvaluation::Queued,
            _ => PolicyEvaluation::NotApplicable,
        },
    }
}

fn strip_refs_prefix(refname: &str) -> String {
    refname
        .strip_prefix("refs/heads/")
        .unwrap_or(refname)
        .to_string()
}
