use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use ratatui::style::Color;
use serde::Deserialize;

use super::Theme;

const OPENCODE_DARK: &str = include_str!("../../assets/themes/opencode-dark.toml");
const OPENCODE_LIGHT: &str = include_str!("../../assets/themes/opencode-light.toml");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeAppearance {
    Dark,
    Light,
}

impl ThemeAppearance {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "dark" | "night" => Some(Self::Dark),
            "light" | "day" => Some(Self::Light),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoadedTheme {
    pub canonical: String,
    pub display_name: String,
    pub description: String,
    pub appearance: ThemeAppearance,
    pub theme: Theme,
}

#[derive(Debug, Deserialize)]
struct ThemeFile {
    name: String,
    #[serde(default)]
    display_name: Option<String>,
    appearance: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    colors: BTreeMap<String, String>,
}

pub fn bundled() -> Vec<LoadedTheme> {
    [
        ("opencode-dark.toml", OPENCODE_DARK),
        ("opencode-light.toml", OPENCODE_LIGHT),
    ]
    .into_iter()
    .filter_map(|(source, content)| match parse_document(content, source) {
        Ok(theme) => Some(theme),
        Err(error) => {
            tracing::warn!(source, %error, "bundled theme ignored");
            None
        }
    })
    .collect()
}

pub fn load_directory(directory: &Path) -> Vec<LoadedTheme> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut paths: Vec<_> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && matches!(
                    path.extension().and_then(|ext| ext.to_str()),
                    Some("toml") | Some("json")
                )
        })
        .collect();
    paths.sort();

    paths
        .into_iter()
        .filter_map(|path| {
            let source = path.to_string_lossy().into_owned();
            let content = match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(error) => {
                    tracing::warn!(path = %path.display(), error = %error, "theme file ignored");
                    return None;
                }
            };
            match parse_document(&content, &source) {
                Ok(theme) => Some(theme),
                Err(error) => {
                    tracing::warn!(path = %path.display(), error = %error, "theme file ignored");
                    None
                }
            }
        })
        .collect()
}

fn parse_document(content: &str, source: &str) -> Result<LoadedTheme, String> {
    let file: ThemeFile = if source.ends_with(".json") {
        serde_json::from_str(content).map_err(|error| error.to_string())?
    } else {
        toml::from_str(content).map_err(|error| error.to_string())?
    };
    parse_theme(file)
}

fn parse_theme(file: ThemeFile) -> Result<LoadedTheme, String> {
    let canonical = canonicalize_name(&file.name)
        .ok_or_else(|| "name must contain only letters, digits, '-' or '_'".to_string())?;
    if matches!(
        canonical.as_str(),
        "auto"
            | "system"
            | "grok-night"
            | "dark"
            | "tokyo-night"
            | "tokyo"
            | "grok-day"
            | "light"
            | "day"
            | "rosepine"
            | "rose-pine"
            | "rose-pine-moon"
            | "oscura"
            | "terminal-default"
            | "transparent"
            | "native"
    ) || super::ThemeKind::ALL
        .iter()
        .any(|kind| kind.display_name() == canonical)
    {
        return Err(format!("name is reserved by a built-in theme: {canonical}"));
    }
    let appearance = ThemeAppearance::parse(&file.appearance)
        .ok_or_else(|| "appearance must be 'dark' or 'light'".to_string())?;
    let mut theme = match appearance {
        ThemeAppearance::Dark => Theme::groknight(),
        ThemeAppearance::Light => Theme::grokday(),
    };
    for (role, value) in &file.colors {
        let color =
            parse_color(value).ok_or_else(|| format!("invalid color for {role}: {value}"))?;
        set_color(&mut theme, role, color).ok_or_else(|| format!("unknown color role: {role}"))?;
    }

    Ok(LoadedTheme {
        display_name: file.display_name.unwrap_or_else(|| canonical.clone()),
        description: file.description.unwrap_or_default(),
        canonical,
        appearance,
        theme,
    })
}

pub fn canonicalize_name(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_lowercase().replace('_', "-");
    if value.is_empty()
        || value.len() > 64
        || !value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        || value.starts_with('-')
        || value.ends_with('-')
        || value.contains("--")
    {
        return None;
    }
    Some(value)
}

fn parse_color(value: &str) -> Option<Color> {
    let raw = value.trim();
    if raw.eq_ignore_ascii_case("reset") || raw.eq_ignore_ascii_case("default") {
        return Some(Color::Reset);
    }
    if let Some(index) = raw.strip_prefix("indexed:") {
        return index.parse::<u8>().ok().map(Color::Indexed);
    }
    let hex = raw.strip_prefix('#').unwrap_or(raw);
    if hex.len() == 6 {
        let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
        return Some(Color::Rgb(red, green, blue));
    }
    match raw.to_ascii_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "dark-gray" | "darkgrey" => Some(Color::DarkGray),
        "light-red" => Some(Color::LightRed),
        "light-green" => Some(Color::LightGreen),
        "light-yellow" => Some(Color::LightYellow),
        "light-blue" => Some(Color::LightBlue),
        "light-magenta" => Some(Color::LightMagenta),
        "light-cyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        _ => None,
    }
}

