#!/usr/bin/env python3
"""Wire Phase 1 call sites to xai_grok_i18n::t(...) for welcome + slash."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def patch_welcome() -> None:
    p = ROOT / "crates/codegen/xai-grok-pager/src/views/welcome/mod.rs"
    t = p.read_text(encoding="utf-8")
    pairs = [
        ('"Yes, proceed"', 'xai_grok_i18n::t("welcome.trust.yes")'),
        ('"No, quit"', 'xai_grok_i18n::t("welcome.trust.no")'),
        (
            '"Do you trust the contents of this directory?"',
            'xai_grok_i18n::t("welcome.trust.question")',
        ),
        (
            '"Grok Build may run or modify contents in this directory,"',
            'xai_grok_i18n::t("welcome.trust.warning_line1")',
        ),
        (
            '"posing security risks."',
            'xai_grok_i18n::t("welcome.trust.warning_line2")',
        ),
        ('"Resume session"', 'xai_grok_i18n::t("welcome.resume_session")'),
        ('"Changelog"', 'xai_grok_i18n::t("welcome.changelog")'),
        ('"New worktree"', 'xai_grok_i18n::t("welcome.new_worktree")'),
        (
            '"Import Claude settings"',
            'xai_grok_i18n::t("welcome.import_claude_settings")',
        ),
        ('"Switch account"', 'xai_grok_i18n::t("welcome.switch_account")'),
        ('"Logout"', 'xai_grok_i18n::t("welcome.logout")'),
        (
            '"Upgrade Subscription"',
            'xai_grok_i18n::t("welcome.upgrade_subscription")',
        ),
        (
            '"Logged in with API key"',
            'xai_grok_i18n::t("welcome.logged_in_api_key")',
        ),
        (
            '"Grok Build is not yet available for this account."',
            'xai_grok_i18n::t("welcome.zdr_unavailable")',
        ),
        (
            '"SuperGrok subscription required"',
            'xai_grok_i18n::t("welcome.supergrok_required")',
        ),
        ('"Connecting..."', 'xai_grok_i18n::t("auth.connecting")'),
        (
            '"Waiting for login to complete..."',
            'xai_grok_i18n::t("auth.waiting_login")',
        ),
        (
            '"Waiting for approval..."',
            'xai_grok_i18n::t("auth.waiting_approval")',
        ),
        (
            '"Waiting for auth URL..."',
            'xai_grok_i18n::t("auth.waiting_url")',
        ),
        ('"copied!"', 'xai_grok_i18n::t("auth.copied")'),
        ('"copy failed"', 'xai_grok_i18n::t("auth.copy_failed")'),
        ('"go back"', 'xai_grok_i18n::t("auth.go_back")'),
        ('"submit"', 'xai_grok_i18n::t("auth.submit")'),
        (
            '"Paste your token here..."',
            'xai_grok_i18n::t("auth.paste_token_placeholder")',
        ),
    ]
    for a, b in pairs:
        if a not in t:
            print("welcome MISS", a[:60])
        else:
            t = t.replace(a, b)
            print("welcome ok", a[:40])

    # Menu "Quit" is title-case — only replace menu tuples carefully
    t = t.replace('("q", "Quit")', '("q", xai_grok_i18n::t("welcome.quit_menu"))')
    t = t.replace('("q", "Quit",', '("q", xai_grok_i18n::t("welcome.quit_menu"),')

    old = 'const AUTH_HEADER: &str = "A browser window will open for authentication.";'
    new = 'fn auth_header() -> &\'static str { xai_grok_i18n::t("auth.browser_open_header") }'
    t = t.replace(old, new)
    old = 'const DEVICE_AUTH_HEADER: &str = "Approve in your browser to finish signing in.";'
    new = 'fn device_auth_header() -> &\'static str { xai_grok_i18n::t("auth.device_header") }'
    t = t.replace(old, new)
    old = 'const DEVICE_CODE_CAPTION: &str = "Make sure your browser shows this code.";'
    new = 'fn device_code_caption() -> &\'static str { xai_grok_i18n::t("auth.device_code_caption") }'
    t = t.replace(old, new)

    t = re.sub(r"(?<![_a-zA-Z])AUTH_HEADER(?!\()", "auth_header()", t)
    t = re.sub(r"(?<![_a-zA-Z])DEVICE_AUTH_HEADER(?!\()", "device_auth_header()", t)
    t = re.sub(r"(?<![_a-zA-Z])DEVICE_CODE_CAPTION(?!\()", "device_code_caption()", t)

    p.write_text(t, encoding="utf-8")
    print("welcome patched")


def patch_hero_prompt() -> None:
    hero = ROOT / "crates/codegen/xai-grok-pager/src/views/welcome/hero_box.rs"
    t = hero.read_text(encoding="utf-8")
    old = 'const HERO_SUBTITLE: &str = "Thanks for trying Grok Build, give feedback with /feedback!";'
    if old in t:
        t = t.replace(
            old,
            'fn hero_subtitle() -> &\'static str { xai_grok_i18n::t("welcome.hero_subtitle") }',
        )
        t = re.sub(r"(?<![_a-zA-Z])HERO_SUBTITLE(?!\()", "hero_subtitle()", t)
        hero.write_text(t, encoding="utf-8")
        print("hero_box patched")
    prompt = ROOT / "crates/codegen/xai-grok-pager/src/views/welcome/prompt.rs"
    t = prompt.read_text(encoding="utf-8")
    if 'Some("Type a message...")' in t:
        t = t.replace(
            'Some("Type a message...")',
            'Some(xai_grok_i18n::t("welcome.type_message"))',
        )
        prompt.write_text(t, encoding="utf-8")
        print("prompt patched")


def patch_slash() -> None:
    root = ROOT / "crates/codegen/xai-grok-pager/src/slash/commands"
    count = 0
    for f in root.glob("*.rs"):
        t = f.read_text(encoding="utf-8")
        names = re.findall(r'fn name\(&self\)[^{]*\{\s*"([^"]+)"', t)
        descs = re.findall(
            r'(fn description\(&self\)[^{]*\{\s*)"((?:\\.|[^"])*)"(\s*\})', t
        )
        if not descs:
            continue
        new_t = t
        for i, (prefix, eng, suffix) in list(enumerate(descs))[::-1]:
            if i >= len(names):
                continue
            name = names[i]
            key = f"slash.{name}.description"
            old = f'{prefix}"{eng}"{suffix}'
            replacement = f'{prefix}xai_grok_i18n::t("{key}"){suffix}'
            if old in new_t:
                new_t = new_t.replace(old, replacement, 1)
                count += 1
        if new_t != t:
            f.write_text(new_t, encoding="utf-8")
            print("slash", f.name)
    print("slash total", count)


def patch_turn_status() -> None:
    p = ROOT / "crates/codegen/xai-grok-pager/src/views/turn_status.rs"
    t = p.read_text(encoding="utf-8")
    pairs = [
        ('"Cancelling…"', 'xai_grok_i18n::t("turn.activity.cancelling")'),
        ('"Verifying…"', 'xai_grok_i18n::t("turn.activity.verifying")'),
        ('"Thinking…"', 'xai_grok_i18n::t("turn.activity.thinking")'),
        ('"Responding…"', 'xai_grok_i18n::t("turn.activity.responding")'),
        ('"Compacting…"', 'xai_grok_i18n::t("turn.activity.compacting")'),
        ('"Running…"', 'xai_grok_i18n::t("turn.activity.running")'),
        ('"Waiting…"', 'xai_grok_i18n::t("turn.activity.waiting")'),
        (
            '"Starting session…"',
            'xai_grok_i18n::t("turn.activity.starting_session")',
        ),
        ('"[stop]"', 'xai_grok_i18n::t("turn.button.stop")'),
        ('" [stop]"', 'xai_grok_i18n::t("turn.button.stop_spaced")'),
        ('"watching"', 'xai_grok_i18n::t("turn.watchers.watching")'),
        ('"Run "', 'xai_grok_i18n::t("turn.tool.prefix.run")'),
        ('"Search "', 'xai_grok_i18n::t("turn.tool.prefix.search")'),
        ('"Fetch "', 'xai_grok_i18n::t("turn.tool.prefix.fetch")'),
    ]
    for a, b in pairs:
        if a in t:
            t = t.replace(a, b)
            print("turn ok", a)
        else:
            print("turn MISS", a)
    p.write_text(t, encoding="utf-8")

    tracker = ROOT / "crates/codegen/xai-grok-pager/src/acp/tracker.rs"
    t = tracker.read_text(encoding="utf-8")
    pairs = [
        (
            '"Waiting for response…"',
            'xai_grok_i18n::t("turn.wait.model")',
        ),
        (
            '"Waiting on subagent…"',
            'xai_grok_i18n::t("turn.wait.subagent")',
        ),
        (
            '"Waiting on task output…"',
            'xai_grok_i18n::t("turn.wait.task_output")',
        ),
        (
            '"Waiting on tasks…"',
            'xai_grok_i18n::t("turn.wait.tasks_complete")',
        ),
        ('"Sleeping…"', 'xai_grok_i18n::t("turn.wait.sleep")'),
    ]
    for a, b in pairs:
        if a in t:
            t = t.replace(a, b)
            print("tracker ok", a)
        else:
            print("tracker MISS", a)
    tracker.write_text(t, encoding="utf-8")


def patch_actions() -> None:
    """Add label_t/description_t helpers on ActionDef and use in hint()."""
    mod = ROOT / "crates/codegen/xai-grok-pager/src/actions/mod.rs"
    t = mod.read_text(encoding="utf-8")
    if "fn label_t" in t:
        print("actions already has label_t")
        return
    # Insert impl methods before existing impl ActionDef
    needle = "impl ActionDef {\n    /// Convert this action def into a [`HintItem`]"
    insert = '''impl ActionDef {
    /// Localized short label for the current UI locale.
    pub fn label_t(&self) -> &'static str {
        let key = xai_grok_i18n::intern_key(&format!("actions.{:?}.label", self.id));
        xai_grok_i18n::t_or(key, self.label)
    }

    /// Localized description for the current UI locale.
    pub fn description_t(&self) -> &'static str {
        let key = xai_grok_i18n::intern_key(&format!("actions.{:?}.description", self.id));
        xai_grok_i18n::t_or(key, self.description)
    }

    /// Convert this action def into a [`HintItem`]'''
    if needle not in t:
        print("actions MISS impl block")
        return
    t = t.replace(needle, insert)
    t = t.replace(
        "let mut item = HintItem::new(self.default_key, self.label);\n        item.custom_display = self.hint_key_display;\n        item.description = Some(std::borrow::Cow::Borrowed(self.description));",
        "let mut item = HintItem::new(self.default_key, self.label_t());\n        item.custom_display = self.hint_key_display;\n        item.description = Some(std::borrow::Cow::Borrowed(self.description_t()));",
    )
    mod.write_text(t, encoding="utf-8")
    print("actions helpers patched")


def patch_tips() -> None:
    tips = {
        "clear_detector.rs": [
            ('"Input cleared · "', 'xai_grok_i18n::t("tips.undo.prefix")'),
            ('" to undo"', 'xai_grok_i18n::t("tips.undo.suffix")'),
        ],
        "clipboard_focus.rs": [
            (
                '"Image in clipboard · "',
                'xai_grok_i18n::t("tips.clipboard_image.prefix")',
            ),
            ('" to paste"', 'xai_grok_i18n::t("tips.clipboard_image.suffix")'),
        ],
        "plan_nudge.rs": [
            (
                '"Planning? Check out plan mode via "',
                'xai_grok_i18n::t("tips.plan_nudge")',
            ),
        ],
        "send_now.rs": [
            ('"Queued · "', 'xai_grok_i18n::t("tips.send_now.prefix")'),
            ('" to send now"', 'xai_grok_i18n::t("tips.send_now.suffix")'),
        ],
        "small_screen.rs": [
            (
                '"Tight on space? Try /compact-mode"',
                'xai_grok_i18n::t("tips.small_screen")',
            ),
        ],
    }
    root = ROOT / "crates/codegen/xai-grok-pager/src/tips"
    for name, pairs in tips.items():
        p = root / name
        if not p.exists():
            print("tips missing file", name)
            continue
        t = p.read_text(encoding="utf-8")
        for a, b in pairs:
            if a in t:
                t = t.replace(a, b)
                print("tips ok", name, a[:30])
            else:
                print("tips MISS", name, a[:40])
        p.write_text(t, encoding="utf-8")


def main() -> None:
    patch_welcome()
    patch_hero_prompt()
    patch_slash()
    patch_turn_status()
    patch_actions()
    patch_tips()


if __name__ == "__main__":
    main()
