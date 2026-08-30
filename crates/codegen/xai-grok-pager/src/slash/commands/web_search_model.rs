//! `/web-search-model` (aliases `/wsm` and `/web_search_model`) -- choose the model used by Web Search.

use agent_client_protocol as acp;

use crate::acp::model_state::ModelState;
use crate::app::actions::Action;
use crate::slash::command::{AppCtx, ArgItem, CommandExecCtx, CommandResult, SlashCommand};

/// Choose the independent Web Search model.
pub struct WebSearchModelCommand;

impl SlashCommand for WebSearchModelCommand {
    fn name(&self) -> &str {
        "web-search-model"
    }

    fn aliases(&self) -> &[&str] {
        &["wsm", "web_search_model"]
    }

    fn description(&self) -> &str {
        xai_grok_i18n::t("slash.web-search-model.description")
    }

    fn session_scoped(&self) -> bool {
        false
    }

    fn usage(&self) -> &str {
        "/web-search-model <name>"
    }

    fn takes_args(&self) -> bool {
        true
    }

    fn args_required(&self) -> bool {
        true
    }

    fn arg_placeholder(&self) -> Option<&str> {
        Some("<model>")
    }

    fn suggest_args(&self, ctx: &AppCtx, _args_query: &str) -> Option<Vec<ArgItem>> {
        if ctx.models.is_empty() {
            return None;
        }
        Some(
            ctx.models
                .available
                .values()
                .map(|info| ArgItem {
                    display: info.name.clone(),
                    match_text: info.name.clone(),
                    insert_text: info.name.clone(),
                    description: info.description.clone().unwrap_or_default(),
                })
                .collect(),
        )
    }

    fn run(&self, ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        let trimmed = args.trim();
        if trimmed.is_empty() {
            return CommandResult::Error(
                xai_grok_i18n::t("slash.err.usage_web-search-model").into(),
            );
        }

        if let Some(id) = ctx.models.resolve_by_name_or_id(trimmed) {
            return CommandResult::Action(Action::SetWebSearchModel(id));
        }

        CommandResult::Error(
            xai_grok_i18n::t_or(
                "slash.web-search-model.unknown",
                "Unknown Web Search model: {model}",
            )
            .replace("{model}", trimmed),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    static EMPTY_BUNDLE: crate::app::bundle::BundleState = crate::app::bundle::BundleState {
        has_cache: false,
        version: String::new(),
        personas: Vec::new(),
        roles: Vec::new(),
        agents: Vec::new(),
        skills: Vec::new(),
        persona_details: Vec::new(),
        role_details: Vec::new(),
    };

    fn ctx<'a>(models: &'a ModelState) -> CommandExecCtx<'a> {
        CommandExecCtx {
            models,
            session_id: None,
            bundle_state: &EMPTY_BUNDLE,
            screen_mode: crate::app::ScreenMode::Inline,
            billing_surface_visible: true,
            usage_command_visible: true,
            pager_state: crate::settings::PagerLocalSnapshot::default(),
        }
    }

    #[test]
    fn metadata_uses_web_search_alias() {
        let command = WebSearchModelCommand;
        assert_eq!(command.name(), "web-search-model");
        assert_eq!(command.aliases(), &["wsm", "web_search_model"]);
        assert!(!command.session_scoped());
    }

    #[test]
    fn run_resolves_model_without_changing_chat_selection() {
        let mut models = ModelState::default();
        let id = acp::ModelId::new(Arc::from("search-model"));
        models
            .available
            .insert(id.clone(), acp::ModelInfo::new(id.clone(), "Search Model".to_string()));
        let mut context = ctx(&models);

        assert!(matches!(
            WebSearchModelCommand.run(&mut context, "search model"),
            CommandResult::Action(Action::SetWebSearchModel(resolved)) if resolved == id
        ));
        assert!(models.current.is_none());
    }

    #[test]
    fn run_rejects_unknown_model() {
        let models = ModelState::default();
        let mut context = ctx(&models);
        assert!(matches!(
            WebSearchModelCommand.run(&mut context, "missing"),
            CommandResult::Error(message) if message.contains("missing")
        ));
    }
}
