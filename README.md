# Grok Build — 社区 Fork（liansishen）

本仓库基于 [SpaceXAI / Grok Build](https://x.ai/cli) 上游源码的社区 fork，在官方能力之上做了面向中文与 Windows 使用的增强。上游仍会不定期同步 monorepo 变更。

**官方安装：** 见下方 [Installing the released binary](#installing-the-released-binary)。  
**本 fork 本地构建与替换：** 见 [Windows 一键构建与替换](#windows-一键构建与替换)。

---

## 本 Fork 额外实现的功能

### 1. 界面中文（i18n / zh-CN）

- 新增 `xai-grok-i18n` 语言包（`en` / `zh-CN`），设置中可切换 **界面语言**（跟随系统 / English / 简体中文）。
- 覆盖高影响界面：设置、权限提示、命令面板、toast、快捷键帮助、仪表盘、底栏 hint、`/usage` 等。
- 模式相关：Shift+Tab 横幅（`已切换模式：…`）、输入框角标（计划 / 自动 / 始终批准）、回合标记（`工作了 …`）、空提示占位等。

配置示例：

```toml
[ui]
language = "zh-CN"   # auto | en | zh-CN
```

### 2. 全屏透明背景（Acrylic / Mica）

- 设置项 **透明背景**：全屏主题画布使用终端背景（`Color::Reset`），便于 Windows Terminal Acrylic / Mica 等透出。
- 悬停、选区等抬升层仍使用主题色。

配置示例：

```toml
[ui]
transparent_bg = true
```

环境变量：`GROK_TRANSPARENT_BG=1`。

### 3. 提示栏用量状态（周/月限额 + 重置时间）

- 在提示框信息行、**模型名称左侧**显示紧凑用量，例如：  
  `每周限额: 45% · 重置 Mar 31, 12:00 · Grok 4.5 (high) · 始终批准`
- 数据来自现有 billing / credit API（`usage_pct`、周期类型、`period_end_display`）。
- **刷新策略**
  - 默认每 **5 分钟** 静默拉取一次用量；
  - **回合结束**时也会立即刷新（原有逻辑）；
  - 间隔可在设置中调整。

配置示例：

```toml
[ui]
usage_refresh_interval_minutes = 5   # 1–60，默认 5
```

设置 UI 中对应项：**用量刷新间隔** / Usage refresh interval。

### 4. Windows 一键构建与替换

仓库 `scripts/` 下提供本地开发辅助脚本（见脚本内注释）：

| 脚本 | 作用 |
|------|------|
| `scripts/build.ps1` | 编译 release 产物 |
| `scripts/replace.ps1` | 用自建二进制替换 `%USERPROFILE%\.grok\bin\grok.exe`（先备份） |
| `scripts/restore.ps1` | 从备份恢复官方/上一版二进制 |

```powershell
# 在仓库根目录
.\scripts\replace.ps1 -Build    # 编译并替换安装目录中的 grok.exe
.\scripts\restore.ps1           # 恢复备份
```

> **注意：** 官方 **自动更新** 默认开启，会覆盖你替换的 `grok.exe`。长期使用自建版请在设置中关闭自动更新，或在配置中写入：
>
> ```toml
> [cli]
> auto_update = false
> ```

### 5. 其它相关说明

| 主题 | 说明 |
|------|------|
| 鼠标选字 | 应用内选区与终端原生选区并存；按住 **Shift 拖动** 可走终端原生选区。 |
| 关闭代码库索引 | `[features] codebase_indexing = false` |
| 关闭首轮记忆注入 | `[memory.initial_injection] enabled = false` |

---

## Windows 一键构建与替换

```powershell
cd <本仓库路径>
.\scripts\build.ps1                 # 仅编译
.\scripts\replace.ps1 -Build        # 编译并替换已安装的 grok
grok --version                      # 确认当前 PATH 上的版本
```

安装路径固定为：`%USERPROFILE%\.grok\bin\grok.exe`。

---

## 以下是原内容

> 以下内容保留自上游 README，便于对照官方说明与仓库结构。

<div align="center">

<h1>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://media.x.ai/v1/website/spacexai-symbol-white-transparent-0c31957f.png">
    <source media="(prefers-color-scheme: light)" srcset="https://media.x.ai/v1/website/spacexai-symbol-black-transparent-6435cf42.png">
    <img alt="SpaceXAI logo" src="https://media.x.ai/v1/website/spacexai-symbol-black-transparent-6435cf42.png" width="96">
  </picture>
  <br>
  Grok Build (<code>grok</code>)
</h1>

**Grok Build** is SpaceXAI's terminal-based AI coding agent. It runs as a
full-screen TUI that understands your codebase, edits files, executes shell
commands, searches the web, and manages long-running tasks — interactively,
headlessly for scripting/CI, or embedded in editors via the Agent Client
Protocol (ACP).

[Installing the released binary](#installing-the-released-binary) ·
[Building from source](#building-from-source) ·
[Documentation](#documentation) ·
[Repository layout](#repository-layout) ·
[Development](#development) ·
[Contributing](#contributing) ·
[License](#license)

![Grok Build TUI](https://media.x.ai/v1/website/universe-tui-screenshot-6f7a0837.png)

**Learn more about Grok Build at [x.ai/cli](https://x.ai/cli)**

This repository contains the Rust source for the `grok` CLI/TUI and its agent
runtime. It is synced periodically from the SpaceXAI monorepo.

A small `SOURCE_REV` file at the root records the full monorepo commit SHA
for the version of the code present in this tree.

</div>

---

## Installing the released binary

Prebuilt binaries are published for macOS, Linux, and Windows:

```sh
curl -fsSL https://x.ai/cli/install.sh | bash   # macOS / Linux / Git Bash
irm https://x.ai/cli/install.ps1 | iex          # Windows PowerShell
grok --version
```

See the [changelog](https://x.ai/build/changelog) for the latest fixes,
features, and improvements in each release.

## Building from source

Requirements:

- **Rust** — the toolchain is pinned by [`rust-toolchain.toml`](rust-toolchain.toml);
  `rustup` installs it automatically on first build.
- **[DotSlash](https://dotslash-cli.com)** — required so hermetic tools under
  [`bin/`](bin/) (notably [`bin/protoc`](bin/protoc)) can download and run.
  Install it and ensure `dotslash` is on your `PATH` **before** building:

  ```sh
  cargo install dotslash
  # or: prebuilt packages — https://dotslash-cli.com/docs/installation/
  /usr/bin/env dotslash --help   # sanity check
  ```

- **protoc** — proto codegen resolves [`bin/protoc`](bin/protoc) via DotSlash,
  or falls back to a `protoc` on `PATH` / `$PROTOC`.
- macOS and Linux are supported build hosts; Windows builds are best-effort
  and not currently tested from this tree.

```sh
cargo run -p xai-grok-pager-bin              # build + launch the TUI
cargo build -p xai-grok-pager-bin --release  # release binary: target/release/xai-grok-pager
cargo check -p xai-grok-pager-bin            # fast validation
```

The binary artifact is named `xai-grok-pager`; official installs ship it as
`grok`. On first launch it opens your browser to authenticate — see the
[authentication guide](crates/codegen/xai-grok-pager/docs/user-guide/02-authentication.md).

## Documentation

Full online documentation is available at
[docs.x.ai/build/overview](https://docs.x.ai/build/overview).

The user guide ships with the pager crate:
[`crates/codegen/xai-grok-pager/docs/user-guide/`](crates/codegen/xai-grok-pager/docs/user-guide/)
— getting started, keyboard shortcuts, slash commands, configuration, theming,
MCP servers, skills, plugins, hooks, headless mode, sandboxing, and more.

## Repository layout

| Path | Contents |
|------|----------|
| `crates/codegen/xai-grok-pager-bin` | Composition-root package; builds the `xai-grok-pager` binary |
| `crates/codegen/xai-grok-pager` | The TUI: scrollback, prompt, modals, rendering |
| `crates/codegen/xai-grok-shell` | Agent runtime + leader/stdio/headless entry points |
| `crates/codegen/xai-grok-tools` | Tool implementations (terminal, file edit, search, ...) |
| `crates/codegen/xai-grok-workspace` | Host filesystem, VCS, execution, checkpoints |
| `crates/codegen/...` | The rest of the CLI crate closure (config, MCP, markdown, sandbox, ...) |
| `crates/common/`, `crates/build/`, `prod/mc/` | Small shared leaf crates pulled in by the closure |
| `third_party/` | Vendored upstream source (Mermaid diagram stack) — see below |

> [!IMPORTANT]
> The root `Cargo.toml` (workspace members, dependency versions, lints,
> profiles) is **generated** — treat it as read-only. Prefer editing per-crate
> `Cargo.toml` files.

## Development

```sh
cargo check -p <crate>        # always target specific crates; full-workspace builds are slow
cargo test -p xai-grok-config # per-crate tests
cargo clippy -p <crate>       # lint config: clippy.toml at the repo root
cargo fmt --all               # rustfmt.toml at the repo root
```

## Contributing

> [!NOTE]
> External contributions are not accepted. See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

First-party code in this repository is licensed under the **Apache License,
Version 2.0** — see [`LICENSE`](LICENSE).

Third-party and vendored code remains under its original licenses. See:

- [`THIRD-PARTY-NOTICES`](THIRD-PARTY-NOTICES) — crates.io / git dependencies,
  bundled UI themes, and **in-tree source ports** (including openai/codex and
  sst/opencode tool implementations)
- [`crates/codegen/xai-grok-tools/THIRD_PARTY_NOTICES.md`](crates/codegen/xai-grok-tools/THIRD_PARTY_NOTICES.md)
  — crate-local notice for the codex and opencode ports (license texts +
  Apache §4(b) change notice)
- [`third_party/NOTICE`](third_party/NOTICE) — vendored Mermaid-stack index
