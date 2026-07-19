use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};

pub struct RewindCommand;

impl SlashCommand for RewindCommand {
    fn name(&self) -> &str {
        "rewind"
    }

    fn description(&self) -> &str {
        xai_grok_i18n::t("slash.rewind.description")
    }

    fn session_scoped(&self) -> bool {
        true
    }

    fn usage(&self) -> &str {
        "/rewind"
    }

    fn run(&self, _ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        CommandResult::Action(Action::RewindShowPicker)
    }
}
