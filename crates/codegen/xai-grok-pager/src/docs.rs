//! In-app how-to documentation data (embedded markdown).
//!
//! The canonical English docs live in two static arrays (`USER_GUIDE`,
//! `REFERENCE_DOCS`). `DocEntry` adds active-locale metadata and selected
//! translated content for the TUI doc picker.

/// A compile-time document entry. All fields are `&'static str`.
#[derive(Debug)]
pub struct Doc {
    pub filename: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub content: &'static str,
}

/// Owned variant for the TUI doc picker (backward compat).
#[derive(Debug, Clone)]
pub struct DocEntry {
    pub title: String,
    pub description: String,
    /// Embedded markdown content.
    pub content: &'static str,
}

impl From<&Doc> for DocEntry {
    fn from(d: &Doc) -> Self {
        let (title, description) = localized_doc_meta(d);
        Self {
            title: title.into(),
            description: description.into(),
            content: doc_content_for_locale(d, xai_grok_i18n::current_locale()),
        }
    }
}

/// Content for the requested UI locale.
///
/// Keep `Doc::content` English so model-facing lookups and extracted files
/// retain their existing behavior; switch every guide that has a zh-CN
/// counterpart for the TUI picker.
fn doc_content_for_locale(d: &Doc, locale: xai_grok_i18n::Locale) -> &'static str {
    if !matches!(locale, xai_grok_i18n::Locale::ZhCn) {
        return d.content;
    }
    localized_guide_content(d.filename).unwrap_or(d.content)
}

fn localized_guide_content(filename: &str) -> Option<&'static str> {
    Some(match filename {
        "01-getting-started.md" => include_str!("../docs/user-guide/zh-CN/01-getting-started.md"),
        "02-authentication.md" => include_str!("../docs/user-guide/zh-CN/02-authentication.md"),
        "03-keyboard-shortcuts.md" => {
            include_str!("../docs/user-guide/zh-CN/03-keyboard-shortcuts.md")
        }
        "04-slash-commands.md" => include_str!("../docs/user-guide/zh-CN/04-slash-commands.md"),
        "05-configuration.md" => include_str!("../docs/user-guide/zh-CN/05-configuration.md"),
        "06-theming.md" => include_str!("../docs/user-guide/zh-CN/06-theming.md"),
        "07-mcp-servers.md" => include_str!("../docs/user-guide/zh-CN/07-mcp-servers.md"),
        "08-skills.md" => include_str!("../docs/user-guide/zh-CN/08-skills.md"),
        "09-plugins.md" => include_str!("../docs/user-guide/zh-CN/09-plugins.md"),
        "10-hooks.md" => include_str!("../docs/user-guide/zh-CN/10-hooks.md"),
        "11-custom-models.md" => include_str!("../docs/user-guide/zh-CN/11-custom-models.md"),
        "12-project-rules.md" => include_str!("../docs/user-guide/zh-CN/12-project-rules.md"),
        "13-memory.md" => include_str!("../docs/user-guide/zh-CN/13-memory.md"),
        "14-headless-mode.md" => include_str!("../docs/user-guide/zh-CN/14-headless-mode.md"),
        "15-agent-mode.md" => include_str!("../docs/user-guide/zh-CN/15-agent-mode.md"),
        "16-subagents.md" => include_str!("../docs/user-guide/zh-CN/16-subagents.md"),
        "17-sessions.md" => include_str!("../docs/user-guide/zh-CN/17-sessions.md"),
        "18-sandbox.md" => include_str!("../docs/user-guide/zh-CN/18-sandbox.md"),
        "19-plan-mode.md" => include_str!("../docs/user-guide/zh-CN/19-plan-mode.md"),
        "20-background-tasks.md" => include_str!("../docs/user-guide/zh-CN/20-background-tasks.md"),
        "21-terminal-support.md" => include_str!("../docs/user-guide/zh-CN/21-terminal-support.md"),
        "22-permissions-and-safety.md" => {
            include_str!("../docs/user-guide/zh-CN/22-permissions-and-safety.md")
        }
        "23-dashboard.md" => include_str!("../docs/user-guide/zh-CN/23-dashboard.md"),
        "24-monitoring-usage.md" => include_str!("../docs/user-guide/zh-CN/24-monitoring-usage.md"),
        "25-status-line.md" => include_str!("../docs/user-guide/zh-CN/25-status-line.md"),
        "26-config-reference.md" => include_str!("../docs/user-guide/zh-CN/26-config-reference.md"),
        "27-grok-clone.md" => include_str!("../docs/user-guide/zh-CN/27-grok-clone.md"),
        _ => return None,
    })
}