fn set_color(theme: &mut Theme, role: &str, color: Color) -> Option<()> {
    macro_rules! assign {
        ($($name:literal => $field:ident),+ $(,)?) => {{
            match role {
                $($name => {
                    theme.$field = color;
                },)+
                _ => return None,
            };
            Some(())
        }};
    }
    assign!(
        "bg_base" => bg_base,
        "bg_light" => bg_light,
        "bg_dark" => bg_dark,
        "bg_highlight" => bg_highlight,
        "bg_hover" => bg_hover,
        "bg_terminal" => bg_terminal,
        "accent_user" => accent_user,
        "accent_assistant" => accent_assistant,
        "accent_thinking" => accent_thinking,
        "accent_tool" => accent_tool,
        "accent_system" => accent_system,
        "accent_error" => accent_error,
        "accent_success" => accent_success,
        "accent_running" => accent_running,
        "accent_skill" => accent_skill,
        "text_primary" => text_primary,
        "text_secondary" => text_secondary,
        "gray_dim" => gray_dim,
        "gray" => gray,
        "gray_bright" => gray_bright,
        "command" => command,
        "path" => path,
        "running" => running,
        "warning" => warning,
        "fuzzy_accent" => fuzzy_accent,
        "accent_plan" => accent_plan,
        "accent_verify" => accent_verify,
        "accent_remember" => accent_remember,
        "selection_border" => selection_border,
        "hover_border" => hover_border,
        "prompt_border" => prompt_border,
        "prompt_border_active" => prompt_border_active,
        "accent_model" => accent_model,
        "scrollbar_bg" => scrollbar_bg,
        "scrollbar_fg" => scrollbar_fg,
        "diff_delete_bg" => diff_delete_bg,
        "diff_delete_fg" => diff_delete_fg,
        "diff_insert_bg" => diff_insert_bg,
        "diff_insert_fg" => diff_insert_fg,
        "diff_equal_fg" => diff_equal_fg,
        "diff_gutter_fg" => diff_gutter_fg,
        "bg_visual" => bg_visual,
        "paste_bg" => paste_bg,
        "paste_fg" => paste_fg,
        "paste_dim" => paste_dim,
        "md_heading_h1" => md_heading_h1,
        "md_heading_h2" => md_heading_h2,
        "md_heading_h3" => md_heading_h3,
        "md_heading_h4" => md_heading_h4,
        "md_heading_h5" => md_heading_h5,
        "md_heading_h6" => md_heading_h6,
        "md_code" => md_code,
        "md_task_checked" => md_task_checked,
        "md_task_unchecked" => md_task_unchecked,
        "md_muted" => md_muted,
        "md_code_bg" => md_code_bg,
        "md_text" => md_text,
        "link_fg" => link_fg,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_opencode_themes_have_distinct_appearance_metadata() {
        let themes = bundled();
        let dark = themes.iter().find(|theme| theme.canonical == "opencode");
        let light = themes
            .iter()
            .find(|theme| theme.canonical == "opencode-day");
        assert_eq!(
            dark.map(|theme| theme.appearance),
            Some(ThemeAppearance::Dark)
        );
        assert_eq!(
            light.map(|theme| theme.appearance),
            Some(ThemeAppearance::Light)
        );
        assert!(dark.is_some_and(|theme| theme.theme.bg_base == Color::Rgb(10, 10, 10)));
        assert!(light.is_some_and(|theme| theme.theme.bg_base == Color::Rgb(250, 250, 250)));
    }

    #[test]
    fn custom_names_cannot_shadow_builtin_names() {
        let document = r##"
name = "groknight"
appearance = "dark"
[colors]
bg_base = "#000000"
"##;
        let error =
            parse_document(document, "custom.toml").expect_err("reserved name must be rejected");
        assert!(error.contains("reserved"), "unexpected error: {error}");
    }

    #[test]
    fn load_directory_accepts_toml_and_json_and_skips_invalid_files() {
        let directory = tempfile::tempdir().expect("temp theme directory");
        std::fs::write(
            directory.path().join("dark.toml"),
            "name = \"test-dark\"\nappearance = \"dark\"\n",
        )
        .expect("write toml theme");
        std::fs::write(
            directory.path().join("light.json"),
            r##"{"name":"test-light","appearance":"light","colors":{"bg_base":"#ffffff"}}"##,
        )
        .expect("write json theme");
        std::fs::write(directory.path().join("broken.toml"), "name = \"broken\"")
            .expect("write invalid theme");
        std::fs::write(directory.path().join("ignored.txt"), "not a theme")
            .expect("write ignored file");

        let themes = load_directory(directory.path());
        assert_eq!(
            themes
                .iter()
                .map(|theme| theme.canonical.as_str())
                .collect::<Vec<_>>(),
            vec!["test-dark", "test-light"]
        );
    }
}
