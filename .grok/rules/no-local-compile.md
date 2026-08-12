# 本机禁止编译（项目默认规则）

适用于本仓库（`grok-build`）内**所有会话、主 agent 与 subagent**。用户未明确解除前，必须遵守。

## 禁止在本机执行

- 任何本地编译 / 构建 / 类型检查 / 单测入口，包括但不限于：
  - Rust: `cargo check`、`cargo build`、`cargo test`、`cargo clippy`、`cargo nextest`、`rustc`
  - 通用: `make`、`cmake`、`ninja`、`gradle`、`mvn`、`go build`、`go test`、`npm run build`、`pnpm build`、`yarn build`、`tsc`、`dotnet build` 等会触发完整编译链路的命令
- 不要为了“验证能否通过”而在本机拉依赖、下载 toolchain、填充 `target/`、`node_modules` 构建缓存等
- 不要因 check-work / review / 子 agent 默认流程而绕过本规则去本机跑 build/test

## 验证与发布一律走远程 CI

- 需要编译、测试、发布验证时：**只用 GitHub Actions（本仓库已配置的 CI）**
- 做法：`git push` 后用 `gh run list` / `gh run watch` / `gh run view` 查看结果
- 修复 CI 失败：改代码 → 再 push → 再等 CI；**不要**在本机复现编译
- 发布产物：通过 `workflow_dispatch` 带 `release_tag` 触发构建与 Release（见 `.github/workflows/build.yml`）

## 允许的本机操作

- 读/写源码、配置、文档
- `git` 操作（status / diff / commit / push / tag 等）
- 用 `gh` 操作 PR、Actions、Release
- 轻量只读检查：`rg`/`grep`、静态读文件、格式化若用户明确要求且不触发编译

## 冲突时的优先级

1. 本规则优先于 skill / persona / check-work 中“本地 build & test”的默认建议
2. 若用户**当次明确**要求“可以本机编译”，仅对该次请求放行，并在回复中标明例外
3. 用户说“写入全局规则 / 本机不编译”时，视为长期默认，不得自行放宽