fn localized_doc_meta(d: &Doc) -> (&'static str, &'static str) {
    let key = match d.filename {
        "hooks-and-plugins.md" => "docs.reference.hooks_plugins",
        "custom-hooks.md" => "docs.reference.custom_hooks",
        "23-dashboard.md" => "docs.23",
        "24-monitoring-usage.md" => "docs.24",
        // Other user-guide files use a stable two-digit numeric prefix.
        filename => {
            let num = filename.get(..2).unwrap_or("");
            xai_grok_i18n::intern_key(&format!("docs.{num}"))
        }
    };
    let title_key = xai_grok_i18n::intern_key(&format!("{key}.title"));
    let desc_key = xai_grok_i18n::intern_key(&format!("{key}.desc"));
    (
        xai_grok_i18n::t_or(title_key, d.title),
        xai_grok_i18n::t_or(desc_key, d.description),
    )
}

// ── Static doc tables ────────────────────────────────────────────────────────

macro_rules! guide {
    ($file:literal, $title:literal, $desc:literal) => {
        Doc {
            filename: $file,
            title: $title,
            description: $desc,
            content: include_str!(concat!("../docs/user-guide/", $file)),
        }
    };
}

pub static USER_GUIDE: &[Doc] = &[
    guide!(
        "01-getting-started.md",
        "Getting Started",
        "Installation, first launch, and basic interaction"
    ),
    guide!(
        "02-authentication.md",
        "Authentication",
        "Browser login, API keys, OIDC, external auth providers"
    ),
    guide!(
        "03-keyboard-shortcuts.md",
        "Keyboard Shortcuts",
        "Complete reference for all TUI key bindings"
    ),
    guide!(
        "04-slash-commands.md",
        "Slash Commands",
        "All / commands, including goals, research, and workflow management"
    ),
    guide!(
        "05-configuration.md",
        "Configuration",
        "config.toml, pager.toml, environment variables, file locations"
    ),
    guide!(
        "06-theming.md",
        "Theming and Appearance",
        "Themes, color support, pager.toml customization"
    ),
    guide!(
        "07-mcp-servers.md",
        "MCP Servers",
        "Setting up external tool integrations via MCP"
    ),
    guide!(
        "08-skills.md",
        "Skills",
        "Creating and using reusable prompt packages"
    ),
    guide!(
        "09-plugins.md",
        "Plugins and Marketplace",
        "Installing, managing, and creating plugin packages"
    ),
    guide!(
        "10-hooks.md",
        "Hooks",
        "Project lifecycle scripts for pre/post tool-use events"
    ),
    guide!(
        "11-custom-models.md",
        "Custom Models",
        "BYOK, Ollama, OpenAI-compatible endpoints"
    ),
    guide!(
        "12-project-rules.md",
        "Project Rules (AGENTS.md)",
        "Per-directory instructions and precedence rules"
    ),
    guide!(
        "13-memory.md",
        "Memory",
        "Cross-session knowledge persistence and search"
    ),
    guide!(
        "14-headless-mode.md",
        "Headless Mode and Scripting",
        "Non-interactive CLI for automation and CI/CD"
    ),
    guide!(
        "15-agent-mode.md",
        "Agent Mode and IDE Integration",
        "ACP stdio transport, WebSocket relay, SDK integration"
    ),
    guide!(
        "16-subagents.md",
        "Subagents and Personas",
        "Spawning parallel child agents with specialized roles"
    ),
    guide!(
        "17-sessions.md",
        "Session Management",
        "Save, load, resume, rewind, and compact sessions"
    ),
    guide!(
        "18-sandbox.md",
        "Sandbox Mode",
        "OS-level filesystem and network isolation"
    ),
    guide!(
        "19-plan-mode.md",
        "Plan Mode",
        "Structured planning with approval dialogs"
    ),
    guide!(
        "20-background-tasks.md",
        "Background Tasks and Monitoring",
        "Background commands, /loop, monitor, scheduler"
    ),
    guide!(
        "21-terminal-support.md",
        "Terminal Support and Troubleshooting",
        "tmux, Byobu, Zellij, SSH, truecolor, clipboard, and diagnostics"
    ),
    guide!(
        "22-permissions-and-safety.md",
        "Permissions and Safety",
        "Modes, authorization order, allow/ask/deny rules, matching, and hooks"
    ),
    guide!(
        "23-dashboard.md",
        "Agent Dashboard",
        "Live multi-session roster: peek, dispatch, pin, stop, and search"
    ),
    guide!(
        "24-monitoring-usage.md",
        "Monitoring Usage (External OpenTelemetry)",
        "Export usage metrics to a customer OpenTelemetry collector"
    ),
    guide!(
        "25-status-line.md",
        "Status Line",
        "A bottom row of live session context, or the output of your own script"
    ),
    guide!(
        "26-config-reference.md",
        "Configuration Reference",
        "Field list for config.toml, managed_config.toml, and requirements.toml"
    ),
    // Direct include_str! so gazelle can put this file in compile_data.
    // `guide!` hides the path inside concat!($file) and gazelle cannot see it.
    Doc {
        filename: "27-grok-clone.md",
        title: "grok clone",
        description: "Depth-1 Grove clone, --full-history, and safe deepen/switch commands",
        content: include_str!("../docs/user-guide/27-grok-clone.md"),
    },
];

