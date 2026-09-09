# 本机编译与 CI 等价验证

适用于本仓库（`grok-build`）内所有会话、主 agent 与 subagent。

## 本机验证策略

本机编译、构建、类型检查和测试均允许执行；修改代码后应优先在本机完成与 CI 等价的验证，再推送。不得为了“通过检查”而降低命令强度、移除 `--locked` 或跳过实际构建。

## 与 CI 一致的 pager 验证

`.github/workflows/build.yml` 当前对 Linux 和 Windows 执行以下核心步骤：

- Rust 工具链使用 `1.92.0`；本机使用 `cargo +1.92.0`，不要依赖 `rust-toolchain.toml` 中的其它默认版本。
- Protoc 使用 `29.3`，通过 `PROTOC` 指向实际的 `protoc` 可执行文件。
- 所有 Cargo 命令使用 `--locked`。
- 设置与当前构建或发布一致的 `GROK_VERSION`。
- 执行 `cargo +1.92.0 check --locked -p xai-grok-pager-bin`。
- Linux 执行 `cargo +1.92.0 test --locked -p xai-grok-sampler --lib`。
- 执行 `cargo +1.92.0 build --locked -p xai-grok-pager-bin --release`。
- 运行 `target/release/xai-grok-pager --version`，确认输出包含 `GROK_VERSION`。

Linux Bash 示例：

```bash
PATH=/root/.cargo/bin:$PATH \
PROTOC=/path/to/protoc-29.3/bin/protoc \
GROK_VERSION=1.0.24-fork.2 \
CARGO_BUILD_JOBS=4 \
cargo +1.92.0 check --locked -p xai-grok-pager-bin

PATH=/root/.cargo/bin:$PATH \
PROTOC=/path/to/protoc-29.3/bin/protoc \
GROK_VERSION=1.0.24-fork.2 \
CARGO_BUILD_JOBS=4 \
cargo +1.92.0 test --locked -p xai-grok-sampler --lib

PATH=/root/.cargo/bin:$PATH \
PROTOC=/path/to/protoc-29.3/bin/protoc \
GROK_VERSION=1.0.24-fork.2 \
CARGO_BUILD_JOBS=4 \
cargo +1.92.0 build --locked -p xai-grok-pager-bin --release

target/release/xai-grok-pager --version
```

Windows 本机验证应使用 CI 的 linker 设置：

```powershell
$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = "rust-lld"
$env:GROK_VERSION = "1.0.24-fork.2"
$env:PROTOC = "C:\path\to\protoc-29.3\bin\protoc.exe"
cargo +1.92.0 check --locked -p xai-grok-pager-bin
cargo +1.92.0 build --locked -p xai-grok-pager-bin --release
```

## 额外验证

修改翻译目录或国际化代码时，额外运行相关 crate 的测试，例如：

```bash
cargo +1.92.0 test --locked -p xai-grok-i18n --lib
```

国际化或用户界面修改还必须运行：

```bash
cargo +1.92.0 test --locked -p xai-grok-i18n --test i18n_audit
```

该集成测试会无条件执行当前源码和 Markdown 覆盖审计；diff/upstream 审计在提供对应基线时也会执行。

如果本机无法复现某个平台的环境，必须明确记录未验证的项目，并在推送后检查对应的 GitHub Actions 作业；这不是跳过可执行本机验证的默认理由。

## 推送与发布

- 代码修改在适用的本机检查和实际构建通过前不得推送。
- 推送后使用 `gh run list`、`gh run view` 或 `gh run watch` 检查远程 CI。
- 发布前必须确认远程 CI 成功，且 `crates/codegen/xai-grok-shell/CHANGELOG.md` 已随发布源提交并推送。
- 正式发布使用 `.github/workflows/build.yml` 的 `workflow_dispatch` 和 `release_tag` 参数。
