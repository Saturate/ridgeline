use std::time::Duration;

use base64::Engine;
use reqwest::Client;

use crate::providers::traits::ProviderError;

#[derive(Clone)]
pub struct AzureDevOpsClient {
    client: Client,
    base_url: String,
    auth_header: String,
}

impl AzureDevOpsClient {
    pub fn new(base_url: &str, pat: &str) -> Self {
        let encoded = base64::engine::general_purpose::STANDARD.encode(format!(":{}", pat));
        let auth_header = format!("Basic {}", encoded);

        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_header,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
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

            let response = match self
                .client
                .get(&url)
                .header("Authorization", &self.auth_header)
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
                return Err(ProviderError::Auth {
                    provider: self.base_url.clone(),
                    message: format!("HTTP {status} - check your PAT token"),
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

            return response.json::<T>().await.map_err(|e| {
                ProviderError::Deserialize(format!(
                    "failed to parse response from {url}: {e}"
                ))
            });
        }

        Err(last_err.unwrap_or_else(|| ProviderError::Other("max retries exceeded".to_string())))
    }
}
