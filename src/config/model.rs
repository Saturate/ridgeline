use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
}

#[derive(Debug, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,
    #[serde(default = "default_stale_threshold")]
    pub stale_threshold_hours: u64,
    #[serde(default = "default_true")]
    pub notifications_enabled: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            refresh_interval_secs: default_refresh_interval(),
            stale_threshold_hours: default_stale_threshold(),
            notifications_enabled: true,
        }
    }
}

fn default_refresh_interval() -> u64 {
    60
}

fn default_stale_threshold() -> u64 {
    48
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ProviderConfig {
    #[serde(rename = "azure-devops")]
    AzureDevOps(AzureDevOpsConfig),
}

#[derive(Debug, Deserialize)]
pub struct AzureDevOpsConfig {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub projects: Vec<ProjectFilter>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectFilter {
    pub name: String,
    #[serde(default)]
    pub repos: Vec<String>,
}
