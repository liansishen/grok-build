//! `/share` -- share current session via URL.

use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};

/// Share the current session via a public URL.
pub struct ShareCommand;

impl SlashCommand for ShareCommand {
    fn name(&self) -> &str {
        "share"
    }

    fn description(&self) -> &str {
        xai_grok_i18n::t("slash.share.description")
    }

    fn session_scoped(&self) -> bool {
        true
    }

    fn usage(&self) -> &str {
        "/share"
    }

    fn run(&self, ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        let _ = ctx;
        CommandResult::Error(
            xai_grok_i18n::t("slash.share.temporarily_disabled").to_string(),
        )
    }
}
