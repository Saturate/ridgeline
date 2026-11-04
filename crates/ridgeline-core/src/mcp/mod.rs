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
    let init_futs: Vec<_> = config
        .providers
        .iter()
        .map(|provider_config| async {
            match provider_config {
                ProviderConfig::AzureDevOps(ado_config) => {
                    let pat = credentials::get_token(&ado_config.name).map_err(|e| {
                        eprintln!("ridgeline-mcp: skipping provider {}: {}", ado_config.name, e);
                    }).ok()?;
                    let provider = AzureDevOpsProvider::new(ado_config, &pat);
                    let user = provider.get_current_user().await.map_err(|e| {
                        eprintln!("ridgeline-mcp: skipping provider {}: {}", ado_config.name, e);
                    }).ok()?;
                    eprintln!("ridgeline-mcp: connected to {} ({})", ado_config.name, user.display_name);
                    Some((Arc::new(provider) as Arc<dyn PrProvider>, user))
                }
            }
        })
        .collect();

    let providers: Vec<(Arc<dyn PrProvider>, UserId)> =
        futures::future::join_all(init_futs).await.into_iter().flatten().collect();

    if providers.is_empty() {
        eprintln!("ridgeline-mcp: no providers configured");
    }

    let service = RidgelineMcp::new(providers)
        .serve(rmcp::transport::stdio())
        .await?;

    service.waiting().await?;
    Ok(())
}
