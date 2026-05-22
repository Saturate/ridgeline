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
}
