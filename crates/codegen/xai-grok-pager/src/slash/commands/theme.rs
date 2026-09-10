//! `/theme` (alias `/t`): switch the color theme.
//!
//! Toggles between available themes or switches to a named theme.
//! Selecting `auto` makes the theme follow the system appearance.
//! Selecting an explicit theme turns auto mode off.
//!
//! `run` dispatches `Action::SetTheme(<canonical>)`; the dispatcher handles the state change, persistence, and the toast.
//! `preview_arg` and `cancel_preview` use `Theme::preview_name` so previews never persist or change auto-mode configuration.

use crate::app::actions::Action;
use crate::slash::command::{
    AppCtx, ArgItem, CommandExecCtx, CommandResult, SlashCommand, slash_meta,
};
use crate::slash::{ModeSupport, Remedy};
use crate::theme::{Theme, ThemeKind, cache as theme_cache};

pub struct ThemeCommand;

impl SlashCommand for ThemeCommand {
    slash_meta! {
        name: "theme",
        aliases: ["t"],
        description: "Switch the color theme",
        usage: "/theme <name>",
        takes_args: true,
        args_required: false,
        mode_support: ModeSupport::FullscreenOnly(Remedy::SwitchMode {
            why: "minimal renders with your terminal's own palette",
        }),
        arg_placeholder: "<theme>",
    }

    fn supports_preview(&self) -> bool {
        true
    }

    fn preview_state(&self) -> Option<String> {
        Some(theme_cache::current_name())
    }

    fn preview_arg(&self, arg: &str) {
        let _ = Theme::preview_name(arg);
    }

    fn cancel_preview(&self, previous: &str) {
        let _ = Theme::preview_name(previous);
    }

