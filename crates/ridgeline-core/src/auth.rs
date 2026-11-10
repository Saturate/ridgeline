use async_trait::async_trait;

use crate::providers::traits::ProviderError;

#[async_trait]
pub trait Credential: Send + Sync {
    async fn auth_header(&self) -> Result<String, ProviderError>;

    fn display_name(&self) -> &str;
}

/// PAT-based auth: the token is static and encoded as Basic auth.
pub struct PatCredential {
    header: String,
}

impl PatCredential {
    pub fn new(pat: &str) -> Self {
        let encoded = base64::engine::general_purpose::STANDARD.encode(format!(":{pat}"));
        Self {
            header: format!("Basic {encoded}"),
        }
    }
}

#[async_trait]
impl Credential for PatCredential {
    async fn auth_header(&self) -> Result<String, ProviderError> {
        Ok(self.header.clone())
    }

    fn display_name(&self) -> &str {
        "PAT"
    }
}

/// Reuses the token from an active `az login` session.
/// Calls `az account get-access-token` on each request (the CLI caches internally).
pub struct AzureCliCredential;

const AZURE_DEVOPS_RESOURCE: &str = "499b84ac-1321-427f-aa17-267ca6975798";

#[async_trait]
impl Credential for AzureCliCredential {
    async fn auth_header(&self) -> Result<String, ProviderError> {
        let output = tokio::process::Command::new("az")
            .args([
                "account",
                "get-access-token",
                "--resource",
                AZURE_DEVOPS_RESOURCE,
                "--query",
                "accessToken",
                "--output",
                "tsv",
            ])
            .output()
            .await
            .map_err(|e| ProviderError::Auth {
                provider: "azure-cli".into(),
                message: format!("failed to run `az` CLI: {e}"),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ProviderError::Auth {
                provider: "azure-cli".into(),
                message: format!(
                    "`az account get-access-token` failed: {stderr}. Run `az login` first."
                ),
            });
        }

        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if token.is_empty() {
            return Err(ProviderError::Auth {
                provider: "azure-cli".into(),
                message: "empty token from `az` CLI".into(),
            });
        }

        Ok(format!("Bearer {token}"))
    }

    fn display_name(&self) -> &str {
        "Azure CLI"
    }
}

/// Device code flow via Microsoft Entra ID. The user opens a URL in their
/// browser, enters a code, and the app polls until auth completes.
///
/// Requires a registered Entra ID app (client_id) with the Azure DevOps
/// user_impersonation scope.
pub struct DeviceCodeCredential {
    client_id: String,
    tenant_id: String,
    token: tokio::sync::RwLock<Option<TokenCache>>,
}

struct TokenCache {
    access_token: String,
    expires_at: std::time::Instant,
    refresh_token: Option<String>,
}

impl DeviceCodeCredential {
    pub fn new(client_id: &str, tenant_id: &str) -> Self {
        Self {
            client_id: client_id.to_string(),
            tenant_id: tenant_id.to_string(),
            token: tokio::sync::RwLock::new(None),
        }
    }

    /// Initiates the device code flow. Returns the user_code and verification_uri
    /// for the user to complete in their browser. Call `poll_for_token` after.
    pub async fn start_device_flow(&self) -> Result<DeviceCodeResponse, ProviderError> {
        let url = format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/devicecode",
            self.tenant_id
        );

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .form(&[
                ("client_id", self.client_id.as_str()),
                (
                    "scope",
                    "499b84ac-1321-427f-aa17-267ca6975798/.default offline_access",
                ),
            ])
            .send()
            .await
            .map_err(|e| ProviderError::Auth {
                provider: "entra-id".into(),
                message: format!("device code request failed: {e}"),
            })?;

        let body: serde_json::Value = resp.json().await.map_err(|e| ProviderError::Auth {
            provider: "entra-id".into(),
            message: format!("invalid device code response: {e}"),
        })?;

        Ok(DeviceCodeResponse {
            device_code: body["device_code"].as_str().unwrap_or_default().to_string(),
            user_code: body["user_code"].as_str().unwrap_or_default().to_string(),
            verification_uri: body["verification_uri"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            interval: body["interval"].as_u64().unwrap_or(5),
            expires_in: body["expires_in"].as_u64().unwrap_or(900),
        })
    }

