# Windows 编译成功经验（Grok Build 本地 fork）

本文记录在 **Windows 11 + PowerShell 7** 上从源码成功编出 `xai-grok-pager` release 的完整路径。  
上游 README 写明 Windows 为 *best-effort*；按官方步骤直接 `cargo build` 往往会卡在 **protoc** 与 **MSVC 链接 PDB** 两处。下列步骤已在本机验证通过。

| 项 | 本机成功环境 |
|----|----------------|
| 系统 | Windows 11 |
| Shell | PowerShell 7（pwsh） |
| 仓库路径 | `D:\Projects\grok-build` |
| 分支 | `feature/transparent-bg`（含半透明画布改动） |
| Rust | 由 `rust-toolchain.toml` 拉取，成功时为 **1.92.0** |
| protoc | **29.3**（Windows 官方 zip，非仓库内 DotSlash 包装） |
| 链接器 | **rust-lld**（避免 MSVC `link.exe` LNK1318） |
| 产物 | `target\release\xai-grok-pager.exe`（约 123 MB） |
| 版本输出 | `grok 0.2.105 (...)` |

---

## 1. 前置条件

### 1.1 Rust

安装 [rustup](https://rustup.rs/)，进入仓库后会按 `rust-toolchain.toml` 自动装对应 toolchain。

```powershell
cd D:\Projects\grok-build
rustc --version
```

### 1.2 Visual Studio Build Tools（MSVC）

编译大量 native 依赖（openssl 替代栈、git2、sqlite、tree-sitter 等）需要：

- **Desktop development with C++** 或  
- **MSVC v143** + Windows SDK  

本机使用：`Microsoft Visual Studio 2022 BuildTools`（`link.exe` 路径类似  
`...\VC\Tools\MSVC\14.44.x\bin\HostX64\x64\link.exe`）。

### 1.3 protoc（必须用真实 Windows 二进制）

仓库根目录 `bin/protoc` 是 **DotSlash 脚本**，且 **没有 `windows-*` platform**。  
在 Windows 上直接执行会报类似：

```text
%1 不是有效的 Win32 应用程序。(os error 193)
```

即使 PATH 上已有 protoc，build 脚本仍可能先找到坏掉的 `bin/protoc`，再 fallback 时撞上 Unix 专用路径 `/dev/stdout`。

**推荐做法：固定 `PROTOC` 指向 protoc 29.3 win64：**

```powershell
$dest = "D:\Projects\grok-build\tools\protoc"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
$zip = "$dest\protoc.zip"
Invoke-WebRequest `
  -Uri "https://github.com/protocolbuffers/protobuf/releases/download/v29.3/protoc-29.3-win64.zip" `
  -OutFile $zip
Expand-Archive -Path $zip -DestinationPath $dest -Force

$env:PROTOC = "$dest\bin\protoc.exe"
& $env:PROTOC --version   # 期望: libprotoc 29.3
```

> 说明：winget 装的 Protobuf **35.x** 在本流程里也可被找到，但建议与仓库 DotSlash 其它平台一致使用 **29.3**，减少插件/proto 兼容性差异。

### 1.4 不需要 DotSlash

官方 README 要求 macOS/Linux 装 DotSlash 以跑 `bin/protoc`。  
在 Windows 上用上面的 `PROTOC=` 绝对路径即可，**不必**安装 DotSlash。

---

## 2. 获取源码

```powershell
# 若尚未 clone
git clone https://github.com/xai-org/grok-build.git D:\Projects\grok-build
cd D:\Projects\grok-build

# 本 fork 半透明相关改动在分支 feature/transparent-bg
git checkout feature/transparent-bg
```

---

## 3. 本仓库已包含的 Windows 相关补丁

除「半透明画布」功能外，为能在 Windows 上编过，还动过：

### 3.1 protoc 依赖扫描（`crates/build/xai-proto-build`）

`emit_rerun_if_changed` 原先写死：

```text
--dependency_out=/dev/stdout
--descriptor_set_out=/dev/null
```

Windows 上会失败（`/dev/stdout: No such file or directory`）。  
补丁：Windows 改为临时文件 + `NUL`，并接受 `NUL:` 前缀的 dep 输出。

### 3.2 release 配置（根 `Cargo.toml`）

```toml
[profile.release]
debug = 0
strip = "symbols"
```

减轻调试信息体积；**单独这样仍不够**，还必须换链接器（见下节）。

### 3.3 测试（`link_opener`）

`build_open_path_command` 仅在非 Windows 编译；相关 unit test 已用 `#[cfg(not(target_os = "windows"))]` 限定，避免 Windows 下 `cargo test` 误编失败。

---

## 4. 关键失败与对策（踩坑记录）

| 现象 | 原因 | 对策 |
|------|------|------|
| `bin/protoc` os error 193 | DotSlash 脚本、无 Windows platform | 设 `$env:PROTOC` 为真实 `protoc.exe` |
| `/dev/stdout: No such file` | Unix 路径写进 build.rs | 使用本分支已修的 `xai-proto-build` |
| `LINK : fatal error LNK1318` / `PDB ... LIMIT` | MSVC `link.exe` 对巨型二进制 PDB 有上限；rustc 仍可能带 `/DEBUG` | **`$env:RUSTFLAGS = "-C linker=rust-lld"`** |
| 只改 `debug = 0` / `strip` / `/PDBPAGESIZE` 仍 LNK1318 | 最终链接行里仍有 `/DEBUG` + natvis | 换 **rust-lld**，绕过 MSVC PDB |
| release 编很久（10–20+ 分钟） | 依赖极多，属正常 | 耐心等待；用 SSD、关杀软实时扫描可加速 |
| `cargo test -p xai-grok-pager-render` 在 Windows 报 `build_open_path_command` | 测试未做 cfg | 已修；或只跑 lib 非相关测试 |

**最终确认有效的 release 命令（请整段复制）：**

```powershell
cd D:\Projects\grok-build
$env:PROTOC = "D:\Projects\grok-build\tools\protoc\bin\protoc.exe"
$env:RUSTFLAGS = "-C linker=rust-lld"
cargo build -p xai-grok-pager-bin --release
```

成功时末尾类似：

```text
Finished `release` profile [optimized] target(s) in Xm Ys
```

产物：

```text
D:\Projects\grok-build\target\release\xai-grok-pager.exe
```

验证：

```powershell
.\target\release\xai-grok-pager.exe --version
# 例: grok 0.2.105 (...)
```

---

## 5. 一键脚本（推荐）

均在仓库根目录用 PowerShell 执行（可先 `cd D:\Projects\grok-build`）。

| 脚本 | 作用 |
|------|------|
| `.\scripts\build.ps1` | 一键 release 编译（自动下 protoc 29.3 + rust-lld） |
| `.\scripts\replace.ps1` | 备份官方后，用自建版覆盖 `~\.grok\bin\grok.exe` |
| `.\scripts\replace.ps1 -Build` | 先编译再替换 |
| `.\scripts\restore.ps1` | 还原最近一次 replace 的备份 |
| `.\scripts\restore.ps1 -List` | 列出可用备份 |
| `.\scripts\restore.ps1 -FromOld` | 用官方更新留下的 `grok.exe.old` |
| `.\scripts\build-windows.ps1` | 兼容旧名，等同 `build.ps1` |

```powershell
cd D:\Projects\grok-build

# 1) 编译
.\scripts\build.ps1

# 2) 替换官方 grok（请先关掉所有 grok 窗口）
.\scripts\replace.ps1
# 或一步完成：
# .\scripts\replace.ps1 -Build

# 3) 出问题则还原
.\scripts\restore.ps1
.\scripts\restore.ps1 -List
```

备份目录：`%USERPROFILE%\.grok\bin\backups\grok.exe.bak-*`

注意：`grok update` 会再次写成官方二进制；更新后需再跑 `.\scripts\replace.ps1`（或 `-Build`）。

---

## 6. 安装与日常使用

### 6.1 不覆盖官方 `grok`（可选）

```powershell
Copy-Item D:\Projects\grok-build\target\release\xai-grok-pager.exe `
  $env:USERPROFILE\.local\bin\grok-transparent.exe -Force
# 确保 ~/.local/bin 在 PATH 中
grok-transparent
```

### 6.2 替换官方命令

优先用 **§5 一键脚本** `replace.ps1` / `restore.ps1`。  
手动方式：覆盖 `%USERPROFILE%\.grok\bin\grok.exe`（先备份）。

### 6.3 全屏半透明（本 fork 功能）

编辑 `%USERPROFILE%\.grok\config.toml`：

```toml
[ui]
transparent_bg = true
```

或单次：

```powershell
$env:GROK_TRANSPARENT_BG = "1"
.\target\release\xai-grok-pager.exe
```

要求：

1. 使用 Windows Terminal（或其它支持 Acrylic/Mica）的半透明 profile  
2. 使用 **fullscreen**（不要用 minimal 专为透明，本开关就是为全屏主题准备的）  
3. 必须跑 **带补丁的自建二进制**；官方发行版会忽略 `transparent_bg`

---

## 7. 建议的验证清单

```powershell
# 1) 单元测试（与主题/配置相关）
$env:PROTOC = "D:\Projects\grok-build\tools\protoc\bin\protoc.exe"
cargo test -p xai-grok-shared --lib transparent_bg
cargo test -p xai-grok-pager-render --lib with_transparent_canvas
cargo test -p xai-grok-pager-render --lib set_transparent_bg_is_honored

# 2) Release 构建
$env:RUSTFLAGS = "-C linker=rust-lld"
cargo build -p xai-grok-pager-bin --release

# 3) 启动后目视：全屏空区域应透出终端背景
.\target\release\xai-grok-pager.exe
```

---

## 8. 不纳入版本库的本地文件

以下为本地构建产物 / 工具，一般 **不要 commit**：

| 路径 | 说明 |
|------|------|
| `tools/protoc/` | 本机下载的 protoc 29.3 |
| `target/` | cargo 输出 |
| `build-*.log` | 调试日志 |
| `crates/**/NUL` | Windows 下 protoc 误生成的空设备文件，可删 |

可将 `tools/protoc/` 加入个人 `.gitignore`，或在文档中要求每人自备 protoc。

---

## 9. 经验摘要（最短路径）

1. **`PROTOC` = 真实 win64 protoc 29.3 绝对路径**（别依赖 `bin/protoc` DotSlash）  
2. **`RUSTFLAGS = -C linker=rust-lld`**（别用默认 MSVC link 打 release）  
3. 用本分支的 **xai-proto-build Windows 修复**  
4. `cargo build -p xai-grok-pager-bin --release`  
5. 跑 `target\release\xai-grok-pager.exe`，并设置 `transparent_bg = true` 测半透明  

按上述四条，本机可在约 **10 分钟级**（热缓存会更快）完成 release 链接并得到可运行二进制。

---

## 10. 相关代码入口（半透明）

| 文件 | 作用 |
|------|------|
| `crates/codegen/xai-grok-shared/src/ui_config.rs` | `[ui].transparent_bg` + `GROK_TRANSPARENT_BG` |
| `crates/codegen/xai-grok-pager-render/src/theme/cache.rs` | 缓存开关，避免每帧读盘 |
| `crates/codegen/xai-grok-pager-render/src/theme/mod.rs` | `Theme::current()` / `with_transparent_canvas()` |
| `crates/codegen/xai-grok-pager/src/views/agent.rs` | `fill_background` 使用 `theme.bg_base` |
| `crates/build/xai-proto-build/src/lib.rs` | Windows protoc dep 扫描修复 |

功能提交示例：`feat(ui): transparent canvas for fullscreen Acrylic/Mica`（分支 `feature/transparent-bg`）。
