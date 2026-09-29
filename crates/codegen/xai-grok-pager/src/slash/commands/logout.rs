//! `/logout` -- remove auth credentials and return to the login screen.

use crate::app::actions::Action;
use crate::slash::command::{AppCtx, CommandExecCtx, CommandResult, SlashCommand, slash_meta};
use xai_grok_config::{Capability, Distribution};

pub struct LogoutCommand;

impl SlashCommand for LogoutCommand {
    fn name(&self) -> &str {
        "logout"
    }

    fn description(&self) -> &str {
        xai_grok_i18n::t("slash.logout.description")
    }

    fn usage(&self) -> &str {
        "/logout"
    }

    /// Not offered where the build has no account; the router refuses a typed one.
    fn visible(&self, _ctx: &AppCtx) -> bool {
        Distribution::current().allows(Capability::AccountLogin)
    }

    fn run(&self, _ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        CommandResult::Action(Action::Logout)
    }
}
