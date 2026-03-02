use thiserror::Error;

#[derive(Debug, Error)]
pub enum PollError {
    #[error("provider '{provider}' failed: {message}")]
    ProviderFailed { provider: String, message: String },
}

/// Result of a single poll cycle — may contain partial results
#[derive(Debug)]
pub struct PollResult {
    pub reviewing: Vec<crate::providers::types::PullRequest>,
    pub authored: Vec<crate::providers::types::PullRequest>,
    pub errors: Vec<PollError>,
}