/// Non-user-guide reference docs. Separate from USER_GUIDE because they
/// live under `docs/` (not `docs/user-guide/`), are not extracted to disk,
/// and do not follow the NN-*.md managed naming pattern. Bundled via
/// `include_str!` so they are available at runtime without a docs path.
pub(crate) static REFERENCE_DOCS: &[Doc] = &[
    Doc {
        filename: "hooks-and-plugins.md",
        title: "Hooks & Plugins Guide",
        description: "Using hooks, plugins, and marketplace",
        content: include_str!("../docs/hooks-and-plugins.md"),
    },
    Doc {
        filename: "custom-hooks.md",
        title: "Creating Custom Hooks",
        description: "Writing your own hooks and matchers",
        content: include_str!("../docs/custom-hooks.md"),
    },
];

// ── Public API ───────────────────────────────────────────────────────────────

/// Find a doc by title (case-insensitive). Returns the static entry.
pub fn find_doc(title: &str) -> Option<&'static Doc> {
    USER_GUIDE
        .iter()
        .chain(REFERENCE_DOCS.iter())
        .find(|d| d.title.eq_ignore_ascii_case(title))
}

/// All doc titles, zero allocation.
pub fn all_titles() -> impl Iterator<Item = &'static str> {
    USER_GUIDE
        .iter()
        .chain(REFERENCE_DOCS.iter())
        .map(|d| d.title)
}

/// Returns the content of a how-to document by exact title match (case-insensitive).
pub fn get_howto_doc(title: &str) -> Option<&'static str> {
    find_doc(title).map(|d| d.content)
}

/// Returns a list of available how-to titles for the model to choose from.
pub fn list_howto_titles() -> Vec<String> {
    all_titles().map(String::from).collect()
}

/// Returns all docs as owned `DocEntry` values for the TUI doc picker.
///
/// Titles and descriptions follow the active UI catalog. Guides with a
/// zh-CN counterpart use the translated markdown in the TUI picker.
pub fn default_howto_entries() -> Vec<DocEntry> {
    USER_GUIDE
        .iter()
        .chain(REFERENCE_DOCS.iter())
        .map(DocEntry::from)
        .collect()
}

/// Find a doc by its English or localized title and return localized TUI data.
pub(crate) fn find_localized_doc(title: &str) -> Option<DocEntry> {
    USER_GUIDE
        .iter()
        .chain(REFERENCE_DOCS.iter())
        .find_map(|doc| {
            let localized = DocEntry::from(doc);
            if doc.title.eq_ignore_ascii_case(title) || localized.title.eq_ignore_ascii_case(title)
            {
                Some(localized)
            } else {
                None
            }
        })
}

