//! `/docs` -- open How-to Guides (in-TUI) or online Build docs.
//!
//! Bare `/docs` opens the same DocPicker as command-palette "How-to Guides".
//! `/docs web` opens https://docs.x.ai/build/overview in the browser.
//! `/docs <title>` opens a single guide by title (case-insensitive).

use crate::app::actions::Action;
use crate::docs::{default_howto_entries, find_localized_doc};
use crate::slash::command::{AppCtx, ArgItem, CommandExecCtx, CommandResult, SlashCommand};

/// Online Build docs landing page (hardcoded like other TUI deep-links; docs.x.ai can redirect if the path moves).
pub const BUILD_DOCS_URL: &str = "https://docs.x.ai/build/overview";

/// Open How-to Guides or online Build docs.
pub struct DocsCommand;

impl SlashCommand for DocsCommand {
    fn name(&self) -> &str {
        "docs"
    }

    fn aliases(&self) -> &[&str] {
        &["howto", "guides"]
    }

    fn description(&self) -> &str {
        xai_grok_i18n::t("slash.docs.description")
    }

    fn usage(&self) -> &str {
        "/docs [web|title]"
    }

    fn takes_args(&self) -> bool {
        true
    }

    fn args_required(&self) -> bool {
        false
    }

    fn arg_placeholder(&self) -> Option<&str> {
        Some("[web|title]")
    }

    fn suggest_args(&self, _ctx: &AppCtx, _args_query: &str) -> Option<Vec<ArgItem>> {
        let mut items = vec![
            ArgItem {
                display: "how-to".into(),
                match_text: "how-to".into(),
                insert_text: "how-to".into(),
                description: xai_grok_i18n::t_or(
                    "slash.docs.arg_howto",
                    "Browse in-TUI How-to Guides",
                )
                .into(),
            },
            ArgItem {
                display: "web".into(),
                match_text: "web".into(),
                insert_text: "web".into(),
                description: xai_grok_i18n::t_or(
                    "slash.docs.arg_web",
                    "Open docs.x.ai/build in the browser",
                )
                .into(),
            },
        ];
        items.extend(default_howto_entries().into_iter().map(|doc| {
            ArgItem {
                display: doc.title.clone(),
                match_text: doc.title.clone(),
                insert_text: doc.title.clone(),
                description: xai_grok_i18n::t_or("slash.docs.open_title", "Open \"{title}\"")
                    .replace("{title}", &doc.title),
            }
        }));
        Some(items)
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        let trimmed = args.trim();
        if trimmed.is_empty() || is_howto_list_arg(trimmed) {
            return CommandResult::Action(Action::OpenHowtoGuides);
        }
        if is_web_arg(trimmed) {
            return CommandResult::Action(Action::OpenUrl(BUILD_DOCS_URL.into()));
        }
        match find_localized_doc(trimmed) {
            Some(localized) => CommandResult::Action(Action::ShowReleaseNotes {
                title: localized.title,
                content: localized.content.into(),
            }),
            None => CommandResult::Error(
                xai_grok_i18n::t_or(
                    "slash.docs.unknown_target",
                    "Unknown docs target {target}. Try /docs, /docs web, or a guide title (e.g. /docs Getting Started).",
                )
                .replace("{target}", &format!("{trimmed:?}")),
            ),
        }
    }
}

fn is_howto_list_arg(arg: &str) -> bool {
    matches!(
        arg.to_ascii_lowercase().as_str(),
        "how-to" | "howto" | "guides" | "guide" | "list" | "tui"
    )
}

