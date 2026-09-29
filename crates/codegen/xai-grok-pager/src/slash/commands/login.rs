//! `/login` -- log in or re-authenticate with your account.

use crate::app::actions::Action;
use crate::slash::command::{AppCtx, CommandExecCtx, CommandResult, SlashCommand, slash_meta};
use xai_grok_config::{Capability, Distribution};

pub struct LoginCommand;

impl SlashCommand for LoginCommand {
    fn name(&self) -> &str {
        "login"
    }

    fn description(&self) -> &str {
        xai_grok_i18n::t("slash.login.description")
    }

    fn usage(&self) -> &str {
        "/login"
    }

    /// Not offered where the build has no account; the router refuses a typed one.
    fn visible(&self, _ctx: &AppCtx) -> bool {
        Distribution::current().allows(Capability::AccountLogin)
    }

    fn run(&self, _ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        CommandResult::Action(Action::Login)
    }
}
