use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use xai_grok_i18n::{t, t_fmt};

use crate::acp::meta::NotificationMeta;
use crate::acp::tracker::AcpUpdateTracker;
use crate::scrollback::export::render_blocks_to_markdown;
use crate::scrollback::state::ScrollbackState;

#[derive(Debug, clap::Args, Clone)]
pub struct ExportArgs {
    #[arg(help = t("cli.export.help.session_id"))]
    pub session_id: String,
    #[arg(help = t("cli.export.help.output"))]
    pub output: Option<PathBuf>,
    #[arg(long, short, help = t("cli.export.help.clipboard"))]
    pub clipboard: bool,
}

pub fn run(args: ExportArgs) -> Result<()> {
    tracing::info!(session_id = %args.session_id, "export_cmd: starting session export");

    let updates = xai_grok_shell::session::storage::load_updates_for_replay(&args.session_id)?
        .with_context(|| {
            t_fmt(
                "cli.export.session_not_found",
                &[("session_id", args.session_id.as_str())],
            )
        })?;

    let mut tracker = AcpUpdateTracker::new();
    let mut scrollback = ScrollbackState::new();
    let replay_meta = NotificationMeta {
        is_replay: true,
        ..Default::default()
    };

    for update in updates {
        tracker.handle_update(update, &replay_meta, &mut scrollback);
    }

    let blocks: Vec<_> = (0..scrollback.len())
        .filter_map(|i| scrollback.entry(i).map(|e| &e.block))
        .collect();
    let md = render_blocks_to_markdown(blocks);

    if md.is_empty() {
        anyhow::bail!(t_fmt(
            "cli.export.no_content",
            &[("session_id", args.session_id.as_str())]
        ));
    }

    if let Some(path) = args.output {
        let expanded = PathBuf::from(shellexpand::tilde(&path.to_string_lossy()).as_ref());
        if let Some(parent) = expanded.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                t_fmt(
                    "cli.export.create_failed",
                    &[("path", parent.to_string_lossy().as_ref())],
                )
            })?;
        }
        std::fs::write(&expanded, &md).with_context(|| {
            t_fmt(
                "cli.export.write_failed",
                &[("path", expanded.to_string_lossy().as_ref())],
            )
        })?;
        tracing::info!(
            session_id = %args.session_id,
            path = %expanded.display(),
            bytes = md.len(),
            "export_cmd: wrote transcript to file"
        );
        eprintln!(
            "{}",
            t_fmt(
                "cli.export.exported",
                &[("path", expanded.to_string_lossy().as_ref())]
            )
        );
    } else if args.clipboard {
        let _ = crate::clipboard::copy_text(&md);
        let lines = md.lines().count();
        tracing::info!(
            session_id = %args.session_id,
            bytes = md.len(),
            lines,
            "export_cmd: copied transcript to clipboard"
        );
        let chars = md.len().to_string();
        let lines = lines.to_string();
        eprintln!(
            "{}",
            t_fmt(
                "cli.export.copied_to_clipboard",
                &[("chars", chars.as_str()), ("lines", lines.as_str())]
            )
        );
    } else {
        std::io::stdout()
            .write_all(md.as_bytes())
            .with_context(|| t("cli.export.stdout_write_failed"))?;
        std::io::stdout()
            .write_all(b"\n")
            .with_context(|| t("cli.export.stdout_write_failed"))?;
    }

    Ok(())
}
