//! Minimal-mode chrome follows `GROK_LANGUAGE` end-to-end through the real pager binary.
//!
//! `--minimal` paints its idle status, its prompt info row, and the committed welcome card from
//! the `minimal.*` catalog entries, so a `GROK_LANGUAGE=zh-CN` run must put Chinese copy on the
//! emulated screen and must not leave the English literals it replaced anywhere the user can
//! reach (viewport plus native scrollback). Running the same scenario under `GROK_LANGUAGE=en`
//! proves the difference comes from the locale rather than from an always-Chinese surface.

use std::time::Duration;

use anyhow::{Context, Result, bail};

use crate::{ContentController, PtyHarness, pager_binary};

const DEFAULT_ROWS: u16 = 50;
const DEFAULT_COLS: u16 = 120;
/// Cold start on a loaded worker covers the sandbox handshake, the session's first request, and
/// minimal's inline-viewport cursor probe.
const READY_TIMEOUT: Duration = Duration::from_secs(45);

/// One locale's minimal chrome: what it paints, and the other locale's copy for the same surfaces.
struct LocaleChrome {
    /// Value handed to the child as `GROK_LANGUAGE`.
    language: &'static str,
    /// Strings that must reach the visible viewport: the idle status line and the prompt info row.
    /// These extract from the emulated screen byte-for-byte, so they are matched literally.
    visible: &'static [&'static str],
    /// Strings from the welcome card, which minimal commits into native scrollback through the
    /// block-commit path. See [`spaced`] for how those rows extract from the emulated screen.
    committed: &'static [&'static str],
    /// Literals the same surfaces print under the other locale; none may survive here.
    replaced: &'static [&'static str],
    /// Whether this locale's copy is CJK, the cheapest locale fingerprint that needs no key.
    expects_cjk: bool,
}

/// Catalog values from `xai-grok-i18n/locales/zh-CN.toml`: `minimal.live.idle_hint` (a cold
/// `--minimal` start omits the switch-back segment), `minimal.live.transcript_hint`,
/// `minimal.welcome.help_hint`, and `minimal.welcome.model`.
const ZH_CN: LocaleChrome = LocaleChrome {
    language: "zh-CN",
    visible: &["极简模式 · /help", "ctrl+o 查看对话记录"],
    committed: &["/help 查看可用命令", "模型 · ", "版本 v"],
    replaced: &[
        "minimal · /help",
        "minimal · /fullscreen to go back · /help",
        "/help for commands",
        "Model · ",
        "ctrl+o transcript",
    ],
    expects_cjk: true,
};

/// The English catalog (`locales/en.toml`) values for those same four surfaces.
const EN: LocaleChrome = LocaleChrome {
    language: "en",
    visible: &["minimal · /help", "ctrl+o transcript"],
    committed: &["/help for commands", "Model · "],
    replaced: &[
        "极简模式",
        "/help 查看可用命令",
        "模型 · ",
        "版本",
        "ctrl+o 查看对话记录",
    ],
    expects_cjk: false,
};

/// Any CJK ideograph; the minimal chrome in this scenario is the only user-visible text.
fn contains_cjk(text: &str) -> bool {
    text.chars().any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c))
}

/// `text` with all whitespace removed.
///
/// The committed welcome card prints one glyph per cell, and the wide glyph's trailing cell comes
/// back out of the emulated screen as a space — this run's card row reads `/help 查 看 可 用 命 令`
/// while the live status row reads `极简模式 · /help` unseparated. Dropping whitespace from both
/// sides keeps the required characters and their order exact for rows that extract that way, and
/// makes the absence checks stricter than a literal compare.
fn spaced(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Wait until `text` shows up in the committed card region, tolerant of cell spacing.
fn wait_for_committed_text(harness: &mut PtyHarness, text: &str, timeout: Duration) -> Result<()> {
    let result = harness.wait_until(&format!("committed chrome {text:?}"), timeout, |h| {
        spaced(&h.full_text()).contains(&spaced(text))
    });
    result.map_err(|error| anyhow::anyhow!("{error}\nfull contents:\n{}", harness.full_text()))
}

/// Run the real pager in `--minimal` under `language` and assert the chrome it paints comes from
/// that locale's catalog while the other locale's literals are gone.
pub async fn assert_minimal_ui_language(language: &str) -> Result<()> {
    let expected = match language {
        "zh-CN" => &ZH_CN,
        "en" => &EN,
        other => bail!("unsupported GROK_LANGUAGE {other:?}: add its chrome expectations first"),
    };

    let content = ContentController::start()
        .await
        .context("start ContentController")?;
    // `ContentController::start` presets `allow_access`, and minimal has no dock/queue pane, so the
    // sandbox default settings already reach the interactive prompt.

    let binary = pager_binary().context("resolve pager binary")?;
    let mut harness = PtyHarness::spawn_with_content_env_in_dir(
        &binary,
        DEFAULT_ROWS,
        DEFAULT_COLS,
        &content,
        &["--minimal"],
        &[("GROK_LANGUAGE", expected.language)],
        Some(content.home()),
    )
    .context("spawn pager")?;
    // Minimal's inline viewport probes the cursor position at startup: unanswered, `--minimal`
    // silently downgrades to full-height inline and the minimal chrome never paints.
    harness.set_respond_to_queries(true);

    let locale = expected.language;
    for text in expected.visible {
        harness
            .wait_for_text(text, READY_TIMEOUT)
            .with_context(|| {
                format!("GROK_LANGUAGE={locale}: minimal chrome never showed {text:?}")
            })?;
    }
    for text in expected.committed {
        wait_for_committed_text(&mut harness, text, READY_TIMEOUT).with_context(|| {
            format!("GROK_LANGUAGE={locale}: minimal chrome never showed {text:?}")
        })?;
    }

    let full = harness.full_text();
    let squashed = spaced(&full);
    let mut failures: Vec<String> = Vec::new();
    for text in expected.replaced {
        if squashed.contains(&spaced(text)) {
            failures.push(format!(
                "the replaced literal {text:?} still reaches the user"
            ));
        }
    }
    let cjk = contains_cjk(&full);
    if expected.expects_cjk && !cjk {
        failures.push("no CJK character anywhere on screen or in scrollback".to_owned());
    }
    if !expected.expects_cjk && cjk {
        failures.push("CJK text leaked into the English locale's screen".to_owned());
    }
    if !failures.is_empty() {
        bail!(
            "minimal chrome under GROK_LANGUAGE={locale} does not match the {locale} catalog:\n  \
             {}\nfull contents:\n{full}",
            failures.join("\n  ")
        );
    }
    if harness.contains_text("panicked") {
        bail!("pager panicked\nfull contents:\n{full}");
    }

    harness.quit().context("clean quit")?;
    Ok(())
}
