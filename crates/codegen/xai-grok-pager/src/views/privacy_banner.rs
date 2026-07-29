//! Coding-data sharing upsell banner (Figma "Data Sharing Upsell",
//! node 8698:3690). Shared by the welcome tip slot and the agent-view
//! banner slot; visibility is gated by `AppView::privacy_banner_should_show`.

use crate::theme::Theme;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use unicode_width::UnicodeWidthStr;

/// Shares its row with the buttons.
fn privacy_banner_title() -> &'static str {
    xai_grok_i18n::t("privacy.banner.title")
}

fn privacy_banner_desc() -> &'static str {
    xai_grok_i18n::t("privacy.banner.description")
}

pub(crate) const PRIVACY_BANNER_TERMS_URL: &str = "https://x.ai/legal/terms-of-service";
pub(crate) const PRIVACY_BANNER_POLICY_URL: &str = "https://x.ai/legal/privacy-policy";

/// `(text, url_when_link)`.
type LegalSegment = (&'static str, Option<&'static str>);

/// Widest first; the first that fits *whole* wins. A clipped line would
/// leave hit rects over unreadable link text, and every variant keeps both
/// links so neither document becomes unreachable.
fn privacy_banner_legal_variants() -> [Vec<LegalSegment>; 3] {
    let terms = xai_grok_i18n::t("privacy.banner.legal.terms");
    let policy = xai_grok_i18n::t("privacy.banner.legal.policy");
    [
        vec![
            (xai_grok_i18n::t("privacy.banner.legal.read_prefix"), None),
            (terms, Some(PRIVACY_BANNER_TERMS_URL)),
            (xai_grok_i18n::t("privacy.banner.legal.and"), None),
            (policy, Some(PRIVACY_BANNER_POLICY_URL)),
            (xai_grok_i18n::t("privacy.banner.legal.period"), None),
        ],
        vec![
            (terms, Some(PRIVACY_BANNER_TERMS_URL)),
            (xai_grok_i18n::t("privacy.banner.legal.and"), None),
            (policy, Some(PRIVACY_BANNER_POLICY_URL)),
        ],
        vec![
            (terms, Some(PRIVACY_BANNER_TERMS_URL)),
            (xai_grok_i18n::t("privacy.banner.legal.compact_joiner"), None),
            (
                xai_grok_i18n::t("privacy.banner.legal.policy_short"),
                Some(PRIVACY_BANNER_POLICY_URL),
            ),
        ],
    ]
}

fn opt_out_label() -> &'static str {
    xai_grok_i18n::t("privacy.banner.opt_out")
}

fn opt_in_label() -> &'static str {
    xai_grok_i18n::t("privacy.banner.opt_in")
}

/// Title + legal.
const CHROME_ROWS: u16 = 2;

pub(crate) const MIN_HEIGHT: u16 = CHROME_ROWS + 1;

/// Caps banner growth on narrow terminals; overflow is elided with `…` so
/// the disclosure never looks complete when it isn't.
const MAX_BODY_ROWS: usize = 4;

/// Past this, the body abandons the button column for the full slot width:
/// a shorter banner beats a tidy right edge.
const PREFERRED_BODY_ROWS: usize = 3;

pub(crate) struct PrivacyBannerRects {
    pub opt_in: Rect,
    pub opt_out: Rect,
    pub terms: Rect,
    pub policy: Rect,
}

impl PrivacyBannerRects {
    fn none() -> Self {
        Self {
            opt_in: Rect::default(),
            opt_out: Rect::default(),
            terms: Rect::default(),
            policy: Rect::default(),
        }
    }
}

fn display_width(text: &str) -> u16 {
    UnicodeWidthStr::width(text).min(u16::MAX as usize) as u16
}

fn button_block_width() -> u16 {
    display_width(opt_out_label()) + 1 + display_width(opt_in_label())
}

fn legal_width(variant: &[LegalSegment]) -> u16 {
    variant.iter().map(|(text, _)| display_width(text)).sum()
}

