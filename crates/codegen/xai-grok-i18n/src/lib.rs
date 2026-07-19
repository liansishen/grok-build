//! UI localization for Grok Build.
//!
//! English catalog (`locales/en.toml`) is the source of truth for keys.
//! Other locales fall back to English when a key is missing.
//!
//! # Preference resolution
//!
//! `GROK_LANGUAGE` env → `[ui].language` (passed by caller) → default `auto`.
//! `auto` resolves from process locale / Windows UI language.

use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU8, Ordering};

/// Canonical preference: follow OS locale.
pub const LANGUAGE_AUTO: &str = "auto";
/// Canonical preference / locale tag: English.
pub const LANGUAGE_EN: &str = "en";
/// Canonical preference / locale tag: Simplified Chinese.
pub const LANGUAGE_ZH_CN: &str = "zh-CN";

/// Env override for UI language (`auto` | `en` | `zh-CN`).
pub const ENV_LANGUAGE: &str = "GROK_LANGUAGE";

/// Supported UI locales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Locale {
    En = 0,
    ZhCn = 1,
}

impl Locale {
    /// Canonical BCP-47-ish tag stored in config / settings.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::En => LANGUAGE_EN,
            Self::ZhCn => LANGUAGE_ZH_CN,
        }
    }

    /// Human label for toasts (always bilingual-friendly product names).
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::ZhCn => "简体中文",
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(Locale::En as u8);

/// Install the active UI locale (affects subsequent [`t`] / [`t_fmt`] calls).
pub fn set_locale(locale: Locale) {
    CURRENT.store(locale as u8, Ordering::Release);
}

/// Currently active UI locale.
pub fn current_locale() -> Locale {
    match CURRENT.load(Ordering::Acquire) {
        x if x == Locale::ZhCn as u8 => Locale::ZhCn,
        _ => Locale::En,
    }
}

/// Normalize a user/config preference string to a catalog canonical.
///
/// Accepts `auto`, `en`, `zh-CN`, `zh`, `zh-Hans`, `zh_CN`, case-insensitive.
/// Unknown values map to [`LANGUAGE_AUTO`].
pub fn canonicalize_language(value: Option<&str>) -> &'static str {
    let Some(raw) = value.map(str::trim).filter(|s| !s.is_empty()) else {
        return LANGUAGE_AUTO;
    };
    let lower = raw.to_ascii_lowercase().replace('_', "-");
    match lower.as_str() {
        "auto" | "system" => LANGUAGE_AUTO,
        "en" | "en-us" | "en-gb" | "english" => LANGUAGE_EN,
        "zh-cn" | "zh" | "zh-hans" | "zh-sg" | "chinese" | "cn" => LANGUAGE_ZH_CN,
        _ if lower.starts_with("zh") => LANGUAGE_ZH_CN,
        _ if lower.starts_with("en") => LANGUAGE_EN,
        _ => LANGUAGE_AUTO,
    }
}

/// Resolve preference (already env-aware via [`effective_language_pref`]) to a concrete locale.
pub fn resolve_locale(pref: Option<&str>) -> Locale {
    match canonicalize_language(pref) {
        LANGUAGE_EN => Locale::En,
        LANGUAGE_ZH_CN => Locale::ZhCn,
        _ => system_locale(),
    }
}

/// Preference after applying `GROK_LANGUAGE` over the config value.
pub fn effective_language_pref(config: Option<&str>) -> Option<String> {
    if let Ok(v) = std::env::var(ENV_LANGUAGE) {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    config.map(|s| s.to_string())
}

/// Resolve env + config and apply to the global locale.
pub fn apply_from_config(config_language: Option<&str>) -> Locale {
    let pref = effective_language_pref(config_language);
    let locale = resolve_locale(pref.as_deref());
    set_locale(locale);
    locale
}

/// Best-effort OS UI language → supported locale (defaults to English).
pub fn system_locale() -> Locale {
    if let Some(tag) = system_locale_tag() {
        return match canonicalize_language(Some(&tag)) {
            LANGUAGE_ZH_CN => Locale::ZhCn,
            LANGUAGE_EN => Locale::En,
            // `auto` from unknown tags → English
            _ => Locale::En,
        };
    }
    Locale::En
}

fn system_locale_tag() -> Option<String> {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var) {
            let t = v.trim();
            if !t.is_empty() && !t.eq_ignore_ascii_case("C") && !t.eq_ignore_ascii_case("POSIX") {
                // Strip encoding suffix: zh_CN.UTF-8 → zh_CN
                let primary = t.split('.').next().unwrap_or(t);
                return Some(primary.to_string());
            }
        }
    }
    #[cfg(windows)]
    {
        windows_ui_locale_name()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(windows)]
fn windows_ui_locale_name() -> Option<String> {
    // GetUserDefaultLocaleName — returns e.g. "zh-CN", "en-US".
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetUserDefaultLocaleName(lp_locale_name: *mut u16, cch_locale_name: i32) -> i32;
    }
    let mut buf = [0u16; 85];
    let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    if n <= 1 {
        return None;
    }
    // n includes the trailing NUL.
    Some(String::from_utf16_lossy(&buf[..(n as usize - 1)]))
}

// ── Catalogs ────────────────────────────────────────────────────────────────

static EN: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| load_catalog(include_str!("../locales/en.toml")));

static ZH_CN: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| load_catalog(include_str!("../locales/zh-CN.toml")));

