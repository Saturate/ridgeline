mod server;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use rmcp::ServiceExt;

use crate::config::model::ProviderConfig;
use crate::config::{credentials, loader};
use crate::providers::azure_devops::AzureDevOpsProvider;
use crate::providers::traits::PrProvider;
use crate::providers::types::UserId;

use server::RidgelineMcp;

pub async fn run_stdio() -> anyhow::Result<()> {
    let config = loader::load_config()?;
    let mut providers: Vec<(Arc<dyn PrProvider>, UserId)> = Vec::new();

    for provider_config in &config.providers {
        match provider_config {
            ProviderConfig::AzureDevOps(ado_config) => {
                let pat = credentials::get_token(&ado_config.name)?;
                let provider = AzureDevOpsProvider::new(ado_config, &pat);
                let user = provider.get_current_user().await?;
                eprintln!("ridgeline-mcp: connected to {} ({})", ado_config.name, user.display_name);
                providers.push((Arc::new(provider), user));
            }
        }
    }

    if providers.is_empty() {
        eprintln!("ridgeline-mcp: no providers configured");
    }

    let service = RidgelineMcp::new(providers)
        .serve(rmcp::transport::stdio())
        .await?;

    service.waiting().await?;
    Ok(())
}