    fn suggest_args(&self, _ctx: &AppCtx, _args_query: &str) -> Option<Vec<ArgItem>> {
        theme_cache::reload_custom_themes();
        let current = theme_cache::current_name();
        let is_auto = theme_cache::is_auto_mode();
        let choices = theme_cache::theme_choices(None);

        // Prepend "auto" (follow system appearance) as the first option.
        let auto_active = if is_auto { " (active)" } else { "" };
        let mut items = vec![ArgItem {
            display: "auto".to_string(),
            match_text: "auto".to_string(),
            insert_text: "auto".to_string(),
            description: format!("auto (follow system){auto_active}"),
        }];

        // Concrete themes: only show "(active)" when not in auto mode.
        items.extend(choices.into_iter().map(|theme| {
            let active = if theme.canonical == current && !is_auto {
                " (active)"
            } else {
                ""
            };
            ArgItem {
                display: theme.display_name.clone(),
                match_text: theme.canonical.clone(),
                insert_text: theme.canonical,
                description: format!("{}{active}", theme.display_name),
            }
        }));

        Some(items)
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        let trimmed = args.trim();
        theme_cache::reload_custom_themes();
        let choices = theme_cache::theme_choices(None);

        // No args: toggle between available themes.
        if trimmed.is_empty() {
            if choices.is_empty() {
                return CommandResult::Error("No themes are available".to_string());
            }
            let current = theme_cache::current_name();
            let current_idx = choices
                .iter()
                .position(|theme| theme.canonical == current)
                .unwrap_or(0);
            let Some(next) = choices.get((current_idx + 1) % choices.len()) else {
                return CommandResult::Error("No themes are available".to_string());
            };
            return CommandResult::Action(Action::SetTheme(next.canonical.clone()));
        }

        // Named theme (including "auto"): canonicalize built-in aliases and loaded custom names.
        match crate::theme::canonical_name(trimmed) {
            Some(canonical) => CommandResult::Action(Action::SetTheme(canonical)),
            None => {
                let all_names: Vec<&str> = choices.iter().map(|theme| theme.canonical.as_str()).collect();
                CommandResult::Error(format!(
                    "Unknown theme: {}. Available: auto, {}",
                    trimmed,
                    all_names.join(", ")
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{cache as theme_cache, system_appearance};

    /// Run a test with a clean in-memory state.
    /// Prevents disk reads by pre-loading the theme state.
    fn with_test_env(f: impl FnOnce()) {
        let _guard = theme_cache::test_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        theme_cache::reset_for_test();
        theme_cache::seed_auto_theme_defaults_for_test();
        system_appearance::clear_mock();
        // Set LOADED=true so current_kind() doesn't try to read from disk.
        theme_cache::set(ThemeKind::GrokNight);
        f();
        system_appearance::clear_mock();
        theme_cache::reset_for_test();
    }

    // -- suggest_args ---------------------------------------------------------

    #[test]
    fn suggest_args_prepends_auto_option() {
        with_test_env(|| {
            let cmd = ThemeCommand;
            let models = crate::acp::model_state::ModelState::default();
            let ctx = AppCtx {
                models: &models,
                cwd: std::path::Path::new("."),
                has_session_announcements: false,
                billing_surface_visible: true,
                usage_command_visible: true,
                workflows_available: true,
                saved_workflows: &[],
                workflow_runs: &[],
                screen_mode: crate::app::ScreenMode::Fullscreen,
                current_title: None,
            };
            let items = cmd.suggest_args(&ctx, "").expect("should return items");
            assert_eq!(items[0].insert_text, "auto");
            assert!(items[0].description.contains("follow system"));
            // The "auto" entry plus every available concrete theme
            assert_eq!(items.len(), theme_cache::theme_choices(None).len() + 1);
        });
    }

    #[test]
    fn suggest_args_auto_active_when_auto_mode() {
        with_test_env(|| {
            theme_cache::set_auto_mode(true);
            let cmd = ThemeCommand;
            let models = crate::acp::model_state::ModelState::default();
            let ctx = AppCtx {
                models: &models,
                cwd: std::path::Path::new("."),
                has_session_announcements: false,
                billing_surface_visible: true,
                usage_command_visible: true,
                workflows_available: true,
                saved_workflows: &[],
                workflow_runs: &[],
                screen_mode: crate::app::ScreenMode::Fullscreen,
                current_title: None,
            };
            let items = cmd.suggest_args(&ctx, "").expect("should return items");
            assert!(
                items[0].description.contains("(active)"),
                "auto should show (active), got: {}",
                items[0].description
            );
        });
    }

    #[test]
    fn suggest_args_auto_not_active_when_explicit() {
        with_test_env(|| {
            theme_cache::set_auto_mode(false);
            let cmd = ThemeCommand;
            let models = crate::acp::model_state::ModelState::default();
            let ctx = AppCtx {
                models: &models,
                cwd: std::path::Path::new("."),
                has_session_announcements: false,
                billing_surface_visible: true,
                usage_command_visible: true,
                workflows_available: true,
                saved_workflows: &[],
                workflow_runs: &[],
                screen_mode: crate::app::ScreenMode::Fullscreen,
                current_title: None,
            };
            let items = cmd.suggest_args(&ctx, "").expect("should return items");
            assert!(
                !items[0].description.contains("(active)"),
                "auto should not show (active), got: {}",
                items[0].description
            );
        });
    }

    #[test]
    fn suggest_args_explicit_active_when_not_auto() {
        with_test_env(|| {
            theme_cache::set_auto_mode(false);
            theme_cache::set(ThemeKind::GrokNight);
            let cmd = ThemeCommand;
            let models = crate::acp::model_state::ModelState::default();
            let ctx = AppCtx {
                models: &models,
                cwd: std::path::Path::new("."),
                has_session_announcements: false,
                billing_surface_visible: true,
                usage_command_visible: true,
                workflows_available: true,
                saved_workflows: &[],
                workflow_runs: &[],
                screen_mode: crate::app::ScreenMode::Fullscreen,
                current_title: None,
            };
            let items = cmd.suggest_args(&ctx, "").expect("should return items");
            let groknight = items
                .iter()
                .find(|i| i.insert_text == "groknight")
                .expect("groknight should be in list");
            assert!(
                groknight.description.contains("(active)"),
                "explicit theme should show (active), got: {}",
                groknight.description
            );
        });
    }

    #[test]
    fn suggest_args_no_explicit_active_when_auto() {
        with_test_env(|| {
            theme_cache::set_auto_mode(true);
            theme_cache::set(ThemeKind::GrokNight);
            let cmd = ThemeCommand;
            let models = crate::acp::model_state::ModelState::default();
            let ctx = AppCtx {
                models: &models,
                cwd: std::path::Path::new("."),
                has_session_announcements: false,
                billing_surface_visible: true,
                usage_command_visible: true,
                workflows_available: true,
                saved_workflows: &[],
                workflow_runs: &[],
                screen_mode: crate::app::ScreenMode::Fullscreen,
                current_title: None,
            };
            let items = cmd.suggest_args(&ctx, "").expect("should return items");
            // No concrete theme should show "(active)" in auto mode.
            for item in items.iter().skip(1) {
                assert!(
                    !item.description.contains("(active)"),
                    "{} should not show (active) in auto mode",
                    item.insert_text
                );
            }
        });
    }

    // -- run (dispatches Action::SetTheme) ------------------------------------

    /// `/theme <name>` returns `Action::SetTheme(<canonical>)`; the dispatcher handles the in-memory state, the disk write, and the toast.
    #[test]
    fn run_explicit_dispatches_set_theme_action() {
        with_test_env(|| {
            let cmd = ThemeCommand;
            let models = crate::acp::model_state::ModelState::default();
            let bundle = crate::app::bundle::BundleState::default();
            let mut ctx = CommandExecCtx {
                models: &models,
                session_id: None,
                bundle_state: &bundle,
                screen_mode: crate::app::ScreenMode::Inline,
                billing_surface_visible: true,
                usage_command_visible: true,
                pager_state: crate::settings::PagerLocalSnapshot {
                    multiline_mode: false,
                    yolo_mode: false,
                    ..crate::settings::PagerLocalSnapshot::default()
                },
            };
            let result = cmd.run(&mut ctx, "groknight");
            match result {
                CommandResult::Action(Action::SetTheme(name)) => {
                    assert_eq!(name, "groknight");
                }
                other => panic!("expected Action::SetTheme(\"groknight\"), got {other:?}"),
            }
            match cmd.run(&mut ctx, "opencode") {
                CommandResult::Action(Action::SetTheme(name)) => assert_eq!(name, "opencode"),
                other => panic!("expected custom theme action, got {other:?}"),
            }
        });
    }

    /// While the terminal-theme rollout gate is off, a typed `/theme terminal` (or alias) is an unknown name whose error listing omits it, and it drops out of the suggestions.
    #[test]
    fn run_terminal_rejected_and_unlisted_while_gated_off() {
        with_test_env(|| {
            theme_cache::set_terminal_theme_enabled(false);
            let cmd = ThemeCommand;
            let models = crate::acp::model_state::ModelState::default();
            let bundle = crate::app::bundle::BundleState::default();
            let mut ctx = CommandExecCtx {
                models: &models,
                session_id: None,
                bundle_state: &bundle,
                screen_mode: crate::app::ScreenMode::Inline,
                billing_surface_visible: true,
                usage_command_visible: true,
                pager_state: crate::settings::PagerLocalSnapshot {
                    multiline_mode: false,
                    yolo_mode: false,
                    ..crate::settings::PagerLocalSnapshot::default()
                },
            };
            for name in ["terminal", "transparent"] {
                match cmd.run(&mut ctx, name) {
                    CommandResult::Error(msg) => {
                        assert!(msg.contains("Unknown theme"), "got: {msg}");
                        let listing = msg.split("Available:").nth(1).expect("listing");
                        assert!(!listing.contains("terminal"), "gated name listed: {msg}");
                    }
                    other => panic!("expected CommandResult::Error, got {other:?}"),
                }
            }
            let app_ctx = AppCtx {
                models: &models,
                cwd: std::path::Path::new("."),
                has_session_announcements: false,
                billing_surface_visible: true,
                usage_command_visible: true,
                workflows_available: true,
                saved_workflows: &[],
                workflow_runs: &[],
                screen_mode: crate::app::ScreenMode::Fullscreen,
                current_title: None,
            };
            let items = cmd.suggest_args(&app_ctx, "").expect("should return items");
            assert!(
                items.iter().all(|i| i.insert_text != "terminal"),
                "gated theme must not be suggested"
            );

            theme_cache::set_terminal_theme_enabled(true);
            match cmd.run(&mut ctx, "terminal") {
                CommandResult::Action(Action::SetTheme(name)) => assert_eq!(name, "terminal"),
                other => panic!("expected Action::SetTheme(\"terminal\"), got {other:?}"),
            }
        });
    }

    /// `/theme` (no args) toggles by dispatching `Action::SetTheme(<next>)`.
    /// The next theme is selected from the loaded concrete theme catalog.
    #[test]
    fn run_toggle_dispatches_set_theme_action() {
        with_test_env(|| {
            theme_cache::set(ThemeKind::GrokNight);
            // The command must not panic even when the catalog is runtime-loaded.
            assert!(
                !theme_cache::theme_choices(None).is_empty(),
                "toggle test requires at least one available theme"
            );
            let cmd = ThemeCommand;
            let models = crate::acp::model_state::ModelState::default();
            let bundle = crate::app::bundle::BundleState::default();
            let mut ctx = CommandExecCtx {
                models: &models,
                session_id: None,
                bundle_state: &bundle,
                screen_mode: crate::app::ScreenMode::Inline,
                billing_surface_visible: true,
                usage_command_visible: true,
                pager_state: crate::settings::PagerLocalSnapshot {
                    multiline_mode: false,
                    yolo_mode: false,
                    ..crate::settings::PagerLocalSnapshot::default()
                },
            };
            let result = cmd.run(&mut ctx, "");
            match result {
                CommandResult::Action(Action::SetTheme(name)) => {
                    let choices = theme_cache::theme_choices(None);
                    let current = theme_cache::current_name();
                    let current_idx = choices
                        .iter()
                        .position(|theme| theme.canonical == current)
                        .unwrap_or(0);
                    let expected = &choices[(current_idx + 1) % choices.len()].canonical;
                    assert_eq!(&name, expected);
                }
                other => panic!("expected Action::SetTheme(...), got {other:?}"),
            }
        });
    }

    #[test]
    fn run_auto_dispatches_set_theme_auto() {
        with_test_env(|| {
            let cmd = ThemeCommand;
            let models = crate::acp::model_state::ModelState::default();
            let bundle = crate::app::bundle::BundleState::default();
            let mut ctx = CommandExecCtx {
                models: &models,
                session_id: None,
                bundle_state: &bundle,
                screen_mode: crate::app::ScreenMode::Inline,
                billing_surface_visible: true,
                usage_command_visible: true,
                pager_state: crate::settings::PagerLocalSnapshot {
                    multiline_mode: false,
                    yolo_mode: false,
                    ..crate::settings::PagerLocalSnapshot::default()
                },
            };
            let result = cmd.run(&mut ctx, "auto");
            match result {
                CommandResult::Action(Action::SetTheme(name)) => {
                    assert_eq!(name, "auto");
                }
                other => panic!("expected Action::SetTheme(\"auto\"), got {other:?}"),
            }
        });
    }

    /// Aliases normalise to canonical `display_name` before dispatch.
    #[test]
    fn run_alias_normalises_to_canonical() {
        with_test_env(|| {
            let cmd = ThemeCommand;
            let models = crate::acp::model_state::ModelState::default();
            let bundle = crate::app::bundle::BundleState::default();
            let mut ctx = CommandExecCtx {
                models: &models,
                session_id: None,
                bundle_state: &bundle,
                screen_mode: crate::app::ScreenMode::Inline,
                billing_surface_visible: true,
                usage_command_visible: true,
                pager_state: crate::settings::PagerLocalSnapshot {
                    multiline_mode: false,
                    yolo_mode: false,
                    ..crate::settings::PagerLocalSnapshot::default()
                },
            };
            // "dark" is an alias for GrokNight.
            let result = cmd.run(&mut ctx, "dark");
            match result {
                CommandResult::Action(Action::SetTheme(name)) => {
                    assert_eq!(name, "groknight", "alias must normalise to canonical");
                }
                other => panic!("expected Action::SetTheme(\"groknight\"), got {other:?}"),
            }
        });
    }

    // -- preview_arg ----------------------------------------------------------

    #[test]
    fn preview_auto_applies_resolved_theme() {
        with_test_env(|| {
            system_appearance::set_mock(Some(system_appearance::SystemAppearance::Light));
            let cmd = ThemeCommand;
            cmd.preview_arg("auto");
            // The default auto config maps Light to GrokDay
            assert_eq!(Theme::current_kind(), ThemeKind::GrokDay);
        });
    }

    #[test]
    fn preview_explicit_theme_applies_directly() {
        with_test_env(|| {
            theme_cache::set(ThemeKind::GrokNight);
            let cmd = ThemeCommand;
            cmd.preview_arg("grokday");
            assert_eq!(Theme::current_kind(), ThemeKind::GrokDay);
        });
    }

    #[test]
    fn preview_and_cancel_custom_theme() {
        with_test_env(|| {
            theme_cache::set(ThemeKind::GrokNight);
            let cmd = ThemeCommand;
            cmd.preview_arg("opencode");
            assert_eq!(Theme::current_kind(), ThemeKind::Custom);
            assert_eq!(theme_cache::current_name(), "opencode");
            cmd.cancel_preview("groknight");
            assert_eq!(Theme::current_kind(), ThemeKind::GrokNight);
        });
    }

    #[test]
    fn preview_unknown_theme_is_no_op() {
        with_test_env(|| {
            theme_cache::set(ThemeKind::GrokNight);
            let cmd = ThemeCommand;
            cmd.preview_arg("nonexistent-theme");
            assert_eq!(
                Theme::current_kind(),
                ThemeKind::GrokNight,
                "unknown theme name must NOT change Theme::current_kind",
            );
        });
    }

    // -- cancel_preview -------------------------------------------------------

    #[test]
    fn cancel_preview_restores_previous_kind() {
        with_test_env(|| {
            theme_cache::set(ThemeKind::GrokNight);
            let cmd = ThemeCommand;
            // Simulate user navigating into a different theme during preview.
            cmd.preview_arg("grokday");
            assert_eq!(Theme::current_kind(), ThemeKind::GrokDay);

            // Then Escape (or arg picker dismissal): restore.
            cmd.cancel_preview("groknight");
            assert_eq!(
                Theme::current_kind(),
                ThemeKind::GrokNight,
                "cancel_preview must restore the previous canonical",
            );
        });
    }

    #[test]
    fn cancel_preview_unknown_theme_is_no_op() {
        with_test_env(|| {
            theme_cache::set(ThemeKind::GrokDay);
            let cmd = ThemeCommand;
            cmd.cancel_preview("nonexistent-theme");
            assert_eq!(
                Theme::current_kind(),
                ThemeKind::GrokDay,
                "unknown previous must NOT change Theme::current_kind",
            );
        });
    }

    // -- error handling -------------------------------------------------------

    #[test]
    fn run_unknown_lists_auto_in_available() {
        with_test_env(|| {
            let cmd = ThemeCommand;
            let models = crate::acp::model_state::ModelState::default();
            let bundle = crate::app::bundle::BundleState::default();
            let mut ctx = CommandExecCtx {
                models: &models,
                session_id: None,
                bundle_state: &bundle,
                screen_mode: crate::app::ScreenMode::Inline,
                billing_surface_visible: true,
                usage_command_visible: true,
                pager_state: crate::settings::PagerLocalSnapshot {
                    multiline_mode: false,
                    yolo_mode: false,
                    ..crate::settings::PagerLocalSnapshot::default()
                },
            };
            let result = cmd.run(&mut ctx, "nonexistent");
            if let CommandResult::Error(msg) = result {
                assert!(msg.contains("auto"), "error should list auto: {msg}");
            } else {
                panic!("expected Error, got: {result:?}");
            }
        });
    }

    #[test]
    fn run_truecolor_theme_dispatches_set_theme_action() {
        with_test_env(|| {
            let cmd = ThemeCommand;
            let models = crate::acp::model_state::ModelState::default();
            let bundle = crate::app::bundle::BundleState::default();
            let mut ctx = CommandExecCtx {
                models: &models,
                session_id: None,
                bundle_state: &bundle,
                screen_mode: crate::app::ScreenMode::Inline,
                billing_surface_visible: true,
                usage_command_visible: true,
                pager_state: crate::settings::PagerLocalSnapshot {
                    multiline_mode: false,
                    yolo_mode: false,
                    ..crate::settings::PagerLocalSnapshot::default()
                },
            };
            let result = cmd.run(&mut ctx, "tokyonight");
            match result {
                CommandResult::Action(Action::SetTheme(name)) => {
                    assert_eq!(
                        name, "tokyonight",
                        "truecolor themes must be accepted; clamping happens \
                         downstream in `Theme::apply_kind`",
                    );
                }
                other => panic!("expected Action::SetTheme(\"tokyonight\"), got {other:?}"),
            }
        });
    }
}
