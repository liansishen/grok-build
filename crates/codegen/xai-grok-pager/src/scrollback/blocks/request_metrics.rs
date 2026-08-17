//! Compact per-model-request latency and token metrics.

use ratatui::text::{Line, Span};

use crate::appearance::AppearanceConfig;
use crate::render::wrapping::word_wrap_lines;
use crate::scrollback::block::BlockContent;
use crate::scrollback::types::{AccentStyle, BlockContext, BlockLine, BlockOutput};
use crate::theme::Theme;
use xai_grok_shell::extensions::notification::ResponseUsage;

const TPS_MIN_GEN_MS: u64 = 100;

#[derive(Debug, Clone)]
pub struct RequestMetricsBlock {
    time_to_first_token_ms: Option<u64>,
    duration_ms: u64,
    input_tokens: u64,
    cached_tokens: u64,
    output_tokens: u64,
}

impl RequestMetricsBlock {
    pub fn new(
        time_to_first_token_ms: Option<u64>,
        duration_ms: u64,
        usage: &ResponseUsage,
    ) -> Self {
        Self {
            time_to_first_token_ms,
            duration_ms,
            input_tokens: usage
                .input_tokens
                .saturating_add(usage.cache_read_input_tokens)
                .saturating_add(usage.cache_creation_input_tokens),
            cached_tokens: usage.cache_read_input_tokens,
            output_tokens: usage.output_tokens,
        }
    }

    pub fn message(&self) -> String {
        let (timing, tokens) = self.message_parts();
        xai_grok_i18n::t_fmt(
            "scrollback.request_metrics",
            &[("timing", &timing), ("tokens", &tokens)],
        )
    }

    fn message_parts(&self) -> (String, String) {
        let first = self
            .time_to_first_token_ms
            .map(format_seconds)
            .unwrap_or_else(|| "-".to_string());
        let speed = self
            .tokens_per_second()
            .map(|value| format!("{value:.1}"))
            .unwrap_or_else(|| "-".to_string());
        let duration = format_seconds(self.duration_ms);
        let total = format_count(self.input_tokens.saturating_add(self.output_tokens));
        let input = format_count(self.input_tokens);
        let output = format_count(self.output_tokens);
        let breakdown = if self.cached_tokens > 0 {
            xai_grok_i18n::t_fmt(
                "scrollback.request_metrics_breakdown_cached",
                &[
                    ("input", &input),
                    ("cached", &format_count(self.cached_tokens)),
                    ("output", &output),
                ],
            )
        } else {
            xai_grok_i18n::t_fmt(
                "scrollback.request_metrics_breakdown",
                &[("input", &input), ("output", &output)],
            )
        };
        let timing = xai_grok_i18n::t_fmt(
            "scrollback.request_metrics_timing",
            &[
                ("first", &first),
                ("speed", &speed),
                ("duration", &duration),
            ],
        );
        let tokens = xai_grok_i18n::t_fmt(
            "scrollback.request_metrics_tokens",
            &[("total", &total), ("breakdown", &breakdown)],
        );
        (timing, tokens)
    }

    fn tokens_per_second(&self) -> Option<f64> {
        if self.output_tokens == 0 {
            return None;
        }
        let first = self.time_to_first_token_ms?;
        let generation_ms = self
            .duration_ms
            .saturating_sub(first)
            .max(TPS_MIN_GEN_MS);
        Some(self.output_tokens as f64 * 1000.0 / generation_ms as f64)
    }
}

impl BlockContent for RequestMetricsBlock {
    fn output(&self, ctx: &BlockContext) -> BlockOutput {
        let style = Theme::current().muted();
        let (timing, tokens) = self.message_parts();
        let full = xai_grok_i18n::t_fmt(
            "scrollback.request_metrics",
            &[("timing", &timing), ("tokens", &tokens)],
        );
        let lines = if unicode_width::UnicodeWidthStr::width(full.as_str()) <= ctx.width as usize {
            vec![BlockLine::separator(Line::from(Span::styled(full, style)))]
        } else if unicode_width::UnicodeWidthStr::width(timing.as_str()) <= ctx.width as usize
            && unicode_width::UnicodeWidthStr::width(tokens.as_str()) <= ctx.width as usize
        {
            vec![
                BlockLine::separator(Line::from(Span::styled(timing, style))),
                BlockLine::separator(Line::from(Span::styled(tokens, style))),
            ]
        } else {
            word_wrap_lines(
                [
                    Line::from(Span::styled(timing, style)),
                    Line::from(Span::styled(tokens, style)),
                ],
                ctx.width as usize,
            )
            .into_iter()
            .map(BlockLine::separator)
            .collect()
        };
        BlockOutput { lines }
    }

    fn accent(&self, _ctx: &BlockContext) -> Option<AccentStyle> {
        None
    }

    fn has_vpad_for(&self, _appearance: &AppearanceConfig) -> bool {
        false
    }

    fn is_foldable(&self) -> bool {
        false
    }

    fn is_selectable(&self) -> bool {
        false
    }

    fn is_groupable(&self) -> bool {
        true
    }
}

fn format_seconds(ms: u64) -> String {
    format!("{:.1}s", ms as f64 / 1000.0)
}

fn format_count(value: u64) -> String {
    let digits = value.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
    let first_group = digits.len() % 3;
    if first_group > 0 {
        formatted.push_str(&digits[..first_group]);
    }
    for chunk in digits.as_bytes()[first_group..].chunks(3) {
        if !formatted.is_empty() {
            formatted.push(',');
        }
        formatted.push_str(std::str::from_utf8(chunk).expect("decimal digits are valid UTF-8"));
    }
    formatted
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RestoreLocale(xai_grok_i18n::Locale);

    impl Drop for RestoreLocale {
        fn drop(&mut self) {
            xai_grok_i18n::set_locale(self.0);
        }
    }

    fn use_zh_cn() -> RestoreLocale {
        let previous = xai_grok_i18n::current_locale();
        xai_grok_i18n::set_locale(xai_grok_i18n::Locale::ZhCn);
        RestoreLocale(previous)
    }

    #[test]
    #[serial_test::serial(GROK_UI_LOCALE)]
    fn message_includes_cached_tokens_and_generation_tps() {
        let _locale = use_zh_cn();
        let block = RequestMetricsBlock::new(
            Some(900),
            11_900,
            &ResponseUsage {
                input_tokens: 440,
                output_tokens: 480,
                cache_read_input_tokens: 2_650,
                ..Default::default()
            },
        );
        assert_eq!(
            block.message(),
            "首0.9s|速43.6|耗11.9s|词3,570(入3,090|缓2,650|出480)"
        );
    }

    #[test]
    #[serial_test::serial(GROK_UI_LOCALE)]
    fn message_omits_zero_cache_and_marks_unavailable_speed() {
        let _locale = use_zh_cn();
        let block = RequestMetricsBlock::new(
            None,
            9_300,
            &ResponseUsage {
                input_tokens: 1_820,
                output_tokens: 0,
                ..Default::default()
            },
        );
        assert_eq!(
            block.message(),
            "首-|速-|耗9.3s|词1,820(入1,820|出0)"
        );
    }
}
