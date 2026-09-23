<a id="subagents-and-personas"></a>
# 子智能体与 Persona

子智能体是独立的子会话，可以并行处理任务。每个子智能体都有自己的上下文窗口，因此主智能体可以委派研究、实现、测试和代码审查，而不消耗自己的上下文。子智能体完成后，会向父级报告摘要。

默认启用子智能体。

---

<a id="agents-vs-personas"></a>
## 智能体与 Persona 的区别

智能体和 Persona 都可以定制行为，但作用层级不同：

| | **智能体（Agents）** | **Persona** |
|---|---|---|
| **配置内容** | 整个会话：模型、工具、提示模式、系统提示 | 添加到子智能体提示中的行为覆盖层 |
| **作用范围** | 主会话或子智能体 | 仅子智能体 |
| **设置方式** | 启动时设置，或通过智能体定义（`.md` 文件，位于 `.grok/agents/` 或 `~/.grok/agents/`）设置 | 在 `config.toml`（`[subagents.personas]`）中设置，或在 `.grok/personas/` 下的 `.toml` 文件中设置；在解析子智能体时应用 |
| **控制内容** | 模型、工具可用性、提示正文、技能 | 语气、输出格式、任务重点以及输入/输出契约 |
| **编辑者** | 你——在智能体模态框中，或通过编辑文件来创建、删除或切换 | 你——在配置或文件中定义自定义 Persona；内置 Persona 只读 |
| **示例** | `grok-build`、`explore`、`plan` | `researcher`、`concise` |

智能体定义会话本身；Persona 决定子智能体在会话中的行为。子智能体始终以某种智能体类型（例如 `general-purpose`）运行，解析时还可以在其上叠加 Persona。

在智能体模态框中管理二者。使用 `/config-agents`（别名 `/agents`）打开模态框，或直接用 `/personas` 打开 Personas 标签页。模态框有两个标签页：**Agents** 和 **Personas**。

---

<a id="disabling-subagents"></a>
## 禁用子智能体

可通过 CLI 标志、环境变量或配置文件禁用子智能体（优先级从高到低）。同一规则适用于交互式 `grok` TUI、`grok agent stdio` 和无头运行。

```bash
grok --no-subagents                  # 仅本次会话
export GROK_SUBAGENTS=0              # Environment variable
```

```toml
# ~/.grok/config.toml
[subagents]
enabled = false
```

只有显式的 `enabled = false` 才会关闭子智能体。`[subagents]` 表若只设置 `max_depth`、`[subagents.models]` 或 `[subagents.toggle]` 而没有 `enabled` 键，子智能体仍保持开启。

---

<a id="how-subagents-work"></a>
## 子智能体的工作方式

当主智能体确定要委派工作时，会调用 `spawn_subagent` 工具启动子会话。子会话具有：

- 独立于父级的上下文窗口
- 由其智能体类型和可选能力模式决定的工具集
- 在解析过程中应用的可选 Persona 指令

子智能体完成后，父级会收到子智能体的输出（通常是摘要）。

---

<a id="built-in-agent-types"></a>
## 内置智能体类型

内置类型仍作为宿主类型存在。面向模型的生成 schema 省略了 `subagent_type`。省略该键时即为 `general-purpose`。

| 类型              | 说明                                          |
| ----------------- | -------------------------------------------- |
| `general-purpose` | 默认类型。可处理任何任务的全能力智能体。    |
| `explore`         | 研究智能体。搜索、读取、grep 并运行 shell 命令，但不编辑文件。用于代码库调查。 |
| `plan`            | 规划智能体。探索代码库并生成结构化实现计划；不编辑文件。 |

项目或用户定义的智能体可以添加新的类型，或按名称覆盖这些内置类型。

---

<a id="personas"></a>
## Persona

Persona 是一种命名的行为覆盖层。它的指令会作为 `<system-reminder>` 注入子智能体对话，从而塑造语气、输出格式和任务重点，但不会改变子智能体的智能体类型、模型或工具。

可在 `config.toml` 或 `.toml` 文件中定义 Persona：

```toml
[subagents.personas.researcher]
instructions = "You are a thorough researcher. Always cite specific file paths."
description = "Deep investigator."
```

Grok Build 会按优先级顺序从以下位置发现基于文件的 Persona：

- `.grok/personas/*.toml`（项目）
- `~/.grok/personas/*.toml`（用户）
- 内置 Persona 目录（最低优先级）

