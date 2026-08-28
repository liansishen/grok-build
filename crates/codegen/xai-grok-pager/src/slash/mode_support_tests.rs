use pretty_assertions::assert_eq;

use super::{ModeSupport, Remedy};
use crate::app::ScreenMode;

const FULLSCREEN_ONLY: ModeSupport = ModeSupport::FullscreenOnly(Remedy::SwitchMode {
    why: "minimal is single-session",
});
const MINIMAL_ONLY: ModeSupport =
    ModeSupport::MinimalOnly(Remedy::UseInstead("press → on the block"));

#[test]
fn inline_counts_as_fullscreen() {
    for mode in [ScreenMode::Fullscreen, ScreenMode::Inline] {
        assert!(ModeSupport::Both.supports(mode));
        assert!(FULLSCREEN_ONLY.supports(mode));
        assert!(!MINIMAL_ONLY.supports(mode));
    }

    assert!(ModeSupport::Both.supports(ScreenMode::Minimal));
    assert!(!FULLSCREEN_ONLY.supports(ScreenMode::Minimal));
    assert!(MINIMAL_ONLY.supports(ScreenMode::Minimal));
}

#[test]
fn supported_modes_have_no_refusal() {
    assert_eq!(
        ModeSupport::Both.refusal("theme", ScreenMode::Minimal),
        None
    );
    assert_eq!(
        FULLSCREEN_ONLY.refusal("theme", ScreenMode::Inline),
        None,
        "inline is not minimal, so a fullscreen-only command runs"
    );
    assert_eq!(MINIMAL_ONLY.refusal("expand", ScreenMode::Minimal), None);
}

#[test]
fn switch_mode_refusal_names_the_current_mode_and_the_way_out() {
    assert_eq!(
        FULLSCREEN_ONLY.refusal("theme", ScreenMode::Minimal),
        Some(xai_grok_i18n::t_fmt(
            "slash.mode_support.switch_mode",
            &[
                ("token", "theme"),
                (
                    "current",
                    xai_grok_i18n::t("slash.screen_mode.display_minimal"),
                ),
                ("why", "minimal is single-session"),
                ("switch", "/fullscreen"),
            ],
        ))
    );
    assert_eq!(
        ModeSupport::MinimalOnly(Remedy::SwitchMode {
            why: "the full TUI prints nothing to re-print"
        })
        .refusal("expand", ScreenMode::Fullscreen),
        Some(xai_grok_i18n::t_fmt(
            "slash.mode_support.switch_mode",
            &[
                ("token", "expand"),
                (
                    "current",
                    xai_grok_i18n::t("slash.screen_mode.display_fullscreen"),
                ),
                ("why", "the full TUI prints nothing to re-print"),
                ("switch", "/minimal"),
            ],
        ))
    );
}

#[test]
fn use_instead_refusal_names_the_substitute_not_a_relaunch() {
    let refusal = MINIMAL_ONLY
        .refusal("expand", ScreenMode::Fullscreen)
        .expect("minimal-only command is refused in fullscreen");
    assert_eq!(
        refusal,
        xai_grok_i18n::t_fmt(
            "slash.mode_support.use_instead",
            &[
                ("token", "expand"),
                (
                    "current",
                    xai_grok_i18n::t("slash.screen_mode.display_fullscreen"),
                ),
                ("instead", "press → on the block"),
            ],
        )
    );
    assert!(
        !refusal.contains("/minimal"),
        "suggesting a relaunch contradicts the substitute: {refusal:?}"
    );
}

#[test]
fn already_in_mode_refusal_is_a_plain_statement() {
    assert_eq!(
        ModeSupport::FullscreenOnly(Remedy::AlreadyInMode).refusal("minimal", ScreenMode::Minimal),
        Some(xai_grok_i18n::t_fmt(
            "slash.mode_support.already_in_mode",
            &[(
                "current",
                xai_grok_i18n::t("slash.screen_mode.display_minimal"),
            )],
        ))
    );
    assert_eq!(
        ModeSupport::MinimalOnly(Remedy::AlreadyInMode).refusal("fullscreen", ScreenMode::Inline),
        Some(xai_grok_i18n::t_fmt(
            "slash.mode_support.already_in_mode",
            &[(
                "current",
                xai_grok_i18n::t("slash.screen_mode.display_fullscreen"),
            )],
        ))
    );
}

/// Pinned on the composed sentence, not the variant, so a remedy that reads wrong to a user lands in the diff rather than only in the code.
#[test]
fn mode_specific_builtin_refusals_are_pinned() {
    let commands = crate::slash::commands::builtin_commands();
    let mut actual: Vec<(&str, String)> = commands
        .iter()
        .filter_map(|command| {
            let refusal = [ScreenMode::Minimal, ScreenMode::Fullscreen]
                .into_iter()
                .find_map(|mode| command.mode_support().refusal(command.name(), mode))?;
            Some((command.name(), refusal))
        })
        .collect();
    actual.sort_unstable();

    let minimal = xai_grok_i18n::t("slash.screen_mode.display_minimal");
    let fullscreen = xai_grok_i18n::t("slash.screen_mode.display_fullscreen");
    let switch = |token: &str, current: &str, why: &str, switch: &str| {
        xai_grok_i18n::t_fmt(
            "slash.mode_support.switch_mode",
            &[
                ("token", token),
                ("current", current),
                ("why", why),
                ("switch", switch),
            ],
        )
    };
    let use_instead = |token: &str, current: &str, instead: &str| {
        xai_grok_i18n::t_fmt(
            "slash.mode_support.use_instead",
            &[
                ("token", token),
                ("current", current),
                ("instead", instead),
            ],
        )
    };
    let already = |current: &str| {
        xai_grok_i18n::t_fmt("slash.mode_support.already_in_mode", &[("current", current)])
    };

    assert_eq!(
        actual,
        vec![
            (
                "dashboard",
                switch(
                    "dashboard",
                    minimal,
                    "minimal is single-session",
                    "/fullscreen",
                )
            ),
            (
                "edit-prompt",
                switch(
                    "edit-prompt",
                    fullscreen,
                    "the full TUI has no external-editor path — Ctrl+G is the tasks pane there",
                    "/minimal",
                )
            ),
            (
                "expand",
                use_instead(
                    "expand",
                    fullscreen,
                    "press Tab to focus the scrollback, then → on the block",
                )
            ),
            (
                "find",
                switch(
                    "find",
                    minimal,
                    "minimal has no scrollback pane — use your terminal's own search",
                    "/fullscreen",
                )
            ),
            ("fullscreen", already(fullscreen)),
            (
                "jump",
                switch(
                    "jump",
                    minimal,
                    "minimal scrolls with your terminal's native scrollback",
                    "/fullscreen",
                )
            ),
            ("minimal", already(minimal)),
            (
                "theme",
                switch(
                    "theme",
                    minimal,
                    "minimal renders with your terminal's own palette",
                    "/fullscreen",
                )
            ),
            (
                "timeline",
                switch(
                    "timeline",
                    minimal,
                    "the timeline rail needs the interactive scrollback pane",
                    "/fullscreen",
                )
            ),
            (
                "tutorial",
                switch(
                    "tutorial",
                    minimal,
                    "the tutorial overlay needs fullscreen",
                    "/fullscreen",
                )
            ),
            (
                "workflows",
                switch(
                    "workflows",
                    minimal,
                    "the workflow run pane needs fullscreen",
                    "/fullscreen",
                )
            ),
        ]
    );
}
