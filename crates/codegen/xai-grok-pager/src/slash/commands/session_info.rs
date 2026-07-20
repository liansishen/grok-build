//! `/session-info` -- show current session info (instant, not queued).

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};

/// Show session info (session ID, cwd, model, context usage).
pub struct SessionInfoCommand;

impl SlashCommand for SessionInfoCommand {
    fn name(&self) -> &str {
        "session-info"
    }

    fn description(&self) -> &str {
        xai_grok_i18n::t("slash.session-info.description")
    }

    fn session_scoped(&self) -> bool {
        true
    }

    fn usage(&self) -> &str {
        "/session-info"
    }

    fn run(&self, ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        // Check if we have an active session
        if ctx.session_id.is_none() {
            return CommandResult::Error(
                xai_grok_i18n::t("slash.err.no_active_session").to_string(),
            );
        }

        CommandResult::Action(Action::ShowSessionInfo)
    }
}
