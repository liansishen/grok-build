# grok clone

`grok clone` 会将 Git 仓库提取到 Grove 内容存储中，并挂载投影工作树（macOS 使用 NFS，Linux 使用 FUSE）。每次调用都会先读取当前进程中的 `GROK_CLONE` / `GROVE_CLONE`，然后读取 grok 的全局启用设置（`~/.grok/config.toml` 中的 `GROK_GROVE` 或 `[cli] grove`），最后读取 `[clone] enabled`，以授权 Clone IPC。

这不会为会话 / `-w` 工作树启用 Grove。它们使用独立的开关（`~/.grok/config.toml` 中的 `GROK_WORKTREE_TYPE` 和 `[cli] grove_worktree`；参见[配置参考](26-config-reference.md)）。`GROK_WORKTREE_TYPE` 和 `[cli] grove_worktree` 不会启用 `grok clone`。`GROK_CLONE` / `GROVE_CLONE` / `[clone] enabled` 也不会启用会话 / `-w` Grove。

若不改动具体开关即可同时打开两个界面：

```bash
export GROK_GROVE=1
# or in ~/.grok/config.toml:
# [cli]
# grove = true
```

具体开关仍然优先：`GROK_WORKTREE_TYPE=copy` 会让会话工作树保持 copy，同时允许 clone；`GROK_CLONE=0` 会让 `grok clone` 保持关闭，同时工作树仍可启用。

```bash
grok clone <url> [dir] [--branch NAME] [--cone PATH]... [--full-history]
```

## 历史记录

**默认是所选分支的深度为 1 的引导克隆**（`blob:none` + `--depth=1`）。只有该分支会作为远程跟踪引用公布。

需要完整提交历史、标签或克隆时的全部远程分支时，请使用 `--full-history`（这曾经是默认行为）。

深度为 1 的克隆完成后，以下命令只会加深**所选分支**：

```bash
git fetch --deepen=N origin
git fetch --unshallow origin
```

提取其他分支需要显式指定限制深度的 refspec。普通的 `git fetch origin` 或 `git fetch origin other` 不会通过默认 refspec 拉取该分支的完整历史：

```bash
git fetch --depth=1 origin refs/heads/NAME:refs/remotes/origin/NAME
```

默认浅克隆需要 Grove 守护进程支持 `clone_shallow` RPC。如果客户端拒绝，请重启或更新守护进程（或传入 `--full-history`）：

```bash
```

在 macOS 上也可以安装 KeepAlive agent：

```bash
```

## 身份验证

两者属于彼此独立的世界：

| 世界 | 覆盖范围 | 命令 | 存储位置 |
|-------|--------|----------|-------|
| Grok | 模型和 API | `grok login`、`grok logout` | `~/.grok/auth.json` |

`grok clone` 从不读取 `~/.grok/auth.json` 获取 Git 凭据。登录 Grok 不会为远程仓库向守护进程提供凭据，[clone] enabled = true 也不会；该标志只是决定是否运行 `grok clone` 的产品开关，而不是 GitHub 授权。

当 Grove 将失败归类为凭据问题时，克隆会打印类别及负责该类别的命令，但不会打印远程 URL：

```
Grove Git credentials rejected (unavailable).
Check `grove status` then `grove reload-credentials`.
```

| 类别 | 含义 | 下一步 |
|-------|---------|-----------|
| `unavailable` | 守护进程没有可用凭据，或远程仓库拒绝了凭据 | `grove status` 会显示当前提供方；修复该来源后运行 `grove reload-credentials` |
| `expired-static` | 令牌已过期，且此部署不会刷新令牌 | 开始新会话，或启用 `GROVE_TOKEN_ROTATION=expected` |
| `carrier-stale` | 守护进程等待承载令牌改写后仍持有被拒绝的令牌 | 等待改写，或运行 `grove reload-credentials` |
| `other` | 凭据提供方因其他原因失败 | 运行 `grove status`，然后运行 `grove reload-credentials` |

只有 `expired-static` 和 `carrier-stale` 会在消息中添加专属行。`unavailable` 的建议取决于守护进程持有的提供方，而克隆无法读取该信息，因此会保持使用 `grove status` 和 `grove reload-credentials` 的建议。

有一种凭据拒绝无法分类：令牌无法访问私有仓库时，GitHub 会像仓库不存在一样响应。这与公共 URL 拼写错误无法区分，因此克隆会报告缺少仓库，而不会猜测是凭据问题。

即使没有挂载点，`grove status` 也会打印守护进程范围的身份验证块：

```
  auth: mode=auto live=git-delegate health=ok last=- reload=supported
        hint=credentials look healthy
```

`mode` 是配置的 `auth_mode`；`live` 是守护进程实际持有的提供方。两者可能不一致——这正是 `grove reload-credentials` 要修复的情况。例如，`auth_mode = auto` 时，如果守护进程在写入令牌文件前启动，它会一直使用 `git-delegate`，直到凭据单元重建。

```bash
grove reload-credentials
# grove: credentials reloaded → token-file
```

重新加载会从环境和磁盘重建凭据单元，并打印最终提供方；不会打印任何秘密。它不能创建登录：使用 `auto` 或 `git` 且没有承载令牌时，请先配置 `git credential` 或 `gh auth`，再重新加载。

`grove doctor` 会将相同字段报告为一个 finding：`auth.ok`、`auth.degraded`、`auth.unavailable`、`auth.daemon-down` 或 `auth.old-daemon`。

## 守护进程

`grok clone` 使用正在运行的 Grove 守护进程。如果控制套接字关闭，它会以分离进程启动 `grove daemon --foreground`（因此退出或按 Ctrl-C 结束 `grok` 不会停止守护进程或其挂载点），然后等待套接字。

`grove` 二进制文件先从 `PATH` 查找，再从 `grok` 可执行文件所在目录查找（例如与 `grok` 同目录的 `~/.grok/bin/grove`）。不存在单独的安装位置。macOS 没有 Grove 的 PATH 安装包；请从 monorepo 构建：

```bash
cargo build -p grove --release
```

在 Linux 上，clone 在启动守护进程前需要可用的 FUSE：`/dev/fuse` 必须存在，并且当前用户可以打开它，或 `PATH` 中有 setuid 的 `fusermount3` / `fusermount`（Grove 会通过任一方式挂载）。缺少 FUSE 会立即报错并给出安装命令，而不是挂起。守护进程已运行时会跳过检查，因为该守护进程可能拥有当前进程没有的权限。

在 Windows 上，clone 通过 ProjFS（Windows 投影文件系统）挂载，无论该功能是否启用。`target`、`node_modules` 等构建产物目录会以 NTFS 联接（junction）重定向出投影树（`[redirects] windows_junction`，默认开启）。
