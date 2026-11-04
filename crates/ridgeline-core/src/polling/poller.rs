use std::sync::Arc;

use futures::future::join_all;

use super::error::{PollError, PollErrorKind, PollResult};
use crate::providers::traits::{PrProvider, ProviderError};
use crate::providers::types::UserId;

pub struct Poller {
    providers: Vec<(Arc<dyn PrProvider>, UserId)>,
}

impl Poller {
    pub fn new(providers: Vec<(Arc<dyn PrProvider>, UserId)>) -> Self {
        Self { providers }
    }

    pub async fn poll_once(&self) -> PollResult {
        let mut all_reviewing = Vec::new();
        let mut all_authored = Vec::new();
        let mut errors = Vec::new();

        let review_futures: Vec<_> = self
            .providers
            .iter()
            .map(|(provider, user)| {
                let provider = Arc::clone(provider);
                let user = user.clone();
                async move {
                    let result = provider.list_reviewing(&user).await;
                    (provider.name().to_string(), result)
                }
            })
            .collect();

        let authored_futures: Vec<_> = self
            .providers
            .iter()
            .map(|(provider, user)| {
                let provider = Arc::clone(provider);
                let user = user.clone();
                async move {
                    let result = provider.list_authored(&user).await;
                    (provider.name().to_string(), result)
                }
            })
            .collect();

        let (review_results, authored_results) =
            tokio::join!(join_all(review_futures), join_all(authored_futures));

        for (name, result) in review_results {
            match result {
                Ok(prs) => all_reviewing.extend(prs),
                Err(e) => errors.push(poll_error(name, e)),
            }
        }

        for (name, result) in authored_results {
            match result {
                Ok(prs) => all_authored.extend(prs),
                Err(e) => errors.push(poll_error(name, e)),
            }
        }

        PollResult {
            reviewing: all_reviewing,
            authored: all_authored,
            errors,
        }
    }
}

fn poll_error(provider: String, error: ProviderError) -> PollError {
    let kind = match &error {
        ProviderError::Http(_) => PollErrorKind::Network,
        ProviderError::Auth { .. } => PollErrorKind::Auth,
        ProviderError::Api { status, .. } if *status >= 500 => PollErrorKind::Server,
        ProviderError::Api { .. } => PollErrorKind::Unknown,
        ProviderError::Deserialize(_) => PollErrorKind::Parse,
        ProviderError::Other(_) => PollErrorKind::Unknown,
    };
    PollError {
        provider,
        kind,
        message: error.to_string(),
    }
}
