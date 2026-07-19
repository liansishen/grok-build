#!/usr/bin/env python3
"""Wire Phase 2–4 call sites to xai_grok_i18n."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def patch_settings_footers() -> None:
    p = ROOT / "crates/codegen/xai-grok-pager/src/views/settings_modal/render.rs"
    t = p.read_text(encoding="utf-8")
    pairs = [
        ('label: "\\u{2191}/\\u{2193}/j/k nav"', 'label: xai_grok_i18n::t("settings.modal.footer.nav_jk")'),
        ('label: "g/G top/btm"', 'label: xai_grok_i18n::t("settings.modal.footer.top_bottom")'),
        ('label: "Space toggle"', 'label: xai_grok_i18n::t("settings.modal.footer.space_toggle")'),
        ('label: "\\u{2192} expand"', 'label: xai_grok_i18n::t("settings.modal.footer.expand")'),
        ('label: "/ search"', 'label: xai_grok_i18n::t("settings.modal.footer.slash_search")'),
        ('label: "d reset"', 'label: xai_grok_i18n::t("settings.modal.footer.reset")'),
        ('label: "F2/Esc close"', 'label: xai_grok_i18n::t("settings.modal.footer.close")'),
        ('label: "type to filter"', 'label: xai_grok_i18n::t("settings.modal.footer.type_to_filter")'),
        ('label: "\\u{2191}/\\u{2193} nav"', 'label: xai_grok_i18n::t("settings.modal.footer.nav")'),
        ('label: "Backspace edit"', 'label: xai_grok_i18n::t("settings.modal.footer.backspace_edit")'),
        ('label: "Enter commit"', 'label: xai_grok_i18n::t("settings.modal.footer.enter_commit")'),
        ('label: "Esc clear"', 'label: xai_grok_i18n::t("settings.modal.footer.esc_clear")'),
        ('label: "y reset"', 'label: xai_grok_i18n::t("settings.modal.footer.y_reset")'),
        ('label: "n cancel"', 'label: xai_grok_i18n::t("settings.modal.footer.n_cancel")'),
        ('label: "Esc cancel"', 'label: xai_grok_i18n::t("settings.modal.footer.esc_cancel")'),
        ('label: "F2 cancel"', 'label: xai_grok_i18n::t("settings.modal.footer.f2_cancel")'),
        ('label: "type to edit"', 'label: xai_grok_i18n::t("settings.modal.footer.type_to_edit")'),
    ]
    # Unicode arrows may already be real chars in source
    pairs2 = [
        ('label: "↑/↓/j/k nav"', 'label: xai_grok_i18n::t("settings.modal.footer.nav_jk")'),
        ('label: "→ expand"', 'label: xai_grok_i18n::t("settings.modal.footer.expand")'),
        ('label: "↑/↓ nav"', 'label: xai_grok_i18n::t("settings.modal.footer.nav")'),
        ('label: "↑/↓ try"', 'label: xai_grok_i18n::t("settings.modal.footer.nav_try")'),
        ('label: "Esc revert"', 'label: xai_grok_i18n::t("settings.modal.footer.esc_revert")'),
    ]
    for a, b in pairs + pairs2:
        if a in t:
            t = t.replace(a, b)
            print("footer ok", a[:40])
    # enter_label variables
    t = t.replace(
        'Some((_, meta)) if matches!(meta.kind, SettingKind::Bool { .. }) => "Enter toggle",\n                _ => "Enter edit",',
        'Some((_, meta)) if matches!(meta.kind, SettingKind::Bool { .. }) => xai_grok_i18n::t("settings.modal.footer.enter_toggle"),\n                _ => xai_grok_i18n::t("settings.modal.footer.enter_edit"),',
    )
    t = t.replace(
        'let nav_label = if *sp {\n                "\\u{2191}/\\u{2193} try"\n            } else {\n                "\\u{2191}/\\u{2193} nav"\n            };\n            let esc_label = if *sp { "Esc revert" } else { "Esc cancel" };',
        'let nav_label = if *sp {\n                xai_grok_i18n::t("settings.modal.footer.nav_try")\n            } else {\n                xai_grok_i18n::t("settings.modal.footer.nav")\n            };\n            let esc_label = if *sp { xai_grok_i18n::t("settings.modal.footer.esc_revert") } else { xai_grok_i18n::t("settings.modal.footer.esc_cancel") };',
    )
    # Also real unicode form
    t = t.replace(
        'let nav_label = if *sp {\n                "↑/↓ try"\n            } else {\n                "↑/↓ nav"\n            };\n            let esc_label = if *sp { "Esc revert" } else { "Esc cancel" };',
        'let nav_label = if *sp {\n                xai_grok_i18n::t("settings.modal.footer.nav_try")\n            } else {\n                xai_grok_i18n::t("settings.modal.footer.nav")\n            };\n            let esc_label = if *sp { xai_grok_i18n::t("settings.modal.footer.esc_revert") } else { xai_grok_i18n::t("settings.modal.footer.esc_cancel") };',
    )
    p.write_text(t, encoding="utf-8")
    print("settings footers patched")


def patch_save_success_toast() -> None:
    p = ROOT / "crates/codegen/xai-grok-pager/src/app/dispatch/settings/ui.rs"
    t = p.read_text(encoding="utf-8")
    old = '''pub(in crate::app::dispatch) fn save_success_toast(label: &str, on: bool) -> String {
    let value = if on { "on" } else { "off" };
    format!("\\u{2713} {label}: {value}")
}'''
    new = '''pub(in crate::app::dispatch) fn save_success_toast(label: &str, on: bool) -> String {
    let value = if on {
        xai_grok_i18n::t("settings.modal.value_on")
    } else {
        xai_grok_i18n::t("settings.modal.value_off")
    };
    xai_grok_i18n::t_fmt(
        "toast.setting_changed",
        &[("label", label), ("value", value)],
    )
}'''
    if old in t:
        t = t.replace(old, new)
        print("save_success_toast patched")
    else:
        # try without unicode escape
        old2 = old.replace("\\u{2713}", "\u2713")
        if old2 in t:
            t = t.replace(old2, new)
            print("save_success_toast patched (unicode)")
        else:
            print("save_success_toast MISS")
    # already_at_default toast
    t = t.replace(
        'format!("{}: already at default", meta.label)',
        'xai_grok_i18n::t_fmt("toast.already_default", &[("label", meta.label_t())])',
    )
    t = t.replace(
        'app.show_toast("Mouse reporting on");',
        'app.show_toast(xai_grok_i18n::t("toast.mouse_reporting_on"));',
    )
    p.write_text(t, encoding="utf-8")

    # setters: pass localized labels into save_success_toast
    setters = ROOT / "crates/codegen/xai-grok-pager/src/app/dispatch/settings/setters.rs"
    st = setters.read_text(encoding="utf-8")
    label_map = {
        '"Multiline"': 'xai_grok_i18n::t("settings.multiline_mode.label")',
        '"Vim scrollback"': 'xai_grok_i18n::t("settings.vim_mode.label")',
        '"Thinking blocks"': 'xai_grok_i18n::t("settings.show_thinking_blocks.label")',
        '"Group tool calls"': 'xai_grok_i18n::t("settings.group_tool_verbs.label")',
        '"Collapsed edit blocks"': 'xai_grok_i18n::t("settings.collapsed_edit_blocks.label")',
        '"Prompt suggestions"': 'xai_grok_i18n::t("settings.prompt_suggestions.label")',
        '"Invert scroll"': 'xai_grok_i18n::t("settings.invert_scroll.label")',
        '"Respect manual folds"': 'xai_grok_i18n::t("settings.respect_manual_folds.label")',
        '"Compact mode"': 'xai_grok_i18n::t("settings.compact_mode.label")',
        '"Timestamps"': 'xai_grok_i18n::t("settings.show_timestamps.label")',
        '"Timeline sidebar"': 'xai_grok_i18n::t("settings.show_timeline.label")',
        '"Snap prompt to top on send"': 'xai_grok_i18n::t("settings.page_flip_on_send.label")',
        '"Disable vim input mode"': 'xai_grok_i18n::t("settings.simple_mode.label")',
    }
    for a, b in label_map.items():
        if f"save_success_toast({a}," in st:
            st = st.replace(f"save_success_toast({a},", f"save_success_toast({b},")
            print("setter label", a)
    setters.write_text(st, encoding="utf-8")


def patch_shortcuts_categories() -> None:
    p = ROOT / "crates/codegen/xai-grok-pager/src/views/shortcuts_help.rs"
    t = p.read_text(encoding="utf-8")
    old = '''const CATEGORY_ORDER: &[(Category, &str)] = &[
    (Category::GettingStarted, "Essentials"),
    (Category::Input, "Input"),
    (Category::ConversationNav, "Conversation Navigation"),
    (Category::ConversationAction, "Conversation Actions"),
    (Category::Panels, "Panels"),
    (Category::Session, "Session"),
    (Category::Dashboard, "Dashboard"),
];'''
    new = '''fn category_order() -> [(Category, &'static str); 7] {
    [
        (Category::GettingStarted, xai_grok_i18n::t("shortcuts.category.essentials")),
        (Category::Input, xai_grok_i18n::t("shortcuts.category.input")),
        (Category::ConversationNav, xai_grok_i18n::t("shortcuts.category.conversation_nav")),
        (Category::ConversationAction, xai_grok_i18n::t("shortcuts.category.conversation_action")),
        (Category::Panels, xai_grok_i18n::t("shortcuts.category.panels")),
        (Category::Session, xai_grok_i18n::t("shortcuts.category.session")),
        (Category::Dashboard, xai_grok_i18n::t("shortcuts.category.dashboard")),
    ]
}'''
    if old not in t:
        print("CATEGORY_ORDER MISS")
        return
    t = t.replace(old, new)
    t = t.replace("CATEGORY_ORDER.len()", "category_order().len()")
    t = t.replace("CATEGORY_ORDER.iter()", "category_order().iter()")
    # default_collapsed uses CATEGORY_ORDER.len - fixed
    # Other CATEGORY_ORDER references
    t = re.sub(r"\bCATEGORY_ORDER\b", "category_order()", t)
    t = t.replace("category_order().len()", "category_order().len()")  # noop safe
    # Fix double call if any
    t = t.replace("category_order()()", "category_order()")
    p.write_text(t, encoding="utf-8")
    print("shortcuts categories patched")


def patch_tool_prefixes() -> None:
    files = {
        "list_dir.rs": [
            ('let prefix = "List ";', 'let prefix = xai_grok_i18n::t("tool.prefix.list");'),
        ],
        "edit.rs": [
            ('prefix: "Edit ",', 'prefix: "Edit ",  // localized at render via tool.prefix.edit'),
        ],
        "read.rs": [
            ('"Read "', 'xai_grok_i18n::t("tool.prefix.read")'),
        ],
        "execute.rs": [
            ('"Run "', 'xai_grok_i18n::t("tool.prefix.run")'),
        ],
        "search.rs": [
            ('"Search "', 'xai_grok_i18n::t("tool.prefix.search")'),
        ],
        "memory_search.rs": [
            ('let prefix = "Memory Search ";', 'let prefix = xai_grok_i18n::t("tool.prefix.memory_search");'),
        ],
        "web_search.rs": [
            ('"Web search"', 'xai_grok_i18n::t("tool.prefix.web_search").trim_end()'),
        ],
        "web_fetch.rs": [
            ('"Web fetch"', 'xai_grok_i18n::t("tool.prefix.web_fetch").trim_end()'),
        ],
    }
    root = ROOT / "crates/codegen/xai-grok-pager/src/scrollback/blocks/tool"
    for name, pairs in files.items():
        p = root / name
        if not p.exists():
            print("tool missing", name)
            continue
        t = p.read_text(encoding="utf-8")
        for a, b in pairs:
            if a in t:
                t = t.replace(a, b)
                print("tool ok", name, a[:30])
            else:
                print("tool MISS", name, a[:40])
        # edit.rs: localize prefix at use site
        if name == "edit.rs":
            t = t.replace(
                "let prefix = self.prefix;",
                'let prefix = if self.prefix == "Edit " {\n            xai_grok_i18n::t("tool.prefix.edit")\n        } else if self.prefix == "Creating " {\n            xai_grok_i18n::t("tool.prefix.creating")\n        } else {\n            self.prefix\n        };',
            )
        p.write_text(t, encoding="utf-8")


def patch_docs() -> None:
    p = ROOT / "crates/codegen/xai-grok-pager/src/docs.rs"
    t = p.read_text(encoding="utf-8")
    # Change From<&Doc> for DocEntry to localize title/description by filename
    old = '''impl From<&Doc> for DocEntry {
    fn from(d: &Doc) -> Self {
        Self {
            title: d.title.into(),
            description: d.description.into(),
            content: d.content,
        }
    }
}'''
    new = '''impl From<&Doc> for DocEntry {
    fn from(d: &Doc) -> Self {
        let (title, description) = localized_doc_meta(d);
        Self {
            title: title.into(),
            description: description.into(),
            content: d.content,
        }
    }
}

fn localized_doc_meta(d: &Doc) -> (&'static str, &'static str) {
    // Map guide file basename prefix → catalog keys docs.NN.{title,desc}
    let num = d.filename.get(..2).unwrap_or("");
    let title_key = xai_grok_i18n::intern_key(&format!("docs.{num}.title"));
    let desc_key = xai_grok_i18n::intern_key(&format!("docs.{num}.desc"));
    (
        xai_grok_i18n::t_or(title_key, d.title),
        xai_grok_i18n::t_or(desc_key, d.description),
    )
}'''
    if old in t:
        t = t.replace(old, new)
        p.write_text(t, encoding="utf-8")
        print("docs patched")
    else:
        print("docs MISS From impl")


def patch_chat_modes_locale() -> None:
    cargo = ROOT / "crates/codegen/xai-grok-shell/Cargo.toml"
    ct = cargo.read_text(encoding="utf-8")
    if "xai-grok-i18n" not in ct:
        # add dependency near other xai-grok deps
        ct = ct.replace(
            "xai-grok-config = ",
            "xai-grok-i18n = { workspace = true }\nxai-grok-config = ",
            1,
        )
        # might not have that line - try another
        if "xai-grok-i18n" not in ct:
            # append under [dependencies]
            ct = re.sub(
                r"(\[dependencies\]\n)",
                r"\1xai-grok-i18n = { workspace = true }\n",
                ct,
                count=1,
            )
        cargo.write_text(ct, encoding="utf-8")
        print("shell Cargo.toml +i18n")

    p = ROOT / "crates/codegen/xai-grok-shell/src/agent/chat_modes.rs"
    t = p.read_text(encoding="utf-8")
    t = t.replace(
        "let locale = DEFAULT_LOCALE;",
        'let locale = ui_locale();',
    )
    # Also fix spawn_refresh / early seed using DEFAULT_LOCALE where appropriate
    if "fn ui_locale" not in t:
        t = t.replace(
            'const DEFAULT_LOCALE: &str = "en";',
            '''const DEFAULT_LOCALE: &str = "en";

/// UI language for remote catalogs: follows product locale (`en` / `zh-CN`).
fn ui_locale() -> &'static str {
    match xai_grok_i18n::current_locale() {
        xai_grok_i18n::Locale::ZhCn => "zh-CN",
        xai_grok_i18n::Locale::En => "en",
    }
}''',
        )
    # process seed
    t = t.replace(
        "self.spawn_refresh(user_id, DEFAULT_LOCALE);",
        "self.spawn_refresh(user_id, ui_locale());",
    )
    p.write_text(t, encoding="utf-8")
    print("chat_modes locale patched")


def patch_common_toasts() -> None:
    # Broad common phrases across pager
    root = ROOT / "crates/codegen/xai-grok-pager/src"
    replacements = [
        ('show_toast("Copied!")', 'show_toast(xai_grok_i18n::t("toast.copied"))'),
        ('show_toast("Copied")', 'show_toast(xai_grok_i18n::t("toast.copied"))'),
        (
            'show_toast("No active session")',
            'show_toast(xai_grok_i18n::t("toast.no_active_session"))',
        ),
        (
            'show_toast("Copied image")',
            'show_toast(xai_grok_i18n::t("toast.copied"))',
        ),
    ]
    for path in root.rglob("*.rs"):
        t = path.read_text(encoding="utf-8")
        orig = t
        for a, b in replacements:
            t = t.replace(a, b)
        if t != orig:
            path.write_text(t, encoding="utf-8")
            print("toast", path.relative_to(ROOT))


def patch_status_blocks() -> None:
    p = ROOT / "crates/codegen/xai-grok-pager/src/app/status_blocks.rs"
    if not p.exists():
        print("status_blocks missing")
        return
    t = p.read_text(encoding="utf-8")
    pairs = [
        ('"Queue is empty."', 'xai_grok_i18n::t("status.queue_empty")'),
        ('"No background tasks."', 'xai_grok_i18n::t("status.no_tasks")'),
    ]
    for a, b in pairs:
        if a in t:
            t = t.replace(a, b)
            print("status ok", a)
        else:
            print("status MISS", a)
    p.write_text(t, encoding="utf-8")


def main() -> None:
    patch_settings_footers()
    patch_save_success_toast()
    patch_shortcuts_categories()
    patch_tool_prefixes()
    patch_docs()
    patch_chat_modes_locale()
    patch_common_toasts()
    patch_status_blocks()


if __name__ == "__main__":
    main()