每个文件定义一个 Persona，文件名（不含扩展名）成为 Persona 名称。内联 `config.toml` Persona 的优先级高于文件。只发现 `.toml` 文件。

在智能体模态框的 Personas 标签页（`/personas`）中管理 Persona。内置 Persona 为只读；你定义的 Persona 可编辑。

> **注意：** Grok Build 通过子智能体解析和角色应用 Persona，而不是通过 `spawn_subagent` 参数应用。主智能体生成子级时不会传递 Persona 名称。

<a id="persona-fields"></a>
### Persona 字段

| 字段               | 说明                                                          |
| ------------------- | ------------------------------------------------------------ |
| `instructions`      | 以内联指令文本作为 Persona 层应用。               |
| `instructions_file` | 指令文件的路径；在生成时加载，并在 `instructions` 之后合并。 |
| `description`       | Persona 目录中显示的简短摘要。若未提供，则回退为 `instructions` 的第一段。 |
| `inputs` / `outputs`| 声明的输入和输出契约（见下文）。                     |
| `model`             | 使用 Persona 时应用的模型覆盖。                    |
| `reasoning_effort`  | 使用 Persona 时应用的推理力度。                  |
| `default_isolation` | 默认隔离模式（`none` 或 `worktree`）。                      |

<a id="input-output-contracts"></a>
### 输入/输出契约

Persona 可以声明它所需的输入和它产生的输出。父智能体读取这些声明来了解应提供哪些上下文以及应期待哪些工件。这样就可以串联 Persona：一个 Persona 的输出文件成为下一个 Persona 的输入：

```toml
[[subagents.personas.reviewer.inputs]]
name = "review_file"
io_type = "file"
required = true
description = "Path to the code under review"

[[subagents.personas.reviewer.outputs]]
name = "summary_file"
io_type = "file"
required = false
description = "Path to write review notes"
```

每个字段都有 `name`、`io_type`（默认值为 `file`）、`required` 标志和 `description`。

<a id="persona-resolution"></a>
### Persona 解析

应用 Persona 时，Grok Build 按以下顺序解析生效的模型和推理力度，优先级从高到低：

1. 生成时的显式覆盖
2. 角色默认值
3. Persona 默认值
4. 父会话

隔离遵循前三步的相同顺序，但默认值为 `none`（无工作树），而不是继承父会话。

如果请求了 Persona 但无法解析——找不到、没有指令，或其 `instructions_file` 无法读取——生成会失败。

---

<a id="spawning-subagents"></a>
## 生成子智能体

主智能体调用 `spawn_subagent` 工具。参数如下：

| 参数                | 说明                                                       |
| ------------------- | ------------------------------------------------------------ |
| `prompt`            | 给子智能体的完整任务提示。                           |
| `description`       | 任务的简短标签（3–5 个单词）。                          |
| `run_in_background` | 在后台运行并返回子智能体 ID。默认为 `true`。 |
| `isolation`         | `none`（共享工作区，默认）或 `worktree`（隔离的 Git 工作树）。 |
| `resume_from`       | 继续已完成的子智能体对话。传入其子智能体 ID。 |
| `cwd`               | 子智能体的工作目录。与 `isolation: worktree` 互斥；设置 `resume_from` 时忽略（恢复的子级继承源目录）。 |

后台运行子智能体时，稍后使用 `get_command_or_subagent_output` 获取其结果。

### 向子智能体发送消息

`send_subagent_message` 工具默认关闭。使用 `GROK_ACTIVE_AGENT_MESSAGES` 或 `[features] active_agent_messages` 启用它。

根会话可以向自己拥有的子智能体发送后续消息。开关启用后，获授权的子级也会获得该工具：

- `subagent_id: "parent"` 指向该子级当前的父智能体。
- 持久智能体 ID 可指向另一个本地子智能体。符合条件的已完成子智能体会以相同身份恢复。

父级是根会话的子级不能向根会话发消息。精选 harness 工具集不会获得该工具；排除这类工具的能力模式也会将其排除。

每个发送者-目标对最多允许 4 条在途消息，每次发送尝试最多 32 条出站消息。超过限制会返回 `QuotaExceeded`。

不活跃的子智能体会将消息作为下一轮唤醒。对于活跃的子智能体，可选的 `delivery` 参数控制消息到达方式：

