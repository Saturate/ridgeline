use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;

use crate::auth::{Credential, PatCredential};
use crate::providers::traits::ProviderError;

#[derive(Clone)]
pub struct AzureDevOpsClient {
    client: Client,
    base_url: String,
    credential: Arc<dyn Credential>,
}

impl AzureDevOpsClient {
    pub fn new(base_url: &str, pat: &str) -> Self {
        Self::with_credential(base_url, Arc::new(PatCredential::new(pat)))
    }

    pub fn with_credential(base_url: &str, credential: Arc<dyn Credential>) -> Self {
        Self {
            // reqwest has no timeout by default, and one stalled request
            // would hold up the whole poll. Generous enough for Azure's
            // slower org-wide PR queries; a timeout is retried like any
            // other network error.
            client: Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(30))
                .build()
                .expect("HTTP client with timeouts"),
            base_url: base_url.trim_end_matches('/').to_string(),
            credential,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn credential(&self) -> &dyn Credential {
        &*self.credential
    }

    pub async fn get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, ProviderError> {
        let url = format!("{}{}", self.base_url, path);
        let max_retries: u32 = 3;
        let mut last_err: Option<ProviderError> = None;

        for attempt in 0..=max_retries {
            if attempt > 0 {
                let delay = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                tokio::time::sleep(delay).await;
            }

            let auth_header = self.credential.auth_header().await?;

            let response = match self
                .client
                .get(&url)
                .header("Authorization", &auth_header)
                .header("Accept", "application/json")
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    last_err = Some(ProviderError::Http(e));
                    continue;
                }
            };

            let status = response.status();

            if status == reqwest::StatusCode::UNAUTHORIZED
                || status == reqwest::StatusCode::FORBIDDEN
            {
                return Err(ProviderError::Denied {
                    provider: self.base_url.clone(),
                    message: format!(
                        "HTTP {status} - check your {} credentials",
                        self.credential.display_name()
                    ),
                });
            }

            if status.is_server_error() {
                let body = response.text().await.unwrap_or_default();
                last_err = Some(ProviderError::Api {
                    provider: self.base_url.clone(),
                    status: status.as_u16(),
                    message: body,
                });
                continue;
            }

            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(ProviderError::Api {
                    provider: self.base_url.clone(),
                    status: status.as_u16(),
                    message: body,
                });
            }

            match response.json::<T>().await {
                Ok(value) => return Ok(value),
                // The timeout covers reading the body too, and the big
                // org-wide PR lists are the likeliest to run into it there.
                Err(e) if e.is_timeout() => {
                    last_err = Some(ProviderError::Http(e));
                    continue;
                }
                Err(e) => {
                    return Err(ProviderError::Deserialize(format!(
                        "failed to parse response from {url}: {e}"
                    )));
                }
            }
        }

        Err(last_err.unwrap_or_else(|| ProviderError::Other("max retries exceeded".to_string())))
    }
}
