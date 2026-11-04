use async_trait::async_trait;
use thiserror::Error;

use super::types::{PrDetail, PrId, ProviderType, PullRequest, UserId};

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("authentication failed for provider '{provider}': {message}")]
    Auth {
        provider: String,
        message: String,
    },

    #[error("API error from {provider}: {status} {message}")]
    Api {
        provider: String,
        status: u16,
        message: String,
    },

    #[error("deserialization error: {0}")]
    Deserialize(String),

    #[error("{0}")]
    Other(String),
}

#[async_trait]
pub trait PrProvider: Send + Sync {
    /// Display name for this provider instance (e.g. "contoso")
    fn name(&self) -> &str;

    /// Provider type for display (e.g. "Azure DevOps", "GitHub")
    fn provider_type(&self) -> ProviderType;

    /// Get the current user's identity
    async fn get_current_user(&self) -> Result<UserId, ProviderError>;

    /// List PRs where the user is a reviewer
    async fn list_reviewing(&self, user: &UserId) -> Result<Vec<PullRequest>, ProviderError>;

    /// List PRs created by the user
    async fn list_authored(&self, user: &UserId) -> Result<Vec<PullRequest>, ProviderError>;

    /// Get detailed info for a single PR (policies, full description, etc.)
    async fn get_detail(&self, pr_id: &PrId) -> Result<PrDetail, ProviderError>;

    /// Build the web URL for a PR
    fn web_url(&self, pr_id: &PrId) -> String;
}