- `steer`（默认）在下一个安全点加入当前轮次。
- `queue` 作为受保护的后续轮次等待，不进入当前轮次。
- `interject` 为紧急模式：在最早安全点排在待处理 steer 之前；如果子智能体正在等待后台工作，还会中断等待，让它立即读取消息。只有等待调用提前结束，后台工作仍会继续。

如果子智能体处于活跃状态但正处于轮次之间，`steer` 和 `interject` 都会变成一个受保护的排队轮次，随后子智能体开始处理。旧版 `queue: true` 仍受支持，并表示 `delivery: "queue"`；两者同时存在时以 `delivery` 为准。

记录会将每次发送显示为一行 `Message`：先显示结果动词，再显示子智能体标签（其 Persona、角色、标签，或回退名 Subagent）以及花括号引号中的描述；其 `Subagent …: “…”` 回滚行会引用该描述，并截断到首行 40 个字符。动词体现投递方式，因此 steer 不带标记：

- `Message sent to Subagent “find callers”`（steer）
- `Message queued for Subagent “find callers”` / `Message interjected to Subagent “find callers”`
- 发送进行中显示带动画项目符号的 `Message sending to …`
- 拒绝发送显示 `Message rejected · Subagent “find callers”`，Shell 无法确认的发送显示 `Message unconfirmed · Subagent “find callers”`
- 子级向父级发送消息时显示 `Message sent to parent`

折叠行不会显示消息或原因。**Right**（Vim 模式下为 `l`/`e`）展开后会显示请求的投递方式、完整消息文本以及拒绝或未确认发送的原因；**Left**（或 `h`）再次折叠。按 **Enter**、**Ctrl+F** 或双击该行会打开对应子智能体的视图，与其 `Subagent` 行完全相同（Right/Left 仍用于折叠）。如果子智能体未在本会话中生成（无头 `grok export` 或来自其他会话的 ID），该行会用 ID 最后 8 个字符命名为 `subagent …xxxxxxxx`，展开时显示原始 `Subagent ID:`，且无法打开。

---

<a id="capability-modes"></a>
## 能力模式

能力模式不是创建子智能体时的参数。子智能体可用的工具由其**智能体类型**以及**角色 / 定义的默认值**决定。`general-purpose` 不受限制（`all`）；内置 `explore` 和 `plan` 类型可以读取、搜索并运行 shell 命令，但不能编辑文件。

| 模式         | 读取 | 写入 | 执行 | 说明                                  |
| ------------ | ---- | ---- | -------- | -------------------------------------------- |
| `read-only`  | 是   | 否   | 否       | 读取、搜索和检查（也包括网页搜索和 LSP）；不允许文件编辑或 shell。 |
| `read-write` | 是   | 是   | 否       | 读取，以及创建、编辑、删除和移动文件。不允许 shell。 |
| `execute`    | 是   | 否   | 是       | 读取，以及运行 shell 命令和后台任务。不允许文件编辑。 |
| `all`        | 是   | 是   | 是       | 不受限的工具访问；`general-purpose` 的默认值。 |

---

<a id="context-inheritance"></a>
## 上下文继承

<a id="resume-from"></a>
### `resume_from`

`resume_from` 参数允许新的子智能体从已完成的子智能体继续，这对多阶段工作流很有用：

1. 生成一个研究子智能体来调查问题。
2. 生成第二个子智能体，并将第一个子智能体的 ID 设置为 `resume_from`，使其获得完整的研究上下文。

新的子智能体继承源子智能体的记录、工具状态和模型；其系统提示和工具会根据当前智能体定义重新渲染。源子智能体必须已完成（不能仍在运行）、属于当前会话，并使用相同的智能体类型。

<a id="mcp-inheritance"></a>
### MCP 继承

主会话会按名称把当前智能体的 `mcpServers` frontmatter 覆盖到磁盘与客户端的合并结果上（agent.md 的标头优先于 `config.toml`）。切换主智能体时，只用新席位替换该覆盖层。子级内联的 `mcpServers` 仍会成为其自有客户端，并优先于继承来的共享客户端。插件智能体不能声明 `mcpServers`。

默认情况下，子智能体继承父会话中**已经连接**的 MCP 服务器。这包括本地 stdio/HTTP 服务器和插件提供的智能体（例如 `my-plugin:reviewer`）。子级通过 `search_tool` / `use_tool` 以与父级相同的方式发现和调用这些工具。

通过智能体 frontmatter 的 `mcpInheritance` 控制继承：

