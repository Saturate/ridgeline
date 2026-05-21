use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerInfo};
use rmcp::{schemars, tool, tool_handler, tool_router, ServerHandler};

use crate::polling::poller::Poller;
use crate::providers::traits::PrProvider;
use crate::providers::types::UserId;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListPrParams {
    #[schemars(description = "Filter by provider/tenant name (e.g. 'contoso'). Omit to list from all.")]
    pub provider: Option<String>,
    #[schemars(description = "Filter by project name. Omit to list from all projects.")]
    pub project: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetPrDetailParams {
    #[schemars(description = "Provider/tenant name (e.g. 'contoso')")]
    pub provider: String,
    #[schemars(description = "Project name")]
    pub project: String,
    #[schemars(description = "Repository name")]
    pub repository: String,
    #[schemars(description = "Pull request number")]
    pub number: u64,
}

#[derive(Clone)]
pub struct RidgelineMcp {
    providers: Arc<Vec<(Arc<dyn PrProvider>, UserId)>>,
    tool_router: ToolRouter<Self>,
}

impl std::fmt::Debug for RidgelineMcp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RidgelineMcp").finish()
    }
}

impl RidgelineMcp {
    pub fn new(providers: Vec<(Arc<dyn PrProvider>, UserId)>) -> Self {
        Self {
            providers: Arc::new(providers),
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl RidgelineMcp {
    #[tool(description = "List configured Azure DevOps tenants/providers and the authenticated user for each.")]
    async fn list_providers(&self) -> Result<String, String> {
        let info: Vec<serde_json::Value> = self
            .providers
            .iter()
            .map(|(p, user)| {
                serde_json::json!({
                    "name": p.name(),
                    "user": user.display_name,
                })
            })
            .collect();
        serde_json::to_string_pretty(&info).map_err(|e| e.to_string())
    }

    #[tool(description = "List pull requests across configured Azure DevOps organizations. Returns PRs you are reviewing and PRs you authored. Optionally filter by provider/tenant and/or project.")]
    async fn list_pull_requests(
        &self,
        Parameters(params): Parameters<ListPrParams>,
    ) -> Result<String, String> {
        let providers_to_poll: Vec<_> = self
            .providers
            .iter()
            .filter(|(p, _)| {
                params
                    .provider
                    .as_ref()
                    .map_or(true, |name| p.name().eq_ignore_ascii_case(name))
            })
            .cloned()
            .collect();

        if providers_to_poll.is_empty() {
            if let Some(name) = &params.provider {
                return Err(format!("provider '{}' not found", name));
            }
        }

        let poller = Poller::new(providers_to_poll);
        let result = poller.poll_once().await;

        let filter_project = |prs: Vec<crate::providers::types::PullRequest>| -> Vec<crate::providers::types::PullRequest> {
            match &params.project {
                Some(project) => prs
                    .into_iter()
                    .filter(|pr| pr.id.project.eq_ignore_ascii_case(project))
                    .collect(),
                None => prs,
            }
        };

        serde_json::to_string_pretty(&serde_json::json!({
            "reviewing": filter_project(result.reviewing),
            "authored": filter_project(result.authored),
            "errors": result.errors,
        }))
        .map_err(|e| e.to_string())
    }

    #[tool(description = "Get detailed information about a specific pull request including policies, diff stats, and build status.")]
    async fn get_pull_request_detail(
        &self,
        Parameters(params): Parameters<GetPrDetailParams>,
    ) -> Result<String, String> {
        let pr_id = crate::providers::types::PrId {
            provider: params.provider.clone(),
            project: params.project,
            repository: params.repository,
            number: params.number,
        };

        let (provider, _) = self
            .providers
            .iter()
            .find(|(p, _)| p.name() == pr_id.provider)
            .ok_or_else(|| format!("provider '{}' not found", pr_id.provider))?;

        let detail = provider
            .get_detail(&pr_id)
            .await
            .map_err(|e| e.to_string())?;

        serde_json::to_string_pretty(&detail).map_err(|e| e.to_string())
    }
}

#[tool_handler]
impl ServerHandler for RidgelineMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("ridgeline", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Ridgeline provides cross-tenant pull request data from Azure DevOps. \
                 Use list_providers to see configured tenants, list_pull_requests to see PRs \
                 (optionally filtered by provider/project), and get_pull_request_detail for specifics.",
            )
    }
}