/// Buttons render whole or not at all, and never at the cost of the title:
/// a clipped/overflowing opt-in label must not leave a click target in the
/// blank margin (a stray click there would silently opt the user in).
fn buttons_fit(area_width: u16) -> bool {
    area_width >= display_width(privacy_banner_title()) + 1 + button_block_width()
}

fn title_width(area_width: u16) -> u16 {
    if buttons_fit(area_width) {
        area_width - button_block_width() - 1
    } else {
        area_width
    }
}

fn wrap_to(width: usize) -> Vec<std::borrow::Cow<'static, str>> {
    if width == 0 {
        return vec![];
    }
    let opts = textwrap::Options::new(width).wrap_algorithm(textwrap::WrapAlgorithm::FirstFit);
    textwrap::wrap(privacy_banner_desc(), opts)
}

fn body_lines(area_width: u16) -> Vec<std::borrow::Cow<'static, str>> {
    let column = wrap_to(title_width(area_width) as usize);
    let mut lines = if column.len() <= PREFERRED_BODY_ROWS {
        column
    } else {
        let full = wrap_to(area_width as usize);
        if full.len() < column.len() {
            full
        } else {
            column
        }
    };
    if lines.len() > MAX_BODY_ROWS {
        lines.truncate(MAX_BODY_ROWS);
        if let Some(last) = lines.last_mut() {
            let mut s = last.trim_end().to_string();
            while UnicodeWidthStr::width(s.as_str()) + 1 > area_width as usize {
                s.pop();
            }
            s.push('\u{2026}');
            *last = std::borrow::Cow::Owned(s);
        }
    }
    lines
}

/// Rows needed at `width` — the body wraps, so both slot owners must size
/// from this rather than a constant.
pub(crate) fn height(width: u16) -> u16 {
    CHROME_ROWS + (body_lines(width).len() as u16).max(1)
}