fn is_web_arg(arg: &str) -> bool {
    matches!(
        arg.to_ascii_lowercase().as_str(),
        "web" | "online" | "browser" | "site" | "www"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::model_state::ModelState;

    static DEFAULT_BUNDLE_STATE: crate::app::bundle::BundleState =
        crate::app::bundle::BundleState {
            has_cache: false,
            version: String::new(),
            personas: Vec::new(),
            roles: Vec::new(),
            agents: Vec::new(),
            skills: Vec::new(),
            persona_details: Vec::new(),
            role_details: Vec::new(),
        };

    fn make_ctx<'a>(models: &'a ModelState) -> CommandExecCtx<'a> {
        CommandExecCtx {
            models,
            session_id: None,
            bundle_state: &DEFAULT_BUNDLE_STATE,
            screen_mode: crate::app::ScreenMode::Inline,
            billing_surface_visible: true,
            usage_command_visible: true,
            pager_state: crate::settings::PagerLocalSnapshot {
                multiline_mode: false,
                yolo_mode: false,
                ..crate::settings::PagerLocalSnapshot::default()
            },
        }
    }

    #[test]
    fn bare_docs_opens_howto_guides() {
        let models = ModelState::default();
        let mut ctx = make_ctx(&models);
        assert!(matches!(
            DocsCommand.run(&mut ctx, ""),
            CommandResult::Action(Action::OpenHowtoGuides)
        ));
    }

    #[test]
    fn howto_aliases_open_list() {
        let models = ModelState::default();
        let mut ctx = make_ctx(&models);
        for args in ["how-to", "howto", "guides", "list", "tui"] {
            assert!(
                matches!(
                    DocsCommand.run(&mut ctx, args),
                    CommandResult::Action(Action::OpenHowtoGuides)
                ),
                "args={args:?}"
            );
        }
    }

    #[test]
    fn web_opens_build_docs_url() {
        let models = ModelState::default();
        let mut ctx = make_ctx(&models);
        for args in ["web", "online", "browser"] {
            match DocsCommand.run(&mut ctx, args) {
                CommandResult::Action(Action::OpenUrl(url)) => {
                    assert_eq!(url, BUILD_DOCS_URL, "args={args:?}");
                }
                other => panic!("expected OpenUrl for args={args:?}, got {other:?}"),
            }
        }
    }

    #[test]
    #[serial_test::serial(GROK_UI_LOCALE)]
    fn title_opens_guide() {
        struct RestoreLocale(xai_grok_i18n::Locale);
        impl Drop for RestoreLocale {
            fn drop(&mut self) {
                xai_grok_i18n::set_locale(self.0);
            }
        }

        let _restore = RestoreLocale(xai_grok_i18n::current_locale());
        xai_grok_i18n::set_locale(xai_grok_i18n::Locale::En);
        let models = ModelState::default();
        let mut ctx = make_ctx(&models);
        match DocsCommand.run(&mut ctx, "Agent Dashboard") {
            CommandResult::Action(Action::ShowReleaseNotes { title, content }) => {
                assert_eq!(title, "Agent Dashboard");
                assert!(content.starts_with("# Agent Dashboard"));
            }
            other => panic!("expected ShowReleaseNotes, got {other:?}"),
        }
    }

    #[test]
    #[serial_test::serial(GROK_UI_LOCALE)]
    fn translated_guides_open_with_zh_cn_content() {
        struct RestoreLocale(xai_grok_i18n::Locale);
        impl Drop for RestoreLocale {
            fn drop(&mut self) {
                xai_grok_i18n::set_locale(self.0);
            }
        }

        let _restore = RestoreLocale(xai_grok_i18n::current_locale());
        xai_grok_i18n::set_locale(xai_grok_i18n::Locale::ZhCn);
        let models = ModelState::default();
        let mut ctx = make_ctx(&models);

        for (requested, expected_content) in [
            (
                "Agent Dashboard",
                include_str!("../../../docs/user-guide/zh-CN/23-dashboard.md"),
            ),
            (
                "Monitoring Usage (External OpenTelemetry)",
                include_str!("../../../docs/user-guide/zh-CN/24-monitoring-usage.md"),
            ),
        ] {
            match DocsCommand.run(&mut ctx, requested) {
                CommandResult::Action(Action::ShowReleaseNotes { title, content }) => {
                    assert_eq!(title, requested);
                    assert_eq!(content, expected_content);
                }
                other => panic!("expected ShowReleaseNotes for {requested:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn unknown_target_errors() {
        let models = ModelState::default();
        let mut ctx = make_ctx(&models);
        assert!(matches!(
            DocsCommand.run(&mut ctx, "not-a-real-guide"),
            CommandResult::Error(_)
        ));
    }

    #[test]
    fn aliases_and_metadata() {
        let cmd = DocsCommand;
        assert_eq!(cmd.name(), "docs");
        assert_eq!(cmd.aliases(), &["howto", "guides"]);
        assert!(cmd.takes_args());
        assert!(!cmd.args_required());
    }

    #[test]
    fn suggest_args_includes_web_and_titles() {
        let models = ModelState::default();
        let cwd = std::path::Path::new(".");
        let ctx = AppCtx {
            models: &models,
            cwd,
            has_session_announcements: false,
            billing_surface_visible: true,
            usage_command_visible: true,
            workflows_available: true,
            screen_mode: crate::app::ScreenMode::Fullscreen,
        };
        let items = DocsCommand.suggest_args(&ctx, "").expect("suggestions");
        assert!(items.iter().any(|i| i.insert_text == "web"));
        assert!(items.iter().any(|i| i.insert_text == "how-to"));
        assert!(items.iter().any(|i| i.insert_text == "Getting Started"));
    }
}
