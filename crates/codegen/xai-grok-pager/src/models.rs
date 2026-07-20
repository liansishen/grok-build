//! `grok models` subcommand.

use anyhow::Result;
use tokio_util::sync::CancellationToken;
use xai_grok_i18n::{t, t_fmt};
use xai_grok_shell::agent::config::Config as AgentConfig;
use xai_grok_shell::cli_models::{AuthStatus, list_models};

use crate::client_identity::{PAGER_CLIENT_TYPE, PAGER_CLIENT_VERSION};

pub async fn list_available_models(agent_config: &AgentConfig) -> Result<()> {
    match AuthStatus::resolve(agent_config) {
        AuthStatus::ApiKey => println!("{}", t("cli.models.auth.api_key")),
        AuthStatus::LoggedIn(host) => println!(
            "{}",
            t_fmt("cli.models.auth.logged_in", &[("host", host.as_str())])
        ),
        AuthStatus::ModelCredentials(model) => {
            println!(
                "{}",
                t_fmt(
                    "cli.models.auth.model_credentials",
                    &[("model", model.as_str())]
                )
            );
        }
        AuthStatus::DeploymentKey => println!("{}", t("cli.models.auth.deployment_key")),
        AuthStatus::NotAuthenticated => println!("{}", t("cli.models.auth.not_authenticated")),
    }
    println!();

    let cancel = CancellationToken::new();
    let spawned = crate::acp::spawn::spawn_grok_shell(agent_config.clone(), &cancel, None).await?;

    let state = list_models(&spawned.channel.tx, PAGER_CLIENT_TYPE, PAGER_CLIENT_VERSION).await?;

    println!(
        "{}",
        t_fmt(
            "cli.models.default_model",
            &[("model", state.current_model_id.0.as_str())]
        )
    );
    println!();
    println!("{}", t("cli.models.available_models"));
    for m in state.available_models {
        if m.model_id == state.current_model_id {
            println!(
                "{}",
                t_fmt(
                    "cli.models.model_default",
                    &[("model", m.model_id.0.as_str())]
                )
            );
        } else {
            println!("  - {}", m.model_id.0);
        }
    }

    cancel.cancel();
    Ok(())
}
