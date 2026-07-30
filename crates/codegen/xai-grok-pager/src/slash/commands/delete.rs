//! `/delete` — delete this session's history and return home.

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};

pub struct DeleteCommand;

impl SlashCommand for DeleteCommand {
    fn name(&self) -> &str {
        "delete"
    }

    fn description(&self) -> &str {
        xai_grok_i18n::t("slash.delete.description")
    }

    fn session_scoped(&self) -> bool {
        true
    }

    fn usage(&self) -> &str {
        "/delete"
    }

    fn run(&self, ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        if ctx.session_id.is_none() {
            return CommandResult::Error(xai_grok_i18n::t("slash.delete.no_session").into());
        }
        CommandResult::Action(Action::DeleteCurrentSession)
    }
}
