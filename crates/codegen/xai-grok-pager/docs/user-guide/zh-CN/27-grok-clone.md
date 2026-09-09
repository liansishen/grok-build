# grok clone

`grok clone` 会将 Git 仓库提取到 Grove 内容存储中，并挂载一个投影工作树
（macOS 使用 NFS，Linux 使用 FUSE）。使用前必须在 Grove 配置
（`~/.config/grove/config.toml`）中启用 `[clone] enabled = true`。

```bash
grok clone <url> [dir] [--branch NAME] [--cone PATH]... [--full-history]
```

## 历史记录

默认会对所选分支执行**深度为 1 的初始克隆**（`blob:none` +
`--depth=1`）。只有该分支会被公布为远程跟踪引用。

如果克隆时需要完整提交历史、标签或全部远程分支，请使用
`--full-history`（这是之前的默认行为）。

完成深度为 1 的克隆后，以下命令只会加深**所选分支**：

```bash
git fetch --deepen=N origin
git fetch --unshallow origin
```

提取其他分支时，需要显式指定限制深度的 refspec。普通的
`git fetch origin` 或 `git fetch origin other` 不会通过默认 refspec
拉取该分支的完整历史：

```bash
git fetch --depth=1 origin refs/heads/NAME:refs/remotes/origin/NAME
```

默认浅克隆需要 Grove 守护进程支持 `clone_shallow` RPC。如果客户端拒绝，
请重启或更新守护进程（也可以改用 `--full-history`）。


## 身份验证

克隆所需的 Git 凭据属于 **Grove 守护进程**，而不是 `grok login`。两者彼此独立：

| 世界 | 覆盖范围 | 命令 | 存储位置 |
|---|---|---|---|
| Grok | 模型和 API | `grok login`、`grok logout` | `~/.grok/auth.json` |
| Grove Git | 此克隆要访问的远程仓库 | `grove status`、`grove reload-credentials` | 守护进程凭据单元，由 Grove 配置中的 `auth_mode` 构建 |

`grok clone` 从不读取 `~/.grok/auth.json` 获取 Git 凭据。登录 Grok 不会为远程仓库向守护进程提供凭据，`[clone] enabled = true` 也不会；该标志只是决定是否运行 `grok clone` 的产品开关。

凭据被拒绝时，克隆会打印类别及负责该类别的命令，但不会打印远程 URL：

```
Grove Git credentials rejected (unavailable).
Grove Git credentials belong to the Grove daemon, not `grok login`.
Check `grove status` then `grove reload-credentials`.
```

| 类别 | 含义 | 下一步 |
|---|---|---|
| `unavailable` | 守护进程没有可用凭据，或远程仓库拒绝了凭据 | `grove status`，修复来源后运行 `grove reload-credentials` |
| `expired-static` | 令牌已过期，且此部署不会刷新令牌 | 开始新会话，或启用 `GROVE_TOKEN_ROTATION=expected` |
| `carrier-stale` | 守护进程仍持有被拒绝的旧承载令牌 | 等待改写，或运行 `grove reload-credentials` |
| `other` | 凭据提供方因其他原因失败 | 运行 `grove status`，然后 `grove reload-credentials` |

只有 `expired-static` 和 `carrier-stale` 会添加专属消息。私有仓库的访问失败可能被 GitHub 报告为“仓库不存在”；克隆会报告缺少仓库，而不会猜测凭据问题。

`grove status` 即使没有挂载点也会打印身份验证块：

```
  auth: mode=auto live=git-delegate health=ok last=- reload=supported
        hint=credentials look healthy
```

`mode` 是配置的 `auth_mode`，`live` 是守护进程实际持有的提供方。重新加载会从环境和磁盘重建凭据单元，并打印最终提供方，但不会打印秘密。使用 `auto` 或 `git` 且没有承载令牌时，请先配置 `git credential` 或 `gh auth`，再运行 `grove reload-credentials`。

## 守护进程

`grok clone` 使用正在运行的 Grove 守护进程。如果控制套接字关闭，它会以分离进程启动 `grove daemon --foreground`，因此退出或按 Ctrl-C 结束 `grok` 不会停止守护进程或挂载点，然后等待套接字。

`grove` 二进制文件先从 `PATH` 查找，再从 `grok` 可执行文件所在目录查找（例如 `~/.grok/bin/grove`）。Linux 启动守护进程前需要可用的 FUSE；缺少 FUSE 会立即报错，而不是挂起。Windows 不支持此功能（没有 ProjFS 后端）；请使用 `git clone`，或在 macOS/Linux 上运行 `grok clone`。

### 当前版本命令与配置补充

以下示例保留当前版本的可执行命令和配置格式：

```bash
grove doctor --install-agent
```

```bash
grove reload-credentials
# grove: credentials reloaded → token-file
```

```bash
cargo build -p grove --release
```

```bash
grove daemon --foreground
```