    /// Polls the token endpoint until the user completes browser sign-in.
    pub async fn poll_for_token(&self, device_code: &str) -> Result<String, ProviderError> {
        let url = format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            self.tenant_id
        );
        let client = reqwest::Client::new();

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            let resp = client
                .post(&url)
                .form(&[
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("client_id", &self.client_id),
                    ("device_code", device_code),
                ])
                .send()
                .await
                .map_err(|e| ProviderError::Auth {
                    provider: "entra-id".into(),
                    message: format!("token poll failed: {e}"),
                })?;

            let body: serde_json::Value = resp.json().await.map_err(|e| ProviderError::Auth {
                provider: "entra-id".into(),
                message: format!("invalid token response: {e}"),
            })?;

            if let Some(error) = body["error"].as_str() {
                match error {
                    "authorization_pending" => continue,
                    "slow_down" => {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        continue;
                    }
                    "expired_token" => {
                        return Err(ProviderError::Auth {
                            provider: "entra-id".into(),
                            message: "device code expired, try again".into(),
                        });
                    }
                    _ => {
                        let desc = body["error_description"].as_str().unwrap_or(error);
                        return Err(ProviderError::Auth {
                            provider: "entra-id".into(),
                            message: desc.to_string(),
                        });
                    }
                }
            }

            let access_token = body["access_token"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let refresh_token = body["refresh_token"].as_str().map(str::to_string);
            let expires_in = body["expires_in"].as_u64().unwrap_or(3600);

            let mut cache = self.token.write().await;
            *cache = Some(TokenCache {
                access_token: access_token.clone(),
                expires_at: std::time::Instant::now()
                    + std::time::Duration::from_secs(expires_in.saturating_sub(60)),
                refresh_token,
            });

            return Ok(access_token);
        }
    }

    async fn refresh_if_needed(&self) -> Result<String, ProviderError> {
        {
            let cache = self.token.read().await;
            if let Some(ref tc) = *cache {
                if std::time::Instant::now() < tc.expires_at {
                    return Ok(tc.access_token.clone());
                }
            }
        }

        let refresh_token = {
            let cache = self.token.read().await;
            cache.as_ref().and_then(|tc| tc.refresh_token.clone())
        };

        if let Some(refresh) = refresh_token {
            let url = format!(
                "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
                self.tenant_id
            );
            let client = reqwest::Client::new();
            let resp = client
                .post(&url)
                .form(&[
                    ("grant_type", "refresh_token"),
                    ("client_id", &self.client_id),
                    ("refresh_token", &refresh),
                    (
                        "scope",
                        "499b84ac-1321-427f-aa17-267ca6975798/.default offline_access",
                    ),
                ])
                .send()
                .await
                .map_err(|e| ProviderError::Auth {
                    provider: "entra-id".into(),
                    message: format!("token refresh failed: {e}"),
                })?;

            let body: serde_json::Value = resp.json().await.map_err(|e| ProviderError::Auth {
                provider: "entra-id".into(),
                message: format!("invalid refresh response: {e}"),
            })?;

            if body["error"].is_string() {
                return Err(ProviderError::Auth {
                    provider: "entra-id".into(),
                    message: "refresh token expired, sign in again".into(),
                });
            }

            let access_token = body["access_token"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let new_refresh = body["refresh_token"].as_str().map(str::to_string);
            let expires_in = body["expires_in"].as_u64().unwrap_or(3600);

            let mut cache = self.token.write().await;
            *cache = Some(TokenCache {
                access_token: access_token.clone(),
                expires_at: std::time::Instant::now()
                    + std::time::Duration::from_secs(expires_in.saturating_sub(60)),
                refresh_token: new_refresh,
            });

            return Ok(access_token);
        }

        Err(ProviderError::Auth {
            provider: "entra-id".into(),
            message: "no token cached, run device code flow first".into(),
        })
    }
}

#[async_trait]
impl Credential for DeviceCodeCredential {
    async fn auth_header(&self) -> Result<String, ProviderError> {
        let token = self.refresh_if_needed().await?;
        Ok(format!("Bearer {token}"))
    }

    fn display_name(&self) -> &str {
        "Entra ID (Browser)"
    }
}

pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

use base64::Engine;
