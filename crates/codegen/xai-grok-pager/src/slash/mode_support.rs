use crate::app::ScreenMode;

/// What to tell a user who typed a command the current mode cannot run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Remedy {
    SwitchMode {
        /// Sentence fragment, parenthesized in the refusal: `"minimal is single-session"`.
        why: &'static str,
    },
    /// Imperative clause naming what to do in this mode instead.
    /// Name arrows, `Tab`, or `Ctrl+<letter>`.
    /// `Ctrl+G` is the external editor in minimal and the tasks pane everywhere else, and a bare letter resolves only under vim mode (off by default).
    UseInstead(&'static str),
    AlreadyInMode,
}

/// Which render modes a slash command functions in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeSupport {
    Both,
    FullscreenOnly(Remedy),
    MinimalOnly(Remedy),
}

impl ModeSupport {
    pub(crate) fn supports(self, mode: ScreenMode) -> bool {
        match self {
            Self::Both => true,
            Self::FullscreenOnly(_) => !mode.is_minimal(),
            Self::MinimalOnly(_) => mode.is_minimal(),
        }
    }

    pub(crate) fn refusal(self, token: &str, mode: ScreenMode) -> Option<String> {
        if self.supports(mode) {
            return None;
        }
        let (remedy, current, switch) = match self {
            Self::Both => return None,
            Self::FullscreenOnly(remedy) => (remedy, "minimal", "/fullscreen"),
            Self::MinimalOnly(remedy) => (remedy, "fullscreen", "/minimal"),
        };
        let current_label = match current {
            "minimal" => xai_grok_i18n::t("slash.screen_mode.display_minimal"),
            _ => xai_grok_i18n::t("slash.screen_mode.display_fullscreen"),
        };
        Some(match remedy {
            Remedy::SwitchMode { why } => xai_grok_i18n::t_fmt(
                "slash.mode_support.switch_mode",
                &[
                    ("token", token),
                    ("current", current_label),
                    ("why", why),
                    ("switch", switch),
                ],
            ),
            Remedy::UseInstead(instead) => xai_grok_i18n::t_fmt(
                "slash.mode_support.use_instead",
                &[
                    ("token", token),
                    ("current", current_label),
                    ("instead", instead),
                ],
            ),
            Remedy::AlreadyInMode => xai_grok_i18n::t_fmt(
                "slash.mode_support.already_in_mode",
                &[("current", current_label)],
            ),
        })
    }
}

#[cfg(test)]
#[path = "mode_support_tests.rs"]
mod tests;