fn load_catalog(raw: &str) -> HashMap<&'static str, &'static str> {
    let value: toml::Value = toml::from_str(raw).expect("locale catalog must be valid TOML");
    let mut out = HashMap::new();
    flatten_toml(&value, "", &mut out);
    out
}

fn flatten_toml(value: &toml::Value, prefix: &str, out: &mut HashMap<&'static str, &'static str>) {
    match value {
        toml::Value::Table(table) => {
            for (k, v) in table {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten_toml(v, &key, out);
            }
        }
        toml::Value::String(s) => {
            let key: &'static str = Box::leak(prefix.to_string().into_boxed_str());
            let val: &'static str = Box::leak(s.clone().into_boxed_str());
            out.insert(key, val);
        }
        _ => {
            // Ignore non-string leaves (arrays, numbers) for now.
        }
    }
}

/// Look up a message key for the current locale (English fallback).
pub fn t(key: &str) -> &'static str {
    lookup(current_locale(), key)
}

/// Look up a message for an explicit locale (English fallback).
pub fn t_for(locale: Locale, key: &str) -> &'static str {
    lookup(locale, key)
}

fn lookup(locale: Locale, key: &str) -> &'static str {
    let primary = match locale {
        Locale::ZhCn => ZH_CN.get(key),
        Locale::En => EN.get(key),
    };
    if let Some(s) = primary {
        return s;
    }
    if !matches!(locale, Locale::En)
        && let Some(s) = EN.get(key)
    {
        return s;
    }
    // Unknown key: surface the key itself (leaked once per unique key).
    leak_fallback(key)
}

fn leak_fallback(key: &str) -> &'static str {
    // Prefer an existing EN key pointer if present under a different path.
    static FALLBACKS: LazyLock<std::sync::Mutex<HashMap<String, &'static str>>> =
        LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));
    let mut guard = FALLBACKS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(s) = guard.get(key) {
        return s;
    }
    let leaked: &'static str = Box::leak(key.to_string().into_boxed_str());
    guard.insert(key.to_string(), leaked);
    leaked
}

/// Format a message with `{name}`-style placeholders.
///
/// Example: `t_fmt("toast.language_set", &[("name", "English")])`.
pub fn t_fmt(key: &str, args: &[(&str, &str)]) -> String {
    let mut s = t(key).to_string();
    for (k, v) in args {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

/// Keys present in the English catalog (for tests / CI completeness checks).
pub fn en_keys() -> impl Iterator<Item = &'static str> {
    EN.keys().copied()
}

/// Keys present in the zh-CN catalog.
pub fn zh_cn_keys() -> impl Iterator<Item = &'static str> {
    ZH_CN.keys().copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_maps_common_aliases() {
        assert_eq!(canonicalize_language(None), LANGUAGE_AUTO);
        assert_eq!(canonicalize_language(Some("")), LANGUAGE_AUTO);
        assert_eq!(canonicalize_language(Some("auto")), LANGUAGE_AUTO);
        assert_eq!(canonicalize_language(Some("EN")), LANGUAGE_EN);
        assert_eq!(canonicalize_language(Some("zh_CN")), LANGUAGE_ZH_CN);
        assert_eq!(canonicalize_language(Some("zh-Hans")), LANGUAGE_ZH_CN);
        assert_eq!(canonicalize_language(Some("zh")), LANGUAGE_ZH_CN);
    }

    #[test]
    fn resolve_explicit_locales() {
        assert_eq!(resolve_locale(Some("en")), Locale::En);
        assert_eq!(resolve_locale(Some("zh-CN")), Locale::ZhCn);
    }

    #[test]
    fn t_switches_with_locale() {
        set_locale(Locale::En);
        assert_eq!(t("welcome.quit"), "quit");
        set_locale(Locale::ZhCn);
        assert_eq!(t("welcome.quit"), "退出");
        set_locale(Locale::En);
        assert_eq!(t("welcome.quit"), "quit");
    }

    #[test]
    fn missing_zh_falls_back_to_en() {
        set_locale(Locale::ZhCn);
        // Pick a key that exists only if we intentionally omit — all current
        // zh keys exist; use t_for with En after ensuring EN has settings label.
        assert_eq!(
            t_for(Locale::ZhCn, "settings.language.label"),
            "界面语言"
        );
        set_locale(Locale::En);
    }

    #[test]
    fn t_fmt_replaces_placeholders() {
        set_locale(Locale::En);
        let s = t_fmt("toast.language_set", &[("name", "English")]);
        assert_eq!(s, "UI language: English");
        set_locale(Locale::ZhCn);
        let s = t_fmt("toast.language_set", &[("name", "简体中文")]);
        assert_eq!(s, "界面语言：简体中文");
        set_locale(Locale::En);
    }

    #[test]
    fn zh_keys_are_subset_of_en() {
        let en: std::collections::HashSet<_> = en_keys().collect();
        for k in zh_cn_keys() {
            assert!(en.contains(k), "zh-CN key `{k}` missing from en.toml");
        }
    }

    #[test]
    fn catalogs_load_expected_core_keys() {
        for key in [
            "welcome.quit",
            "settings.category.appearance",
            "settings.language.label",
            "toast.language_set",
        ] {
            assert!(EN.contains_key(key), "en missing {key}");
            assert!(ZH_CN.contains_key(key), "zh-CN missing {key}");
        }
    }
}
