use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,
    #[serde(default = "default_stale_threshold")]
    pub stale_threshold_hours: u64,
    #[serde(default = "default_true")]
    pub notifications_enabled: bool,
    #[serde(default)]
    pub notifications: NotificationConfig,
    #[serde(default)]
    pub provider_indicator: ProviderIndicator,
    #[serde(default = "default_warning_hours")]
    pub age_warning_hours: u64,
    #[serde(default = "default_danger_hours")]
    pub age_danger_hours: u64,
    #[serde(default = "default_true")]
    pub show_project_name: bool,
    #[serde(default)]
    pub parse_conventional_commits: bool,
    #[serde(default = "default_tabs")]
    pub tabs: Vec<TabConfig>,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            refresh_interval_secs: default_refresh_interval(),
            stale_threshold_hours: default_stale_threshold(),
            notifications_enabled: true,
            notifications: NotificationConfig::default(),
            provider_indicator: ProviderIndicator::default(),
            age_warning_hours: default_warning_hours(),
            age_danger_hours: default_danger_hours(),
            show_project_name: true,
            parse_conventional_commits: false,
            tabs: default_tabs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderIndicator {
    Off,
    Border,
    Badge,
}

impl Default for ProviderIndicator {
    fn default() -> Self {
        Self::Border
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    #[serde(default = "default_true")]
    pub new_pr: bool,
    #[serde(default = "default_true")]
    pub vote_changed: bool,
    #[serde(default = "default_true")]
    pub waiting_for_author: bool,
    #[serde(default = "default_true")]
    pub build_failed: bool,
    #[serde(default = "default_true")]
    pub completed: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            new_pr: true,
            vote_changed: true,
            waiting_for_author: true,
            build_failed: true,
            completed: true,
        }
    }
}

fn default_refresh_interval() -> u64 {
    60
}

fn default_stale_threshold() -> u64 {
    48
}

fn default_warning_hours() -> u64 {
    48
}

fn default_danger_hours() -> u64 {
    144
}

fn default_true() -> bool {
    true
}

fn default_tabs() -> Vec<TabConfig> {
    vec![
        TabConfig {
            label: "Reviewing".into(),
            source: TabSource::Reviewing,
            display: TabDisplay::Reviewing,
            enabled: true,
            filter: TabFilter::default(),
        },
        TabConfig {
            label: "Authored".into(),
            source: TabSource::Authored,
            display: TabDisplay::Authored,
            enabled: true,
            filter: TabFilter::default(),
        },
        TabConfig {
            label: "Up for grabs".into(),
            source: TabSource::All,
            display: TabDisplay::Reviewing,
            enabled: false,
            filter: TabFilter {
                max_reviewers: Some(0),
                hide_drafts: Some(true),
                ..TabFilter::default()
            },
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabConfig {
    pub label: String,
    #[serde(default = "TabSource::default")]
    pub source: TabSource,
    #[serde(default = "TabDisplay::default")]
    pub display: TabDisplay,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub filter: TabFilter,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TabSource {
    Reviewing,
    Authored,
    All,
}

impl Default for TabSource {
    fn default() -> Self {
        Self::Reviewing
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TabDisplay {
    Reviewing,
    Authored,
}

impl Default for TabDisplay {
    fn default() -> Self {
        Self::Reviewing
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TabFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_reviewers: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hide_drafts: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cc_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProviderConfig {
    #[serde(rename = "azure-devops")]
    AzureDevOps(AzureDevOpsConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureDevOpsConfig {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub projects: Vec<ProjectFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFilter {
    pub name: String,
    #[serde(default)]
    pub repos: Vec<String>,
}