| 值 | 效果 |
| ----- | ------ |
| `all`（省略时的默认值） | 继承父级已连接的每个 MCP 服务器 |
| `none` | 不继承父级 MCP 服务器 |
| `named: [server, …]` | 仅继承列出的服务器名称 |
| `except: [server, …]` | 继承父级除列出名称外的所有服务器 |

示例：

```yaml
---
name: research-only
description: Read MCP tools but not internal connectors
tools: search_tool, use_tool, Read
mcpInheritance:
  except:
    - internal-tools
---
```

**插件智能体**也会以相同方式继承父级 MCP。出于安全原因，它们仍不能：

- 在智能体 frontmatter 中声明自己的 `mcpServers`（会带警告忽略）
- 在智能体 frontmatter 中声明钩子
- 设置 `permissionMode: bypassPermissions`

插件捆绑的 MCP 服务器（插件 `.mcp.json`）在插件受信任后仍附加到**父级/会话**，而不是仅在子级 frontmatter 中声明。参见[插件](09-plugins.md)和[MCP 服务器](07-mcp-servers.md)。

---

<a id="isolation-worktree-mode"></a>
## 隔离：工作树模式

对于会修改文件的任务，请使用 `isolation: worktree` 在隔离的 Git 工作树中运行子智能体。这可避免子级编辑与父级冲突：

- 子智能体在自己的工作树副本中工作。
- 其变更在合并前都与父级隔离。
- 子智能体的结果包含工作树路径。

Grok Build 通过 `x.ai/git/worktree/*` 扩展方法管理工作树，其中包括将变更合并回主工作目录的 apply 操作。

---

<a id="configuration"></a>
## 配置

<a id="per-type-toggles-and-model-overrides"></a>
### 按类型切换和模型覆盖

禁用特定智能体类型，或将其路由到其他模型：

```toml
[subagents.toggle]
explore = true                       # default -- omit to keep enabled
plan = false                         # disable the plan subagent

[subagents.models]
explore = "grok-4.6"                 # route explore to a specific model
```

按类型的模型覆盖适用于任何父级。没有覆盖时，子智能体继承父级模型。

可在 `/settings` → Models → **Subagent model inheritance** 中切换该项：

- 开启：Grok 不能为子智能体设置模型
- 关闭：Grok 可以为子智能体选择不同的模型。重启后生效。
- 注意：此设置仅当所有模型都属于 xAI 的 `model_family` 时适用。你很可能不需要配置此项。

该行显示重启后生效的值。切换会写入 `[features] subagent_model_inheritance = true` 或 `= false`（显式的 `false` 会覆盖远程的 `true`）；按 `d`（重置）会删除该键，使 `managed_config.toml`、远程设置或默认值重新生效。已经在运行的智能体保持其启动时的模式。当某一层不能被你的 `config.toml` 覆盖并决定该值时——`requirements.toml`／MDM 固定、环境变量、`GROK_CONFIG` 覆盖层，或进行中的 campaign——切换和重置都会被拒绝，并弹出指明该层的 toast。

<a id="custom-roles-and-personas"></a>
### 自定义角色和 Persona

定义带有自身能力和模型默认值的自定义角色：

```toml
[subagents.roles.researcher]
description = "Deep research agent"
default_capability_mode = "read-only"
model = "grok-4.6"
prompt_file = ".grok/prompts/researcher.md"
```

定义带行为指令的自定义 Persona：

```toml
[subagents.personas.concise]
instructions = "Be concise. No filler words."
# instructions_file = ".grok/personas/concise.md"  # or load from a file
```

Grok Build 还会从 `.grok/roles/*.toml` 发现角色，从 `.grok/personas/*.toml` 发现 Persona。内联 `config.toml` 定义的优先级高于文件。

---

<a id="the-tasks-pane-tui"></a>
## 任务窗格（TUI）

Grok Build 会在智能体屏幕的侧窗格中显示运行中和已完成的工作：

- 按 `Ctrl+G` 切换任务窗格，其中列出活动和已完成的子智能体以及带状态的后台命令。
- 按 `Ctrl+T` 切换独立的待办窗格。

要查看可用的智能体类型和 Persona，请用 `Ctrl+P` 打开命令面板，然后选择 **Manage Agents**（`/config-agents`）。

子智能体会出现在任务窗格顶部自己可折叠的“Subagents”分组中。

---

<a id="viewing-subagents-in-the-tui"></a>
## 在 TUI 中查看子智能体

交互式 TUI 的多个位置都会显示子智能体：

<a id="scrollback-parent-conversation-history"></a>
### 回滚区（父级对话历史）

