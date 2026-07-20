//! `/context` -- show detailed context usage (instant, not queued).

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};

/// Show context usage breakdown (progress bar, token categories, stats).
pub struct ContextCommand;

impl SlashCommand for ContextCommand {
    fn name(&self) -> &str {
        "context"
    }

    fn description(&self) -> &str {
        xai_grok_i18n::t("slash.context.description")
    }

    fn session_scoped(&self) -> bool {
        true
    }

    fn usage(&self) -> &str {
        "/context"
    }

    fn run(&self, ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        if ctx.session_id.is_none() {
            return CommandResult::Error(
                xai_grok_i18n::t("slash.err.no_active_session").to_string(),
            );
        }

        CommandResult::Action(Action::ShowContextInfo)
    }
}