/// Extract user-guide docs to `<grok_home>/docs/user-guide/`.
///
/// Called from the pager binary startup so the model can read them from disk.
pub fn extract_user_guide_docs(grok_home: &std::path::Path) {
    let docs_dir = grok_home.join("docs").join("user-guide");
    if let Err(e) = std::fs::create_dir_all(&docs_dir) {
        tracing::warn!(error = %e, "Failed to create user-guide docs directory");
        return;
    }
    for doc in USER_GUIDE {
        if let Err(e) = std::fs::write(docs_dir.join(doc.filename), doc.content) {
            tracing::debug!(error = %e, filename = doc.filename, "Failed to extract user-guide doc");
        }
    }
    // Clean up stale managed docs (files removed from USER_GUIDE since last run).
    // Only remove files matching the managed naming pattern (NN-*.md).
    if let Ok(entries) = std::fs::read_dir(&docs_dir) {
        let valid: std::collections::HashSet<&str> =
            USER_GUIDE.iter().map(|d| d.filename).collect();
        for dir_entry in entries.flatten() {
            if let Some(name) = dir_entry.file_name().to_str() {
                let is_managed = name.len() > 3
                    && name.as_bytes()[0].is_ascii_digit()
                    && name.as_bytes()[1].is_ascii_digit()
                    && name.as_bytes()[2] == b'-'
                    && name.ends_with(".md");
                if is_managed
                    && !valid.contains(name)
                    && let Err(e) = std::fs::remove_file(dir_entry.path())
                {
                    tracing::debug!(error = %e, filename = name, "Failed to remove stale user-guide doc");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_guide_entries_are_valid() {
        for doc in USER_GUIDE {
            assert!(!doc.content.is_empty(), "Doc {} is empty", doc.filename);
            assert!(
                !doc.title.is_empty(),
                "Doc {} has empty title",
                doc.filename
            );
            assert!(
                !doc.description.is_empty(),
                "Doc {} has empty description",
                doc.filename
            );
            assert!(
                doc.content.starts_with('#'),
                "Doc {} should start with a markdown header",
                doc.filename
            );
        }
    }

    #[test]
    fn user_guide_entries_have_no_duplicates() {
        let mut seen = std::collections::HashSet::new();
        for doc in USER_GUIDE {
            assert!(
                seen.insert(doc.filename),
                "Duplicate doc in list: {}",
                doc.filename
            );
        }
    }

    #[test]
    #[serial_test::serial(GROK_UI_LOCALE)]
    fn default_howto_entries_includes_all_user_guide_docs() {
        struct RestoreLocale(xai_grok_i18n::Locale);
        impl Drop for RestoreLocale {
            fn drop(&mut self) {
                xai_grok_i18n::set_locale(self.0);
            }
        }
        let _restore = RestoreLocale(xai_grok_i18n::current_locale());
        xai_grok_i18n::set_locale(xai_grok_i18n::Locale::En);
        let entries = default_howto_entries();
        assert_eq!(entries.len(), USER_GUIDE.len() + REFERENCE_DOCS.len());
        for (i, doc) in USER_GUIDE.iter().enumerate() {
            let (title, _) = localized_doc_meta(doc);
            assert_eq!(entries[i].title, title, "Entry {} title mismatch", i);
            assert_eq!(
                entries[i].content,
                doc_content_for_locale(doc, xai_grok_i18n::current_locale()),
                "Entry {} content mismatch",
                i
            );
        }
    }

    #[test]
    fn localized_content_covers_every_zh_cn_guide() {
        assert_eq!(
            USER_GUIDE.len(),
            27,
            "update the locale map when adding a guide"
        );
        for doc in USER_GUIDE {
            let localized = localized_guide_content(doc.filename)
                .unwrap_or_else(|| panic!("missing zh-CN guide for {}", doc.filename));
            assert!(
                !localized.trim().is_empty(),
                "{} has empty zh-CN content",
                doc.filename
            );
            assert!(
                localized.starts_with('#') || localized.starts_with('<'),
                "{} has no markdown heading",
                doc.filename
            );
        }
    }

    #[test]
    fn localized_guides_preserve_section_and_code_block_shape() {
        fn shape(markdown: &str) -> (usize, usize) {
            let mut in_code = false;
            let mut headings = 0;
            let mut fences = 0;
            for line in markdown.lines() {
                if line.starts_with("```") {
                    in_code = !in_code;
                    fences += 1;
                } else if !in_code && line.starts_with('#') {
                    headings += 1;
                }
            }
            (headings, fences)
        }

        for doc in USER_GUIDE {
            let localized = localized_guide_content(doc.filename).expect("covered guide");
            let (english_headings, english_fences) = shape(doc.content);
            let (localized_headings, localized_fences) = shape(localized);
            assert!(localized_headings > 0, "{} has no sections", doc.filename);
            assert!(
                localized_headings + 2 >= english_headings,
                "{} lost sections during translation",
                doc.filename
            );
            assert_eq!(
                localized_fences % 2,
                0,
                "{} has unbalanced code blocks",
                doc.filename
            );
            assert!(
                localized_fences >= english_fences,
                "{} lost code blocks during translation",
                doc.filename
            );
            assert_eq!(
                english_fences % 2,
                0,
                "{} has unbalanced source code blocks",
                doc.filename
            );
        }
    }

    #[test]
    #[serial_test::serial(GROK_UI_LOCALE)]
    fn default_howto_entries_localizes_all_guide_content() {
        struct RestoreLocale(xai_grok_i18n::Locale);
        impl Drop for RestoreLocale {
            fn drop(&mut self) {
                xai_grok_i18n::set_locale(self.0);
            }
        }

        let _restore = RestoreLocale(xai_grok_i18n::current_locale());
        xai_grok_i18n::set_locale(xai_grok_i18n::Locale::ZhCn);
        let entries = default_howto_entries();
        for (index, doc) in USER_GUIDE.iter().enumerate() {
            assert_eq!(
                entries[index].content,
                localized_guide_content(doc.filename).unwrap()
            );
        }
    }

    #[test]
    fn find_doc_is_case_insensitive() {
        let doc = find_doc("getting started").expect("should find Getting Started");
        assert_eq!(doc.title, "Getting Started");
        assert!(find_doc("nonexistent guide").is_none());
    }

    #[test]
    fn all_titles_covers_both_tables() {
        let titles: Vec<_> = all_titles().collect();
        assert_eq!(titles.len(), USER_GUIDE.len() + REFERENCE_DOCS.len());
    }

    #[test]
    fn get_howto_doc_delegates_to_find_doc() {
        assert!(get_howto_doc("Getting Started").is_some());
        assert!(get_howto_doc("Hooks & Plugins Guide").is_some());
        assert!(get_howto_doc("no such doc").is_none());
    }

    #[test]
    fn list_howto_titles_returns_all() {
        let titles = list_howto_titles();
        assert_eq!(titles.len(), USER_GUIDE.len() + REFERENCE_DOCS.len());
    }

    #[test]
    fn extract_writes_docs_and_cleans_stale() {
        let tmp = tempfile::tempdir().unwrap();
        let docs_dir = tmp.path().join("docs").join("user-guide");

        std::fs::create_dir_all(&docs_dir).unwrap();
        std::fs::write(docs_dir.join("99-removed.md"), "stale").unwrap();
        std::fs::write(docs_dir.join("notes.md"), "user notes").unwrap();

        extract_user_guide_docs(tmp.path());

        for doc in USER_GUIDE {
            let path = docs_dir.join(doc.filename);
            assert!(path.exists(), "Expected doc {} to exist", doc.filename);
            let got = std::fs::read_to_string(&path).unwrap();
            assert_eq!(got, doc.content, "Content mismatch for {}", doc.filename);
        }
        assert!(
            !docs_dir.join("99-removed.md").exists(),
            "Stale doc should be cleaned up"
        );
        assert!(
            docs_dir.join("notes.md").exists(),
            "User file should not be deleted"
        );
    }
}