生成子智能体时，父级回滚区会添加一个紧凑的生命周期块：

- `Subagent running: "do the thing" (Implementer · grok-3) · Thinking`
- 对于后台子智能体：`Subagent started: "..."`

运行中时，该块显示实时活动后缀（例如“Running: cargo test”“Compacting”“Retrying (2/3)”），取自子级的轮次跟踪器。项目符号会根据状态动画显示（或着色）。

按 **Enter**（或 Ctrl-F）可打开子智能体的完整记录。

对于阻塞式子智能体，子级完成时同一条记录会更新项目符号颜色。对于后台子智能体，还会追加 `Subagent completed/failed/cancelled in Xs: "..."` 块。

<a id="tasks-pane-ctrl-g"></a>
### 任务窗格（Ctrl+G）

如上所述——任务窗格按“Subagents”分组显示子智能体，并提供旋转指示器、耗时以及快速终止或检查的入口。按 `h` 切换隐藏已完成/显示全部。

<a id="fullscreen-framed-view-the-child-transcript"></a>
### 停靠栏（启用时）

提示框上方的停靠栏会列出子智能体。停靠栏获得焦点时，按 `h` 切换隐藏已完成/显示全部（与任务窗格相同的过滤器）；按 Left / Right 折叠或展开分组标题。

### 全屏框架视图（子级记录）

从回滚区块或任务窗格打开子智能体时，父级视图会被一个带边框的框架替换，其中显示子级的完整记录：

- 框架内的标题栏：状态图标（旋转指示器 / ✓ / ✗）、标签 + 粗体描述 + 模型、可选的“resumed”/“forked”徽章、实时活动 · 已耗时，以及 [✗] 关闭按钮。
- 子智能体自己的回滚、思考和工具调用会在框架内渲染。
- 父级任务窗格、待办窗格和停靠栏在视图期间隐藏。

该视图用于观察。编写器隐藏（零行），无法聚焦、输入提示、暂存草稿或发送后续消息。提示仍由父会话拥有；要引导运行中的子级，请关闭此视图，在父级使用 `send_subagent_message`（参见[向子智能体发送消息](#向子智能体发送消息)）。

**仍然有效**

- 滚动、折叠、复制、打开链接，以及在子级记录中打开块查看器。
- `Ctrl+C` 取消**子级**轮次，不会取消父级。
- `Ctrl+.` / `Ctrl+X` 打开子级按键的快捷键速查表。
- 子级视图不绘制 `[Dashboard]` 按钮。在 dashboard 覆盖层内，该按钮用于返回。
- 块查看器中空闲时按 `Enter` 会将选中行引用到父级编写器并关闭视图。

**无效操作（安全关闭）**

根级快捷键不会在此界面启动，不会在子级打开模态框，也不会泄漏到父级：

- 命令面板（`Ctrl+P`）、模型选择器（`Alt+M`）、会话选择器（`Ctrl+R`）
- 设置、扩展、始终批准（`Ctrl+O`）、发送到后台（`Ctrl+B`）
- 外部提示编辑器、Shift+Tab 模式循环

被拒绝的操作只会静默重绘，不会显示提示。

如果出现提示队列覆盖层，它只是**只读镜像**。无法编辑、立即发送或移除行；队列 RPC 始终指向父会话。

**如何离开**

- 在普通回滚区按 `q` 或 `Esc`，或点击 [✗]。
- 若回滚搜索已打开，`q` / `Esc` 会先关闭搜索；之后再次按键才关闭视图。
- `Ctrl+Q` 始终退出 Grok，在此处不会被吞掉。

关闭后，父级回滚区仍会显示子智能体状态。

---

<a id="depth-limits"></a>
## 深度限制

只有顶层会话可以生成子智能体。子智能体不能再生成自己的子智能体：最大嵌套深度为一层。如果子级调用 `spawn_subagent`，调用会失败并返回深度限制错误。这样可以保持智能体树扁平，避免无限生成。

---

<a id="when-to-use-subagents"></a>
## 何时使用子智能体

**适合的场景：**

- 父级继续其他工作时研究代码库
- 父级实现功能时并行运行测试
- 提交前审查生成的变更
- 委派彼此独立、互不依赖的任务

**不适合的场景：**

- 父级可以直接处理的简单任务
- 需要与用户紧密往返的任务，因为子智能体自主运行，不适合交互式交流
- 上下文准备成本超过并行收益的任务
