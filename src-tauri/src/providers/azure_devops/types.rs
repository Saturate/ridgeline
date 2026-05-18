use serde::Deserialize;

/// Azure DevOps wraps list responses in a `value` array
#[derive(Debug, Deserialize)]
pub struct ListResponse<T> {
    pub value: Vec<T>,
    pub count: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoPullRequest {
    pub pull_request_id: u64,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub is_draft: Option<bool>,
    pub creation_date: String,
    pub source_ref_name: String,
    pub target_ref_name: String,
    pub created_by: AdoIdentity,
    pub reviewers: Option<Vec<AdoReviewer>>,
    pub repository: AdoRepository,
    pub labels: Option<Vec<AdoLabel>>,
    pub merge_status: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoIdentity {
    pub id: String,
    pub display_name: String,
    pub unique_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoReviewer {
    pub id: String,
    pub display_name: String,
    pub unique_name: Option<String>,
    pub vote: i32,
    pub is_required: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoRepository {
    pub id: String,
    pub name: String,
    pub project: AdoProject,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoProject {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoLabel {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoConnectionData {
    pub authenticated_user: AdoAuthenticatedUser,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoAuthenticatedUser {
    pub id: String,
    pub properties: Option<AdoUserProperties>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoUserProperties {
    #[serde(rename = "Account")]
    pub account: Option<AdoPropertyValue>,
}

#[derive(Debug, Deserialize)]
pub struct AdoPropertyValue {
    #[serde(rename = "$value")]
    pub value: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoPolicyEvaluation {
    pub configuration: AdoPolicyConfiguration,
    pub status: String,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoPolicyConfiguration {
    pub is_blocking: bool,
    #[serde(rename = "type")]
    pub policy_type: AdoPolicyType,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoPolicyType {
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoThread {
    pub id: u64,
    pub status: Option<String>,
    pub comments: Option<Vec<AdoComment>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoComment {
    pub id: u64,
    pub content: Option<String>,
    pub author: AdoIdentity,
    pub comment_type: Option<String>,
}
