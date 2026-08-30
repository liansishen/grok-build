//! `ptyctl run` — spawn a PTY session and start the HTTP server.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use tokio::net::TcpListener;

use ptyctl::pty::PtyConfig;
use ptyctl::server;
use ptyctl::session::{PtySession, SessionConfig};

use crate::registry;
use xai_grok_i18n::{t, t_fmt};

/// Run the `ptyctl run` command.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    command: Vec<String>,
    width: u16,
    height: u16,
    cwd: Option<PathBuf>,
    env_vars: Vec<String>,
    port: u16,
    name: Option<String>,
    force: bool,
    timeout: Option<u64>,
    linger: bool,
    quiet: bool,
) -> Result<()> {
    // Refuse to take over a name whose server is still reachable unless --force; stale entries are replaced.
    if let Some(ref session_name) = name
        && !force
        && let Ok(existing) = registry::lookup_session(session_name)
        && registry::server_alive(existing.port).await
    {
        bail!(
            "{}",
            t_fmt(
                "ptyctl.run.already_running",
                &[("name", session_name), ("port", &existing.port.to_string())],
            )
        );
    }

    // Parse env vars.
    let mut env = HashMap::new();
    for var in &env_vars {
        if let Some((k, v)) = var.split_once('=') {
            env.insert(k.to_string(), v.to_string());
        }
    }

    let cwd_str = cwd
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| ".".into());

    let config = SessionConfig {
        pty: PtyConfig {
            command: command.clone(),
            cols: width,
            rows: height,
            cwd,
            env,
        },
        timeout,
        linger,
    };

    // Start the session.
    let session = PtySession::start(config).await?;
    let pid = session.status_basic().1;

    // Build the HTTP server.
    let router = server::build_router(session);

    // Bind to the requested port.
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr)
        .await
        .context(t("ptyctl.error.bind"))?;
    let actual_addr = listener.local_addr()?;
    let actual_port = actual_addr.port();

    // Register named session.
    if let Some(ref session_name) = name {
        let info = registry::SessionInfo {
            port: actual_port,
            pid,
            command: command.clone(),
            cwd: cwd_str,
            started_at: chrono::Utc::now().to_rfc3339(),
        };
        registry::register_session(session_name, &info)?;
    }

    if !quiet {
        eprintln!("{}", t_fmt("ptyctl.server.command", &[("command", &command.join(" "))]));
        if let Some(p) = pid {
            eprintln!("{}", t_fmt("ptyctl.server.pid", &[("pid", &p.to_string())]));
        }
        eprintln!("{}", t_fmt("ptyctl.server.listening", &[("port", &actual_port.to_string())]));
    } else {
        println!("{actual_port}");
    }

    // Serve until shutdown.
    let shutdown_result = axum::serve(listener, router)
        .await
        .context(t("ptyctl.error.server"));

    // Clean up only a registration that still points at this server; a --force takeover may have replaced it.
    if let Some(ref session_name) = name
        && let Ok(info) = registry::lookup_session(session_name)
        && info.port == actual_port
    {
        let _ = registry::unregister_session(session_name);
    }

    shutdown_result
}
