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

        let response = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await?;

        let status = response.status();

        if status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::FORBIDDEN
        {
            return Err(ProviderError::Auth {
                provider: self.base_url.clone(),
                message: format!("HTTP {status} - check your PAT token"),
            });
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::Api {
                provider: self.base_url.clone(),
                status: status.as_u16(),
                message: body,
            });
        }

        response.json::<T>().await.map_err(|e| {
            ProviderError::Deserialize(format!("failed to parse response from {url}: {e}"))
        })
    }
}
