use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PollErrorKind {
    Network,
    Auth,
    Server,
    Parse,
    Unknown,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PollError {
    pub provider: String,
    pub kind: PollErrorKind,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct PollResult {
    pub reviewing: Vec<crate::providers::types::PullRequest>,
    pub authored: Vec<crate::providers::types::PullRequest>,
    pub errors: Vec<PollError>,
    /// Providers whose reviewing / authored query failed this poll. Their
    /// part of that list is missing, not empty, so callers should keep the
    /// previous PRs for them instead of treating them as gone.
    #[serde(skip)]
    pub failed_reviewing: Vec<String>,
    #[serde(skip)]
    pub failed_authored: Vec<String>,
}
