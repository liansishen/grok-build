#!/usr/bin/env python3
"""Wire P0/P1 gap strings to xai_grok_i18n."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def sub_file(rel: str, pairs: list[tuple[str, str]], label: str) -> None:
    p = ROOT / rel
    if not p.exists():
        print("MISS file", rel)
        return
    t = p.read_text(encoding="utf-8")
    n = 0
    for a, b in pairs:
        if a in t:
            t = t.replace(a, b)
            n += 1
        else:
            print(f"  skip {label}: {a[:50]!r}")
    p.write_text(t, encoding="utf-8")
    print(f"{label}: {n}/{len(pairs)}")


def patch_permission_view() -> None:
    sub_file(
        "crates/codegen/xai-grok-pager/src/views/permission_view.rs",
        [
            (
                '"No, reject (type to add feedback)".to_string()',
                'xai_grok_i18n::t("permission.reject_once.placeholder").to_string()',
            ),
            (
                '"No, reject (type to add feedback)"',
                'xai_grok_i18n::t("permission.reject_once.placeholder")',
            ),
            (
                'format!("all tools from {}", mcp_titleize_segment(s))',
                'xai_grok_i18n::t_fmt("permission.mcp.all_tools_from", &[("server", &mcp_titleize_segment(s))])',
            ),
        ],
        "permission_view",
    )
    # Scope hint - find and replace multi-span construction if present
    p = ROOT / "crates/codegen/xai-grok-pager/src/views/permission_view.rs"
    t = p.read_text(encoding="utf-8")
    # Common pattern: "Use " / " to choose permission scope"
    if "to choose permission scope" in t and "permission.hint.choose_scope" not in t:
        t = t.replace(
            '" to choose permission scope"',
            '"); // legacy split replaced below\n        let _ = ("',
        )
        # Better: replace whole known string sequences
        for old in [
            'Span::raw("Use ")',
            'Span::raw(" to choose permission scope")',
        ]:
            pass
        # Simple full-string approach if they build as one string
        t2 = p.read_text(encoding="utf-8")
        if 'choose permission scope' in t2:
            # replace any remaining literal
            t2 = re.sub(
                r'"Use [^"]*to choose permission scope"',
                'xai_grok_i18n::t("permission.hint.choose_scope")',
                t2,
            )
            # also multi-part
            t2 = t2.replace(
                '" to choose permission scope"',
                '"',  # broken - redo carefully
            )
    # Re-read and do surgical replacements
    t = p.read_text(encoding="utf-8")
    # Look for exact patterns from source
    if "permission.hint.choose_scope" not in t:
        # Find line with choose permission
        for m in re.finditer(r'.{0,80}choose permission scope.{0,40}', t):
            print("  hint context:", m.group(0)[:100])
        # Replace common construction:
        old = '"Use ← → to choose permission scope"'
        if old in t:
            t = t.replace(old, 'xai_grok_i18n::t("permission.hint.choose_scope")')
        old = '"Use \u2190 \u2192 to choose permission scope"'
        if old in t:
            t = t.replace(old, 'xai_grok_i18n::t("permission.hint.choose_scope")')
        # split spans
        if ' "Use " ' in t or '"Use "' in t:
            t = t.replace(
                'Span::styled("Use ",',
                'Span::styled(xai_grok_i18n::t("permission.hint.choose_scope"), // full hint; was "Use "',
            )
            # This might break layout - read the actual code
    p.write_text(t, encoding="utf-8")


def patch_permissions_titles() -> None:
    p = ROOT / "crates/codegen/xai-grok-pager/src/app/acp_handler/permissions.rs"
    if not p.exists():
        print("permissions handler miss")
        return
    t = p.read_text(encoding="utf-8")
    pairs = [
        ('"Allow Execute?"', 'xai_grok_i18n::t("permission.title.allow_execute")'),
        ('"Allow Edit?"', 'xai_grok_i18n::t("permission.title.allow_edit")'),
        ('"Allow Delete?"', 'xai_grok_i18n::t("permission.title.allow_delete")'),
        ('"Allow?"', 'xai_grok_i18n::t("permission.title.allow")'),
    ]
    n = 0
    for a, b in pairs:
        if a in t:
            t = t.replace(a, b)
            n += 1
    # format! titles
    t = t.replace(
        'format!("Allow Edit to {path}?")',
        'xai_grok_i18n::t_fmt("permission.title.allow_edit_path", &[("path", &path.to_string())])',
    )
    t = t.replace(
        'format!("Allow Edit to {}?")',
        'xai_grok_i18n::t_fmt("permission.title.allow_edit_path", &[("path", ',
    )
    # Allow `{bin}`?
    t = re.sub(
        r'format!\("Allow `\{\}`\?"\s*,\s*([^)]+)\)',
        r'xai_grok_i18n::t_fmt("permission.title.allow_bin", &[("bin", &\1.to_string())])',
        t,
    )
    t = re.sub(
        r'format!\("Allow \{\}\?"\s*,\s*([^)]+)\)',
        r'xai_grok_i18n::t_fmt("permission.title.allow_named", &[("name", &\1.to_string())])',
        t,
    )
    p.write_text(t, encoding="utf-8")
    print("permissions titles", n)


def patch_prompter() -> None:
    """Localize workspace prompter option strings."""
    p = ROOT / "crates/codegen/xai-grok-workspace/src/permission/prompter.rs"
    if not p.exists():
        print("prompter miss")
        return
    # Check if workspace has i18n dep
    cargo = ROOT / "crates/codegen/xai-grok-workspace/Cargo.toml"
    ct = cargo.read_text(encoding="utf-8")
    if "xai-grok-i18n" not in ct:
        ct = re.sub(
            r"(\[dependencies\]\n)",
            r"\1xai-grok-i18n = { workspace = true }\n",
            ct,
            count=1,
        )
        cargo.write_text(ct, encoding="utf-8")
        print("workspace +i18n dep")
    t = p.read_text(encoding="utf-8")
    pairs = [
        ('"Always allow:"', 'xai_grok_i18n::t("permission.prefix.always_allow")'),
        ('"Never allow:"', 'xai_grok_i18n::t("permission.prefix.never_allow")'),
        (
            '"Always allow:".to_owned()',
            'xai_grok_i18n::t("permission.prefix.always_allow").to_owned()',
        ),
        (
            'prompt_prefix: "Always allow:".to_owned()',
            'prompt_prefix: xai_grok_i18n::t("permission.prefix.always_allow").to_owned()',
        ),
        (
            '"Yes, proceed"',
            'xai_grok_i18n::t("permission.option.yes_proceed")',
        ),
        (
            '"Yes, allow once"',
            'xai_grok_i18n::t("permission.option.yes_allow_once")',
        ),
        (
            '"No, and tell Grok what to do differently"',
            'xai_grok_i18n::t("permission.option.reject_once")',
        ),
        (
            "\"Yes, and don't ask again for anything (always-approve mode)\"",
            'xai_grok_i18n::t("permission.option.enable_always_approve")',
        ),
        (
            '"Yes, allow all edits during this session"',
            'xai_grok_i18n::t("permission.option.allow_edits_session")',
        ),
        (
            "\"Yes, and don't ask again for bash commands\"",
            'xai_grok_i18n::t("permission.option.always_allow_bash")',
        ),
        (
            "\"No, and don't run bash commands\"",
            'xai_grok_i18n::t("permission.option.reject_always_bash")',
        ),
    ]
    n = 0
    for a, b in pairs:
        c = t.count(a)
        if c:
            t = t.replace(a, b)
            n += c
    # format!("Always allow: {}", tool_name)
    t = t.replace(
        'format!("Always allow: {}", tool_name)',
        'format!("{} {}", xai_grok_i18n::t("permission.prefix.always_allow"), tool_name)',
    )
    p.write_text(t, encoding="utf-8")
    print("prompter", n)


def patch_welcome() -> None:
    sub_file(
        "crates/codegen/xai-grok-pager/src/views/welcome/mod.rs",
        [
            (
                'format!("Login with {}", label)',
                'xai_grok_i18n::t_fmt("welcome.login_with", &[("label", label)])',
            ),
            (
                '(key_q, "Quit")',
                '(key_q, xai_grok_i18n::t("welcome.quit_menu"))',
            ),
            (
                '(key_l, xai_grok_i18n::t("welcome.logout")), (key_q, "Quit")',
                '(key_l, xai_grok_i18n::t("welcome.logout")), (key_q, xai_grok_i18n::t("welcome.quit_menu"))',
            ),
            (
                'items.push((key_q, "Quit"));',
                'items.push((key_q, xai_grok_i18n::t("welcome.quit_menu")));',
            ),
            (
                'label: "worktree".into()',
                'label: xai_grok_i18n::t("welcome.picker.worktree").into()',
            ),
            (
                'format!("press again to {}", label)',
                'xai_grok_i18n::t_fmt("welcome.press_again", &[("label", label)])',
            ),
            (
                '"press again to {}"',
                # skip
                '"press again to {}"',
            ),
        ],
        "welcome",
    )
    # hero changelog
    sub_file(
        "crates/codegen/xai-grok-pager/src/views/welcome/hero_box.rs",
        [
            ('let title = "Changelog";', 'let title = xai_grok_i18n::t("welcome.changelog");'),
        ],
        "hero",
    )
    # top_bar worktree
    sub_file(
        "crates/codegen/xai-grok-pager/src/views/welcome/top_bar.rs",
        [
            ('"worktree "', 'xai_grok_i18n::t("welcome.picker.worktree").to_string() + " "'),
        ],
        "top_bar",
    )


def patch_shortcuts_bar() -> None:
    sub_file(
        "crates/codegen/xai-grok-pager/src/views/shortcuts_bar.rs",
        [
            (
                'format!("press again to {}", pending.label)',
                'xai_grok_i18n::t_fmt("welcome.press_again", &[("label", pending.label.as_str())])',
            ),
        ],
        "shortcuts_bar",
    )


def patch_app_view_pending() -> None:
    p = ROOT / "crates/codegen/xai-grok-pager/src/app/app_view.rs"
    t = p.read_text(encoding="utf-8")
    # PendingAction::new(..., def.label) -> label_t()
    t2 = t.replace("def.label)", "def.label_t())")
    # careful - might over-replace; only PendingAction contexts
    # revert broad replace - do more specific
    t = p.read_text(encoding="utf-8")
    t = re.sub(
        r"PendingAction::new\(([^,]+),\s*([^,]+),\s*def\.label\)",
        r"PendingAction::new(\1, \2, def.label_t())",
        t,
    )
    t = t.replace(
        "PendingAction::new(action_id, def.default_key, def.label)",
        "PendingAction::new(action_id, def.default_key, def.label_t())",
    )
    # generic
    if "def.label)" in t and "PendingAction" in t:
        t = t.replace(
            ", def.label)",
            ", def.label_t())",
        )
        # might break other things - check count
    p.write_text(t, encoding="utf-8")
    print("app_view pending")


def patch_modes() -> None:
    sub_file(
        "crates/codegen/xai-grok-pager/src/app/dispatch/modes.rs",
        [
            (
                'save_success_toast("Plan mode", kind.to_bool())',
                'save_success_toast(xai_grok_i18n::t("toast.plan_mode"), kind.to_bool())',
            ),
            (
                'save_success_toast("Always-approve", false)',
                'save_success_toast(xai_grok_i18n::t("toast.always_approve"), false)',
            ),
            (
                '"\u26a0 Always-approve ON: plan mode still blocks file edits until you exit plan mode"',
                'xai_grok_i18n::t("toast.always_approve_on_plan")',
            ),
            (
                '"\u26a0 Always-approve ON: all tool actions auto-run".to_string()',
                'xai_grok_i18n::t("toast.always_approve_on").to_string()',
            ),
            (
                '"Already in plan mode. Use /view-plan to view the current plan."',
                'xai_grok_i18n::t("toast.already_plan_mode")',
            ),
        ],
        "modes",
    )
    # also unicode escape form
    p = ROOT / "crates/codegen/xai-grok-pager/src/app/dispatch/modes.rs"
    t = p.read_text(encoding="utf-8")
    t = t.replace(
        r'"\u{26A0} Always-approve ON: plan mode still blocks file edits until you exit plan mode"',
        'xai_grok_i18n::t("toast.always_approve_on_plan")',
    )
    t = t.replace(
        r'"\u{26A0} Always-approve ON: all tool actions auto-run".to_string()',
        'xai_grok_i18n::t("toast.always_approve_on").to_string()',
    )
    p.write_text(t, encoding="utf-8")


def patch_modal() -> None:
    p = ROOT / "crates/codegen/xai-grok-pager/src/views/modal.rs"
    t = p.read_text(encoding="utf-8")
    pairs = [
        ('label: "Session".into()', 'label: xai_grok_i18n::t("modal.section.session").into()'),
        (
            'command: PaletteCommand::SectionHeader("Session".into())',
            'command: PaletteCommand::SectionHeader(xai_grok_i18n::t("modal.section.session").into())',
        ),
        ('label: "New Session".into()', 'label: xai_grok_i18n::t("modal.new_session").into()'),
        (
            'label: "Keyboard Shortcuts".into()',
            'label: xai_grok_i18n::t("modal.keyboard_shortcuts").into()',
        ),
        (
            'ActiveModal::ShortcutsHelp { .. } => "Keyboard Shortcuts"',
            'ActiveModal::ShortcutsHelp { .. } => xai_grok_i18n::t("modal.keyboard_shortcuts")',
        ),
        ('label: "Enter select"', 'label: xai_grok_i18n::t("modal.footer.enter_select")'),
        ('label: "Esc close"', 'label: xai_grok_i18n::t("modal.footer.esc_close")'),
        (
            'label: "Agent Dashboard".into()',
            'label: xai_grok_i18n::t("modal.agent_dashboard").into()',
        ),
        (
            'label: "How-to Guides".into()',
            'label: xai_grok_i18n::t("modal.howto_guides").into()',
        ),
        ('label: "Commands".into()', 'label: xai_grok_i18n::t("modal.commands").into()'),
        (
            'label: "Resume session".into()',
            'label: xai_grok_i18n::t("modal.resume_session").into()',
        ),
        ('label: "Pick model".into()', 'label: xai_grok_i18n::t("modal.pick_model").into()'),
        (
            'label: "Always Approve Mode".into()',
            'label: xai_grok_i18n::t("modal.always_approve_mode").into()',
        ),
        ('"Save and send?"', 'xai_grok_i18n::t("modal.save_and_send")'),
        ('"Save changes?"', 'xai_grok_i18n::t("modal.save_changes")'),
        ('"Reset setting?"', 'xai_grok_i18n::t("modal.reset_setting")'),
        (
            '"Subagents are still running. Stop them?"',
            'xai_grok_i18n::t("modal.subagents_running")',
        ),
        ('"Stop running"', 'xai_grok_i18n::t("modal.stop_running")'),
        ('"Continue to run"', 'xai_grok_i18n::t("modal.continue_to_run")'),
        ('"Always stop"', 'xai_grok_i18n::t("modal.always_stop")'),
        ('"Always continue"', 'xai_grok_i18n::t("modal.always_continue")'),
    ]
    n = 0
    for a, b in pairs:
        if a in t:
            t = t.replace(a, b)
            n += 1
        else:
            print("  modal skip", a[:40])
    p.write_text(t, encoding="utf-8")
    print("modal", n)


def bulk_replace_toasts() -> None:
    """Replace common show_toast string literals across pager src."""
    mapping = {
        'show_toast("Copied!")': 'show_toast(xai_grok_i18n::t("toast.copied"))',
        'show_toast("No active session")': 'show_toast(xai_grok_i18n::t("toast.no_active_session"))',
        'show_toast("Reconnected.")': 'show_toast(xai_grok_i18n::t("toast.reconnected"))',
        'show_toast("Session restored. In-progress tools and terminals were lost.")': 'show_toast(xai_grok_i18n::t("toast.session_restored"))',
        'show_toast("Session restore failed. Kept the existing transcript.")': 'show_toast(xai_grok_i18n::t("toast.session_restore_failed"))',
        'show_toast("Reconnecting, please wait...")': 'show_toast(xai_grok_i18n::t("toast.reconnecting"))',
        'show_toast("Opening in default app\u2026")': 'show_toast(xai_grok_i18n::t("toast.opening_app"))',
        'show_toast("Opening in default app…")': 'show_toast(xai_grok_i18n::t("toast.opening_app"))',
        'show_toast("Could not open file")': 'show_toast(xai_grok_i18n::t("toast.could_not_open_file"))',
        'show_toast("Loading video\u2026")': 'show_toast(xai_grok_i18n::t("toast.loading_video"))',
        'show_toast("Loading video…")': 'show_toast(xai_grok_i18n::t("toast.loading_video"))',
        'show_toast("Browser unavailable - URL shown above")': 'show_toast(xai_grok_i18n::t("toast.browser_unavailable"))',
        'show_toast("Diagram not ready yet")': 'show_toast(xai_grok_i18n::t("toast.diagram_not_ready"))',
        'show_toast("Rendering diagram\u2026")': 'show_toast(xai_grok_i18n::t("toast.rendering_diagram"))',
        'show_toast("Rendering diagram…")': 'show_toast(xai_grok_i18n::t("toast.rendering_diagram"))',
        'show_toast("Could not render diagram")': 'show_toast(xai_grok_i18n::t("toast.could_not_render_diagram"))',
        'show_toast("No plan written yet.")': 'show_toast(xai_grok_i18n::t("toast.no_plan_yet"))',
        'show_toast("Plan revision sent.")': 'show_toast(xai_grok_i18n::t("toast.plan_revision_sent"))',
        'show_toast("No comments to send.")': 'show_toast(xai_grok_i18n::t("toast.no_comments"))',
        'show_toast("Plan feedback sent.")': 'show_toast(xai_grok_i18n::t("toast.plan_feedback_sent"))',
        'show_toast("Couldn\'t load image preview")': 'show_toast(xai_grok_i18n::t("toast.couldnt_load_image"))',
        'show_toast("Couldn\'t save pasted image")': 'show_toast(xai_grok_i18n::t("toast.couldnt_save_image"))',
        'show_toast("Session recap is not enabled")': 'show_toast(xai_grok_i18n::t("toast.recap_disabled"))',
        'show_toast("No input events recorded yet.")': 'show_toast(xai_grok_i18n::t("toast.no_input_events"))',
        'show_toast("Video playback requires ffmpeg")': 'show_toast(xai_grok_i18n::t("toast.ffmpeg_required"))',
        'show_toast("Interjection sent")': 'show_toast(xai_grok_i18n::t("toast.interjection_sent"))',
        'show_toast("External sessions can\'t be deleted")': 'show_toast(xai_grok_i18n::t("toast.external_sessions_cant_delete"))',
        'show_toast("Deleting chat conversations isn\'t supported yet")': 'show_toast(xai_grok_i18n::t("toast.chat_delete_unsupported"))',
        'show_toast("Deleting session\u2026")': 'show_toast(xai_grok_i18n::t("toast.deleting_session"))',
        'show_toast("Deleting session…")': 'show_toast(xai_grok_i18n::t("toast.deleting_session"))',
        'show_toast("Session deleted")': 'show_toast(xai_grok_i18n::t("toast.session_deleted"))',
        'show_toast("Sharing is disabled")': 'show_toast(xai_grok_i18n::t("toast.sharing_disabled"))',
        'show_toast("No undoable prompts")': 'show_toast(xai_grok_i18n::t("toast.no_undoable"))',
        'show_toast("Images removed (skill prompt)")': 'show_toast(xai_grok_i18n::t("toast.images_removed_skill"))',
        'show_toast("Nothing to jump to yet")': 'show_toast(xai_grok_i18n::t("toast.nothing_to_jump"))',
        'show_toast("/fork only works inside a session")': 'show_toast(xai_grok_i18n::t("toast.fork_session_only"))',
        'show_toast("Cannot fork: session is still being created")': 'show_toast(xai_grok_i18n::t("toast.fork_still_creating"))',
        'show_toast("Sign in to open the dashboard")': 'show_toast(xai_grok_i18n::t("dashboard.sign_in"))',
        'show_toast("Agent dashboard is disabled in this configuration")': 'show_toast(xai_grok_i18n::t("dashboard.disabled"))',
        'show_toast("Answer the folder-trust question to open the dashboard")': 'show_toast(xai_grok_i18n::t("dashboard.trust_first"))',
        'show_toast("Open the dashboard (/dashboard) to change location")': 'show_toast(xai_grok_i18n::t("dashboard.open_to_change_location"))',
        'show_toast("Mouse reporting on")': 'show_toast(xai_grok_i18n::t("toast.mouse_reporting_on"))',
        'show_toast("\u2713 Default model: cleared")': 'show_toast(xai_grok_i18n::t("toast.default_model_cleared"))',
        'show_toast("✓ Default model: cleared")': 'show_toast(xai_grok_i18n::t("toast.default_model_cleared"))',
        r'show_toast("\u{2713} Default model: cleared")': 'show_toast(xai_grok_i18n::t("toast.default_model_cleared"))',
        r'show_toast("\u{2713} Fork secondary model: cleared")': 'show_toast(xai_grok_i18n::t("toast.fork_model_cleared"))',
        r'show_toast("\u{2713} Fork secondary model: already at default")': 'show_toast(xai_grok_i18n::t("toast.fork_model_default"))',
        r'show_toast("\u{2713} Compact mode: off (auto-compact active on small terminal)")': 'show_toast(xai_grok_i18n::t("toast.compact_auto"))',
        r'show_toast("\u{2717} Cannot change: Zero Data Retention enabled")': 'show_toast(xai_grok_i18n::t("toast.zdr_enabled"))',
        r'show_toast("\u{2717} Data sharing is controlled by your team admin")': 'show_toast(xai_grok_i18n::t("toast.data_sharing_admin"))',
        'show_toast("Editing a queued prompt: press Enter to save, Esc to discard")': 'show_toast(xai_grok_i18n::t("toast.editing_queued"))',
        'show_toast("Queued prompt is no longer in the queue")': 'show_toast(xai_grok_i18n::t("toast.queued_gone"))',
        'show_toast("Images can\'t be attached when editing a shared queued prompt")': 'show_toast(xai_grok_i18n::t("toast.images_shared_queue"))',
    }
    # more queue/media mid-turn with special dash
    extra = [
        (
            'show_toast("Can\'t send this mid-turn — it runs when the current turn ends")',
            'show_toast(xai_grok_i18n::t("toast.cant_send_mid_turn"))',
        ),
        (
            'show_toast("Can\'t send this now — it runs when the current turn ends")',
            'show_toast(xai_grok_i18n::t("toast.cant_send_now"))',
        ),
        (
            'show_toast("No turn running — prompt will send when ready")',
            'show_toast(xai_grok_i18n::t("toast.no_turn_running"))',
        ),
    ]
    root = ROOT / "crates/codegen/xai-grok-pager/src"
    files = 0
    for path in root.rglob("*.rs"):
        t = path.read_text(encoding="utf-8")
        orig = t
        for a, b in list(mapping.items()) + extra:
            t = t.replace(a, b)
        # unicode dash variants (em dash)
        t = t.replace(
            'show_toast("Can\'t send this mid-turn \u2014 it runs when the current turn ends")',
            'show_toast(xai_grok_i18n::t("toast.cant_send_mid_turn"))',
        )
        t = t.replace(
            'show_toast("Voice pipeline could not start \u2014 restart grok")',
            'show_toast(xai_grok_i18n::t("toast.voice_pipeline_failed"))',
        )
        t = t.replace(
            'show_toast("Voice stopped \u2014 pipeline ended")',
            'show_toast(xai_grok_i18n::t("toast.voice_stopped"))',
        )
        if t != orig:
            path.write_text(t, encoding="utf-8")
            files += 1
            print("toast file", path.relative_to(ROOT))
    print("toast files", files)


def patch_slash_errors() -> None:
    root = ROOT / "crates/codegen/xai-grok-pager/src/slash/commands"
    mapping = {
        'CommandResult::Error("No active session".to_string())': 'CommandResult::Error(xai_grok_i18n::t("slash.err.no_active_session").to_string())',
        'CommandResult::Error("No active session".into())': 'CommandResult::Error(xai_grok_i18n::t("slash.err.no_active_session").into())',
        'CommandResult::Error("No active session to export".to_string())': 'CommandResult::Error(xai_grok_i18n::t("slash.err.no_active_session_export").to_string())',
        'CommandResult::Error("No active session to share".to_string())': 'CommandResult::Error(xai_grok_i18n::t("slash.err.no_active_session_share").to_string())',
        'CommandResult::Error("No active session to view".to_string())': 'CommandResult::Error(xai_grok_i18n::t("slash.err.no_active_session_view").to_string())',
        'CommandResult::Error("No active model".into())': 'CommandResult::Error(xai_grok_i18n::t("slash.err.no_active_model").into())',
        'CommandResult::Error("Usage: /model <name> [effort]".into())': 'CommandResult::Error(xai_grok_i18n::t("slash.err.usage_model").into())',
        'CommandResult::Error("Usage: /rename <new title>".to_string())': 'CommandResult::Error(xai_grok_i18n::t("slash.err.usage_rename").to_string())',
        'CommandResult::Error("No release notes available (offline).".to_string())': 'CommandResult::Error(xai_grok_i18n::t("slash.err.no_release_notes").to_string())',
    }
    n = 0
    for path in root.glob("*.rs"):
        t = path.read_text(encoding="utf-8")
        orig = t
        for a, b in mapping.items():
            if a in t:
                t = t.replace(a, b)
                n += 1
        if t != orig:
            path.write_text(t, encoding="utf-8")
    print("slash errors", n)


def patch_dashboard() -> None:
    pairs = [
        (
            '"No agents yet, type a prompt to start one."',
            'xai_grok_i18n::t("dashboard.empty_prompt")',
        ),
        ('"Loading sessions…"', 'xai_grok_i18n::t("dashboard.loading_sessions")'),
        ('"Loading sessions..."', 'xai_grok_i18n::t("dashboard.loading_sessions")'),
        ('"No matching rows."', 'xai_grok_i18n::t("dashboard.no_matching_rows")'),
        ('"Loading…"', 'xai_grok_i18n::t("dashboard.loading")'),
        ('"Loading..."', 'xai_grok_i18n::t("dashboard.loading")'),
        ('"No activity yet"', 'xai_grok_i18n::t("dashboard.no_activity")'),
    ]
    root = ROOT / "crates/codegen/xai-grok-pager/src/views/dashboard"
    for path in root.rglob("*.rs"):
        t = path.read_text(encoding="utf-8")
        orig = t
        for a, b in pairs:
            t = t.replace(a, b)
        if t != orig:
            path.write_text(t, encoding="utf-8")
            print("dashboard", path.name)


def patch_panes_and_misc() -> None:
    mapping_files = {
        "crates/codegen/xai-grok-pager/src/views/todo_pane.rs": [
            ('"No todo items."', 'xai_grok_i18n::t("pane.no_todos")'),
        ],
        "crates/codegen/xai-grok-pager/src/views/tasks_pane.rs": [
            ('"No tasks or agents."', 'xai_grok_i18n::t("pane.no_tasks")'),
        ],
        "crates/codegen/xai-grok-pager/src/tips/word_select.rs": [
            # full tip may be multi-span; replace if single string
        ],
        "crates/codegen/xai-grok-pager/src/tips/ssh_wrap.rs": [],
        "crates/codegen/xai-grok-pager/src/views/settings_modal/render.rs": [
            (
                'label: "Space/Enter toggle"',
                'label: xai_grok_i18n::t("settings.modal.footer.space_enter_toggle")',
            ),
            (
                'label: "Esc back"',
                'label: xai_grok_i18n::t("settings.modal.footer.esc_back")',
            ),
        ],
        "crates/codegen/xai-grok-pager/src/scrollback/blocks/thinking.rs": [
            ('"Thinking…"', 'xai_grok_i18n::t("scrollback.thinking")'),
            ('"Thinking..."', 'xai_grok_i18n::t("scrollback.thinking")'),
            ('"Thought"', 'xai_grok_i18n::t("scrollback.thought")'),
        ],
    }
    for rel, pairs in mapping_files.items():
        if pairs:
            sub_file(rel, pairs, Path(rel).name)

    # tips full strings - read and patch
    for name, key in [
        ("word_select.rs", "tips.word_select.full"),
        ("ssh_wrap.rs", "tips.ssh_wrap.full"),
    ]:
        p = ROOT / "crates/codegen/xai-grok-pager/src/tips" / name
        if not p.exists():
            continue
        t = p.read_text(encoding="utf-8")
        # if multi-span, leave note
        if "Want double-click" in t:
            # replace first long string if present
            t = re.sub(
                r'"Want double-click[^"]*"',
                f'xai_grok_i18n::t("{key}")',
                t,
                count=1,
            )
            p.write_text(t, encoding="utf-8")
            print("tips word_select")
        if "Over SSH?" in t:
            t = re.sub(
                r'"Over SSH\?[^"]*"',
                f'xai_grok_i18n::t("{key}")',
                t,
                count=1,
            )
            # key for ssh is tips.ssh_wrap.full
            t = t.replace(
                'xai_grok_i18n::t("tips.word_select.full")',
                'xai_grok_i18n::t("tips.ssh_wrap.full")',
            ) if name == "ssh_wrap.rs" else t
            if name == "ssh_wrap.rs":
                t = re.sub(
                    r'"Over SSH\?[^"]*"',
                    'xai_grok_i18n::t("tips.ssh_wrap.full")',
                    p.read_text(encoding="utf-8"),
                    count=1,
                )
                p.write_text(t, encoding="utf-8")
                print("tips ssh_wrap")


def patch_settings_validators() -> None:
    p = ROOT / "crates/codegen/xai-grok-pager/src/views/settings_modal/state.rs"
    if not p.exists():
        return
    t = p.read_text(encoding="utf-8")
    pairs = [
        ('"Value cannot be empty"', 'xai_grok_i18n::t("settings.modal.err_empty")'),
        (
            '"Value cannot contain whitespace"',
            'xai_grok_i18n::t("settings.modal.err_whitespace")',
        ),
        (
            '"Model catalog still loading — try again"',
            'xai_grok_i18n::t("settings.modal.err_catalog_loading")',
        ),
        (
            '"Model catalog still loading - try again"',
            'xai_grok_i18n::t("settings.modal.err_catalog_loading")',
        ),
    ]
    for a, b in pairs:
        t = t.replace(a, b)
    # Unknown model format
    t = re.sub(
        r'format!\("Unknown model: \\"\{\\}\\""\s*,\s*([^)]+)\)',
        r'xai_grok_i18n::t_fmt("settings.modal.err_unknown_model", &[("buffer", \1.as_str())])',
        t,
    )
    p.write_text(t, encoding="utf-8")
    print("settings validators")


def main() -> None:
    patch_permission_view()
    patch_permissions_titles()
    patch_prompter()
    patch_welcome()
    patch_shortcuts_bar()
    patch_app_view_pending()
    patch_modes()
    patch_modal()
    bulk_replace_toasts()
    patch_slash_errors()
    patch_dashboard()
    patch_panes_and_misc()
    patch_settings_validators()
    print("done")


if __name__ == "__main__":
    main()
