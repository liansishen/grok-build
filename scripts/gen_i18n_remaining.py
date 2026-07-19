#!/usr/bin/env python3
"""Append Phase 2–4 catalog keys to en.toml / zh-CN.toml (merge, no wipe)."""

from __future__ import annotations

import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EN_PATH = ROOT / "crates/codegen/xai-grok-i18n/locales/en.toml"
ZH_PATH = ROOT / "crates/codegen/xai-grok-i18n/locales/zh-CN.toml"


def esc(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def flat_section(d: dict[str, str]) -> str:
    tables: dict[tuple[str, ...], dict[str, str]] = {}
    for k, v in d.items():
        *path, last = k.split(".")
        tables.setdefault(tuple(path), {})[last] = v
    out: list[str] = []
    for path, fields in sorted(tables.items(), key=lambda x: x[0]):
        out.append(f"[{'.'.join(path)}]")
        for fk, fv in fields.items():
            out.append(f'{fk} = "{esc(fv)}"')
        out.append("")
    return "\n".join(out)


def existing_keys(path: Path) -> set[str]:
    data = tomllib.loads(path.read_text(encoding="utf-8"))

    def walk(node, prefix=""):
        keys = set()
        if isinstance(node, dict):
            for k, v in node.items():
                p = f"{prefix}.{k}" if prefix else k
                if isinstance(v, dict):
                    keys |= walk(v, p)
                else:
                    keys.add(p)
        return keys

    return walk(data)


EN = {
    # Settings modal footers / chrome
    "settings.modal.footer.nav_jk": "↑/↓/j/k nav",
    "settings.modal.footer.top_bottom": "g/G top/btm",
    "settings.modal.footer.space_toggle": "Space toggle",
    "settings.modal.footer.enter_toggle": "Enter toggle",
    "settings.modal.footer.enter_edit": "Enter edit",
    "settings.modal.footer.expand": "→ expand",
    "settings.modal.footer.slash_search": "/ search",
    "settings.modal.footer.reset": "d reset",
    "settings.modal.footer.close": "F2/Esc close",
    "settings.modal.footer.type_to_filter": "type to filter",
    "settings.modal.footer.nav": "↑/↓ nav",
    "settings.modal.footer.backspace_edit": "Backspace edit",
    "settings.modal.footer.enter_commit": "Enter commit",
    "settings.modal.footer.esc_clear": "Esc clear",
    "settings.modal.footer.nav_try": "↑/↓ try",
    "settings.modal.footer.esc_revert": "Esc revert",
    "settings.modal.footer.esc_cancel": "Esc cancel",
    "settings.modal.footer.y_reset": "y reset",
    "settings.modal.footer.n_cancel": "n cancel",
    "settings.modal.footer.f2_cancel": "F2 cancel",
    "settings.modal.footer.type_to_edit": "type to edit",
    "settings.modal.tip_long": 'Tip · Ask Grok: "change theme to grokday" or "what does compact mode do?"',
    "settings.modal.tip_short": "Tip · Ask Grok to change a setting",
    "settings.modal.no_matches_for": "No matches for",
    "settings.modal.value_on": "on",
    "settings.modal.value_off": "off",
    "settings.modal.restart_pill": "restart",
    "toast.check": "✓",
    "toast.setting_changed": "✓ {label}: {value}",
    "toast.already_default": "{label}: already at default",
    "toast.mouse_reporting_on": "Mouse reporting on",
    "toast.copied": "Copied!",
    "toast.no_active_session": "No active session",
    "toast.mermaid": "✓ Mermaid: {value}",
    "toast.theme": "✓ Theme: {name}",
    # Shortcuts help categories
    "shortcuts.category.essentials": "Essentials",
    "shortcuts.category.input": "Input",
    "shortcuts.category.conversation_nav": "Conversation Navigation",
    "shortcuts.category.conversation_action": "Conversation Actions",
    "shortcuts.category.panels": "Panels",
    "shortcuts.category.session": "Session",
    "shortcuts.category.dashboard": "Dashboard",
    # Tool prefixes
    "tool.prefix.list": "List ",
    "tool.prefix.edit": "Edit ",
    "tool.prefix.creating": "Creating ",
    "tool.prefix.read": "Read ",
    "tool.prefix.run": "Run ",
    "tool.prefix.search": "Search ",
    "tool.prefix.fetch": "Fetch ",
    "tool.prefix.memory_search": "Memory Search ",
    "tool.prefix.web_search": "Web search ",
    "tool.prefix.web_fetch": "Web fetch ",
    # Status / empty
    "status.queue_empty": "Queue is empty.",
    "status.no_tasks": "No background tasks.",
    "dashboard.empty": "No agents yet",
    "dashboard.title": "Agent Dashboard",
    # Docs titles (phase 4 — titles/descriptions only)
    "docs.01.title": "Getting Started",
    "docs.01.desc": "Installation, first launch, and basic interaction",
    "docs.02.title": "Authentication",
    "docs.02.desc": "Browser login, API keys, OIDC, external auth providers",
    "docs.03.title": "Keyboard Shortcuts",
    "docs.03.desc": "Complete reference for all TUI key bindings",
    "docs.04.title": "Slash Commands",
    "docs.04.desc": "All / commands for session management, models, memory, hooks",
    "docs.05.title": "Configuration",
    "docs.05.desc": "config.toml, pager.toml, environment variables, file locations",
    "docs.06.title": "Theming and Appearance",
    "docs.06.desc": "Themes, color support, pager.toml customization",
    "docs.07.title": "MCP Servers",
    "docs.07.desc": "Setting up external tool integrations via MCP",
    "docs.08.title": "Skills",
    "docs.08.desc": "Creating and using reusable prompt packages",
    "docs.09.title": "Plugins and Marketplace",
    "docs.09.desc": "Installing, managing, and creating plugin packages",
    "docs.10.title": "Hooks",
    "docs.10.desc": "Project lifecycle scripts for pre/post tool-use events",
    "docs.11.title": "Custom Models",
    "docs.11.desc": "BYOK, Ollama, OpenAI-compatible endpoints",
    "docs.12.title": "Project Rules (AGENTS.md)",
    "docs.12.desc": "Per-directory instructions and precedence rules",
    "docs.13.title": "Memory",
    "docs.13.desc": "Cross-session knowledge persistence and search",
    "docs.14.title": "Headless Mode and Scripting",
    "docs.14.desc": "Non-interactive CLI for automation and CI/CD",
    "docs.15.title": "Agent Mode and IDE Integration",
    "docs.15.desc": "ACP stdio transport, WebSocket relay, SDK integration",
}

ZH = {
    "settings.modal.footer.nav_jk": "↑/↓/j/k 导航",
    "settings.modal.footer.top_bottom": "g/G 顶/底",
    "settings.modal.footer.space_toggle": "Space 切换",
    "settings.modal.footer.enter_toggle": "Enter 切换",
    "settings.modal.footer.enter_edit": "Enter 编辑",
    "settings.modal.footer.expand": "→ 展开",
    "settings.modal.footer.slash_search": "/ 搜索",
    "settings.modal.footer.reset": "d 重置",
    "settings.modal.footer.close": "F2/Esc 关闭",
    "settings.modal.footer.type_to_filter": "输入筛选",
    "settings.modal.footer.nav": "↑/↓ 导航",
    "settings.modal.footer.backspace_edit": "Backspace 编辑",
    "settings.modal.footer.enter_commit": "Enter 确认",
    "settings.modal.footer.esc_clear": "Esc 清除",
    "settings.modal.footer.nav_try": "↑/↓ 预览",
    "settings.modal.footer.esc_revert": "Esc 还原",
    "settings.modal.footer.esc_cancel": "Esc 取消",
    "settings.modal.footer.y_reset": "y 重置",
    "settings.modal.footer.n_cancel": "n 取消",
    "settings.modal.footer.f2_cancel": "F2 取消",
    "settings.modal.footer.type_to_edit": "输入编辑",
    "settings.modal.tip_long": "提示 · 问 Grok：「把主题改成 grokday」或「紧凑模式是什么？」",
    "settings.modal.tip_short": "提示 · 让 Grok 帮你改设置",
    "settings.modal.no_matches_for": "无匹配",
    "settings.modal.value_on": "开",
    "settings.modal.value_off": "关",
    "settings.modal.restart_pill": "需重启",
    "toast.check": "✓",
    "toast.setting_changed": "✓ {label}: {value}",
    "toast.already_default": "{label}: 已是默认值",
    "toast.mouse_reporting_on": "鼠标报告已开启",
    "toast.copied": "已复制！",
    "toast.no_active_session": "无活动会话",
    "toast.mermaid": "✓ Mermaid: {value}",
    "toast.theme": "✓ 主题: {name}",
    "shortcuts.category.essentials": "常用",
    "shortcuts.category.input": "输入",
    "shortcuts.category.conversation_nav": "对话导航",
    "shortcuts.category.conversation_action": "对话操作",
    "shortcuts.category.panels": "面板",
    "shortcuts.category.session": "会话",
    "shortcuts.category.dashboard": "仪表盘",
    "tool.prefix.list": "列表 ",
    "tool.prefix.edit": "编辑 ",
    "tool.prefix.creating": "创建 ",
    "tool.prefix.read": "读取 ",
    "tool.prefix.run": "运行 ",
    "tool.prefix.search": "搜索 ",
    "tool.prefix.fetch": "获取 ",
    "tool.prefix.memory_search": "记忆搜索 ",
    "tool.prefix.web_search": "网页搜索 ",
    "tool.prefix.web_fetch": "网页获取 ",
    "status.queue_empty": "队列为空。",
    "status.no_tasks": "无后台任务。",
    "dashboard.empty": "暂无智能体",
    "dashboard.title": "智能体仪表盘",
    "docs.01.title": "入门",
    "docs.01.desc": "安装、首次启动与基本交互",
    "docs.02.title": "身份验证",
    "docs.02.desc": "浏览器登录、API 密钥、OIDC 与外部认证",
    "docs.03.title": "键盘快捷键",
    "docs.03.desc": "TUI 全部快捷键参考",
    "docs.04.title": "斜杠命令",
    "docs.04.desc": "会话、模型、记忆、hooks 等 / 命令",
    "docs.05.title": "配置",
    "docs.05.desc": "config.toml、pager.toml、环境变量与路径",
    "docs.06.title": "主题与外观",
    "docs.06.desc": "主题、颜色支持与 pager.toml 自定义",
    "docs.07.title": "MCP 服务器",
    "docs.07.desc": "通过 MCP 接入外部工具",
    "docs.08.title": "技能",
    "docs.08.desc": "创建与使用可复用提示包",
    "docs.09.title": "插件与市场",
    "docs.09.desc": "安装、管理与创建插件包",
    "docs.10.title": "Hooks",
    "docs.10.desc": "工具调用前后的项目生命周期脚本",
    "docs.11.title": "自定义模型",
    "docs.11.desc": "BYOK、Ollama、OpenAI 兼容端点",
    "docs.12.title": "项目规则（AGENTS.md）",
    "docs.12.desc": "按目录指令与优先级",
    "docs.13.title": "记忆",
    "docs.13.desc": "跨会话知识持久化与搜索",
    "docs.14.title": "无头模式与脚本",
    "docs.14.desc": "自动化与 CI/CD 的非交互 CLI",
    "docs.15.title": "智能体模式与 IDE 集成",
    "docs.15.desc": "ACP stdio、WebSocket 中继与 SDK 集成",
}


def append_missing(path: Path, new_map: dict[str, str], banner: str) -> int:
    have = existing_keys(path)
    missing = {k: v for k, v in new_map.items() if k not in have}
    if not missing:
        print(f"{path.name}: no new keys")
        return 0
    text = path.read_text(encoding="utf-8")
    if not text.endswith("\n"):
        text += "\n"
    text += f"\n# {banner}\n"
    text += flat_section(missing)
    path.write_text(text, encoding="utf-8")
    print(f"{path.name}: +{len(missing)} keys")
    return len(missing)


def main() -> None:
    append_missing(EN_PATH, EN, "Phase 2–4 remaining surfaces")
    append_missing(ZH_PATH, ZH, "Phase 2–4 remaining surfaces")
    # validate parse
    tomllib.loads(EN_PATH.read_text(encoding="utf-8"))
    tomllib.loads(ZH_PATH.read_text(encoding="utf-8"))
    print("toml ok")


if __name__ == "__main__":
    main()