/// Needs `area.height >= MIN_HEIGHT`; give it [`height`] rows for the full
/// body.
pub(crate) fn render(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    mouse_pos: Option<(u16, u16)>,
) -> PrivacyBannerRects {
    if area.height < MIN_HEIGHT || area.width == 0 {
        return PrivacyBannerRects::none();
    }

    let hovered = |r: Rect| {
        mouse_pos.is_some_and(|(mx, my)| r.contains(ratatui::layout::Position::new(mx, my)))
    };

    // Figma node 8698:3806.
    buf.set_stringn(
        area.x,
        area.y,
        privacy_banner_title(),
        title_width(area.width) as usize,
        Style::default().fg(theme.text_primary),
    );

    let body_style = Style::default().fg(theme.gray_bright);
    let body_rows = area.height - CHROME_ROWS;
    let body: Vec<Line> = body_lines(area.width)
        .into_iter()
        .take(body_rows as usize)
        .map(|l| Line::styled(l.into_owned(), body_style))
        .collect();
    Paragraph::new(body).render(
        Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: body_rows,
        },
        buf,
    );

    // Last row, so it gets the full width — no buttons to dodge.
    let gray = Style::default().fg(theme.gray);
    let legal_y = area.y + area.height - 1;
    let mut terms_rect = Rect::default();
    let mut policy_rect = Rect::default();
    let legal_variants = privacy_banner_legal_variants();
    if let Some(variant) = legal_variants
        .iter()
        .find(|v| legal_width(v) <= area.width)
    {
        let mut x = area.x;
        let mut spans = Vec::with_capacity(variant.len());
        for (text, url) in variant {
            let w = display_width(text);
            let style = match url {
                None => gray,
                Some(url) => {
                    let rect = Rect {
                        x,
                        y: legal_y,
                        width: w,
                        height: 1,
                    };
                    if *url == PRIVACY_BANNER_TERMS_URL {
                        terms_rect = rect;
                    } else {
                        policy_rect = rect;
                    }
                    let fg = if hovered(rect) {
                        theme.gray_bright
                    } else {
                        theme.gray
                    };
                    Style::default().fg(fg).add_modifier(Modifier::UNDERLINED)
                }
            };
            spans.push(Span::styled(*text, style));
            x += w;
        }
        Paragraph::new(Line::from(spans)).render(
            Rect {
                x: area.x,
                y: legal_y,
                width: x - area.x,
                height: 1,
            },
            buf,
        );
    }

    if !buttons_fit(area.width) {
        return PrivacyBannerRects {
            opt_in: Rect::default(),
            opt_out: Rect::default(),
            terms: terms_rect,
            policy: policy_rect,
        };
    }
    let opt_out_rect = Rect {
        x: area.x + area.width - button_block_width(),
        y: area.y,
        width: display_width(opt_out_label()),
        height: 1,
    };
    let opt_in_rect = Rect {
        x: opt_out_rect.x + opt_out_rect.width + 1,
        y: area.y,
        width: display_width(opt_in_label()),
        height: 1,
    };
    let opt_out_style = if hovered(opt_out_rect) {
        Style::default().fg(theme.text_primary).bg(theme.bg_hover)
    } else {
        Style::default().fg(theme.gray_bright)
    };
    let opt_in_style = if hovered(opt_in_rect) {
        Style::default().fg(theme.link_fg).bg(theme.bg_hover)
    } else {
        Style::default().fg(theme.text_primary)
    };
    buf.set_stringn(
        opt_out_rect.x,
        opt_out_rect.y,
        opt_out_label(),
        opt_out_rect.width as usize,
        opt_out_style,
    );
    buf.set_stringn(
        opt_in_rect.x,
        opt_in_rect.y,
        opt_in_label(),
        opt_in_rect.width as usize,
        opt_in_style,
    );
    PrivacyBannerRects {
        opt_in: opt_in_rect,
        opt_out: opt_out_rect,
        terms: terms_rect,
        policy: policy_rect,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Render at `width` into a buffer sized by [`height`], returning the
    /// rows (trailing blanks trimmed) and the hit rects.
    fn draw(width: u16) -> (Vec<String>, PrivacyBannerRects) {
        let h = height(width);
        let area = Rect::new(0, 0, width, h);
        let mut buf = Buffer::empty(area);
        let rects = render(area, &mut buf, &Theme::current(), None);
        let rows = (0..h)
            .map(|y| {
                (0..width)
                    .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect();
        (rows, rects)
    }

    fn rows(width: u16) -> Vec<String> {
        draw(width).0
    }

    /// The text a legal variant reassembles to.
    fn legal_text(variant: &[LegalSegment]) -> String {
        variant.iter().map(|(text, _)| *text).collect()
    }

    /// The buffer text under `rect` on its row.
    fn text_at(rows: &[String], rect: Rect) -> String {
        let row = &rows[rect.y as usize];
        let mut out = String::new();
        let mut column = 0;
        let start = rect.x as usize;
        let end = start + rect.width as usize;
        for ch in row.chars() {
            let width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            let next = column + width;
            if column >= start && next <= end {
                out.push(ch);
            }
            column = next;
            if column >= end {
                break;
            }
        }
        out
    }

    /// Slot owners reserve [`height`] rows, so the last one it promises must
    /// be the legal line — not a body row pushed off the end.
    #[test]
    fn height_reserves_every_row_the_banner_paints() {
        for width in [200, 117, 110, 100, 80, 72, 60, 45, 40, 36, 30, 24, 18] {
            let rows = rows(width);
            assert_eq!(rows.len(), height(width) as usize);
            assert!(
                rows[0].starts_with(privacy_banner_title()),
                "width {width}: title must never be clipped, got {:?}",
                rows[0]
            );
            let legal = rows.last().expect("legal row");
            assert!(
                privacy_banner_legal_variants()
                    .iter()
                    .any(|v| legal_text(v) == *legal),
                "width {width}: legal line must survive whole, got {legal:?}"
            );
            assert!(
                rows[1..rows.len() - 1].iter().all(|r| !r.is_empty()),
                "width {width}: body rows must not be blank: {rows:?}"
            );
        }
    }

    /// The row cap's elision is a narrow-terminal fallback, not the norm.
    #[test]
    fn body_copy_is_complete_at_common_widths() {
        for width in [200, 117, 100, 80, 60] {
            let body = rows(width)[1..].join(" ");
            let flattened: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(
                flattened.contains(privacy_banner_desc()),
                "width {width}: body copy was truncated: {flattened:?}"
            );
        }
    }

    #[test]
    fn buttons_drop_whole_when_the_row_is_too_narrow() {
        let width = display_width(privacy_banner_title()) + button_block_width(); // one short
        let h = height(width);
        let mut buf = Buffer::empty(Rect::new(0, 0, width, h));
        let rects = render(Rect::new(0, 0, width, h), &mut buf, &Theme::current(), None);
        assert_eq!(rects.opt_in, Rect::default());
        assert_eq!(rects.opt_out, Rect::default());
        assert_ne!(rects.terms, Rect::default(), "terms link still clickable");
        assert_ne!(rects.policy, Rect::default(), "policy link still clickable");

        let rects = {
            let width = width + 1;
            let h = height(width);
            let mut buf = Buffer::empty(Rect::new(0, 0, width, h));
            render(Rect::new(0, 0, width, h), &mut buf, &Theme::current(), None)
        };
        assert_eq!(rects.opt_out.width, display_width(opt_out_label()));
        assert_eq!(rects.opt_in.width, display_width(opt_in_label()));
    }

    #[test]
    #[serial_test::serial(GROK_UI_LOCALE)]
    fn zh_cn_layout_uses_terminal_width_for_buttons_and_links() {
        struct RestoreLocale(xai_grok_i18n::Locale);
        impl Drop for RestoreLocale {
            fn drop(&mut self) {
                xai_grok_i18n::set_locale(self.0);
            }
        }
        let _restore = RestoreLocale(xai_grok_i18n::current_locale());
        xai_grok_i18n::set_locale(xai_grok_i18n::Locale::ZhCn);

        let width = 80;
        let (rows, rects) = draw(width);
        assert!(rows[0].starts_with(privacy_banner_title()));
        assert_eq!(rects.opt_out.width, display_width(opt_out_label()));
        assert_eq!(rects.opt_in.width, display_width(opt_in_label()));
        assert_eq!(text_at(&rows, rects.terms), "服务条款");
        assert!(matches!(text_at(&rows, rects.policy).as_str(), "隐私政策" | "隐私"));
    }

    #[test]
    fn slot_below_min_height_arms_no_hit_rects() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 100, MIN_HEIGHT));
        let rects = render(
            Rect::new(0, 0, 100, MIN_HEIGHT - 1),
            &mut buf,
            &Theme::current(),
            None,
        );
        assert_eq!(rects.opt_in, Rect::default());
        assert_eq!(rects.opt_out, Rect::default());
        assert_eq!(rects.terms, Rect::default());
        assert_eq!(rects.policy, Rect::default());
    }

    /// The two links open different documents, so an off-by-one rect sends
    /// the user to the wrong page.
    #[test]
    fn each_legal_link_hits_its_own_words() {
        for width in [200, 117, 80, 60, 40, 30, 24, 18] {
            let (rows, rects) = draw(width);
            assert_eq!(
                text_at(&rows, rects.terms),
                xai_grok_i18n::t("privacy.banner.legal.terms"),
                "width {width}: terms rect is off its word: {rows:?}"
            );
            let policy = text_at(&rows, rects.policy);
            assert!(
                policy == xai_grok_i18n::t("privacy.banner.legal.policy")
                    || policy == xai_grok_i18n::t("privacy.banner.legal.policy_short"),
                "width {width}: policy rect is off its word, got {policy:?}"
            );
            assert!(
                rects.terms.right() <= rects.policy.x,
                "width {width}: link rects must not overlap"
            );
        }
    }
}
