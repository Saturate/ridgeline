mod config;
mod notifications;
mod polling;
mod providers;
mod state;
mod ui;
mod util;

use std::sync::Arc;

use anyhow::{Context, Result};
use async_compat::CompatExt;
use clap::{Parser, Subcommand};
use iocraft::prelude::*;

use config::credentials;
use config::model::ProviderConfig;
use polling::poller::Poller;
use providers::azure_devops::AzureDevOpsProvider;
use providers::traits::PrProvider;
use ui::app::App;
use ui::views::setup::SetupWizard;

#[derive(Parser)]
#[command(name = "ridgeline", about = "Monitor pull requests across providers")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage authentication tokens
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    /// Show config file location
    Config,
    /// Run the setup wizard to configure providers
    Setup,
}

#[derive(Subcommand)]
enum AuthAction {
    /// Store a PAT for a provider
    Add {
        /// Provider name (must match name in config.toml)
        provider: String,
    },
    /// Remove a stored PAT
    Remove {
        /// Provider name
        provider: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Auth { action }) => handle_auth(action),
        Some(Commands::Config) => {
            let path = config::loader::config_path()?;
            println!("Config file: {}", path.display());
            if !path.exists() {
                println!("(does not exist yet)");
                config::loader::ensure_config_dir()?;
                println!("Created config directory. Add your config.toml there.");
            }
            Ok(())
        }
        Some(Commands::Setup) => run_setup(),
        None => run_tui(),
    }
}

fn handle_auth(action: AuthAction) -> Result<()> {
    match action {
        AuthAction::Add { provider } => {
            eprintln!("Enter PAT for provider '{provider}':");
            let mut token = String::new();
            std::io::stdin()
                .read_line(&mut token)
                .context("failed to read token from stdin")?;
            let token = token.trim();

            if token.is_empty() {
                anyhow::bail!("token cannot be empty");
            }

            credentials::store_token(&provider, token)?;
            eprintln!("Token stored for '{provider}'");
            Ok(())
        }
        AuthAction::Remove { provider } => {
            credentials::delete_token(&provider)?;
            eprintln!("Token removed for '{provider}'");
            Ok(())
        }
    }
}

fn run_setup() -> Result<()> {
    smol::block_on(element! { SetupWizard() }.fullscreen())?;
    Ok(())
}

enum InitResult {
    Success {
        instances: Vec<(Arc<dyn PrProvider>, providers::types::UserId)>,
        names: Vec<String>,
    },
    NeedsSetup(String),
}

/// Try to authenticate and validate all configured providers.
/// Returns NeedsSetup with a reason if something is wrong.
fn init_providers(config: &config::model::Config) -> Result<InitResult> {
    smol::block_on(async {
        let mut instances: Vec<(Arc<dyn PrProvider>, providers::types::UserId)> = Vec::new();
        let mut names = Vec::new();

        for provider_config in &config.providers {
            match provider_config {
                ProviderConfig::AzureDevOps(ado_config) => {
                    let pat = match credentials::get_token(&ado_config.name) {
                        Ok(p) => p,
                        Err(_) => {
                            return Ok(InitResult::NeedsSetup(format!(
                                "No token found for provider '{}'",
                                ado_config.name
                            )));
                        }
                    };

                    let provider = AzureDevOpsProvider::new(ado_config, &pat);

                    let user = match provider.get_current_user().compat().await {
                        Ok(u) => u,
                        Err(e) => {
                            return Ok(InitResult::NeedsSetup(format!(
                                "Authentication failed for '{}': {}",
                                ado_config.name, e
                            )));
                        }
                    };

                    // Warn about invalid projects but don't block startup
                    // (org-level queries work without project config)
                    if !ado_config.projects.is_empty() {
                        if let Ok(invalid) = provider.validate_projects().compat().await {
                            if !invalid.is_empty() {
                                eprintln!(
                                    "Warning: project(s) not found in '{}': {} (skipping)",
                                    ado_config.name,
                                    invalid.join(", ")
                                );
                            }
                        }
                    }

                    eprintln!(
                        "Authenticated as '{}' on '{}'",
                        user.display_name, ado_config.name
                    );
                    names.push(ado_config.name.clone());
                    instances.push((Arc::new(provider), user));
                }
            }
        }

        Ok(InitResult::Success { instances, names })
    })
}

fn run_tui() -> Result<()> {
    let mut config = match config::loader::load_config() {
        Ok(c) if !c.providers.is_empty() => c,
        _ => {
            run_setup()?;
            config::loader::load_config()?
        }
    };

    let (provider_instances, provider_names) = match init_providers(&config)? {
        InitResult::Success { instances, names } => (instances, names),
        InitResult::NeedsSetup(reason) => {
            eprintln!("{reason}");
            eprintln!("Launching setup wizard...\n");
            run_setup()?;
            config = config::loader::load_config()?;
            match init_providers(&config)? {
                InitResult::Success { instances, names } => (instances, names),
                InitResult::NeedsSetup(reason) => {
                    anyhow::bail!("{reason}\nRun `ridgeline setup` to reconfigure.");
                }
            }
        }
    };

    if provider_instances.is_empty() {
        anyhow::bail!("No providers configured. Run `ridgeline setup` to configure.");
    }

    let poller = Arc::new(Poller::new(provider_instances));

    smol::block_on(
        element! {
            App(
                poller: poller,
                refresh_interval_secs: config.general.refresh_interval_secs,
                stale_threshold_hours: config.general.stale_threshold_hours,
                notifications_enabled: config.general.notifications_enabled,
                provider_names: provider_names,
            )
        }
        .fullscreen(),
    )?;

    Ok(())
}
