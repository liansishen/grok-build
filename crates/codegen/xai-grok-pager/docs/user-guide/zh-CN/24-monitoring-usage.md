# 监控用量（外部 OpenTelemetry）

> **状态：alpha。** 以下模式定义带有版本号（`grok_code.schema.version = v1`）；
> 可能会在不另行通知的情况下进行增量添加，重命名 / 移除则会提升版本号，
> 并在变更日志中明确说明。

Grok CLI 可以将用量**指标**和**事件**导出到你所在组织自己的 OpenTelemetry 收集器，使平台团队能够监控整个设备群中的采用情况、词元消耗、工具权限决策和错误——任何数据都不会流经 SpaceXAI。

## 相关设置

以下开关彼此独立（也独立于本指南所述的外部 OTEL 流）：

| 设置 | 设置方式 |
|---------|---------------|
| 遥测总开关 | `[features] telemetry` / `GROK_TELEMETRY_ENABLED` |
| 编码数据、保留与训练 | 设置——`/privacy` 会打开对应行 |
| 跟踪上传 | `[telemetry] trace_upload` / `GROK_TELEMETRY_TRACE_UPLOAD` |
| 外部 OpenTelemetry | `GROK_EXTERNAL_OTEL` / `[telemetry] otel_*`（本指南） |

另请参阅[身份验证](../02-authentication.md#related-settings)和[配置](../05-configuration.md#telemetry)。

## 外部 OTEL 流

外部流具有以下特性：

- **默认关闭**，并要求进行*双重选择加入*（一个总开关**以及**明确选择导出器）。
- **默认不含内容**：不包含提示、代码、文件路径（仅包含扩展名）、工具参数、bash 命令；MCP / 技能 / 插件名称会折叠为类别。可通过可选的内容门控重新启用其中一部分。
- 与 SpaceXAI 内部遥测**在结构上相互独立**：其导出器只携带你配置的请求头，绝不会携带 SpaceXAI 凭据。
- **不受 SpaceXAI 数据保留退出选项影响**：即使禁用了 `telemetry`，或对于 ZDR（零数据保留）团队，它仍然可以工作。上述设置控制 SpaceXAI 侧的数据保留；外部流则完全由你自己的 OTEL 配置控制。

## 快速开始

```bash
export GROK_EXTERNAL_OTEL=1                  # 总开关
export OTEL_METRICS_EXPORTER=otlp
export OTEL_LOGS_EXPORTER=otlp
export OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf  # 或 grpc
export OTEL_EXPORTER_OTLP_ENDPOINT=https://collector.corp.example:4318
export OTEL_EXPORTER_OTLP_HEADERS="Authorization=Bearer <collector-token>"
grok
```

仅设置 `GROK_EXTERNAL_OTEL=1` **不会启用任何功能**——你还必须选择至少一个导出器。反之，仅设置 `OTEL_*` 变量而不开启总开关，也不会启用任何功能。

## 环境变量

| 变量 | 默认值 | 含义 |
|---|---|---|
| `GROK_EXTERNAL_OTEL` | `0` | 总开关。不同于控制 SpaceXAI 内部产品分析的 `GROK_TELEMETRY_ENABLED`——两者控制方向相反的数据流。 |
| `OTEL_METRICS_EXPORTER` | `none` | `otlp` \| `console` \| `none`。 |
| `OTEL_LOGS_EXPORTER` | `none` | `otlp` \| `console` \| `none`。控制事件流。 |
| `OTEL_EXPORTER_OTLP_PROTOCOL` | `http/protobuf` | `http/protobuf` \| `grpc`。 |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | HTTP 为 `http://localhost:4318`，gRPC 为 `http://localhost:4317` | 基础端点。对于 `http/protobuf`，会按照 OTLP 规范分别追加 `/v1/logs` 和 `/v1/metrics`；对于 `grpc`，收集器端点会原样使用。 |
| `OTEL_EXPORTER_OTLP_LOGS_ENDPOINT` / `..._METRICS_ENDPOINT` | — | 针对特定信号的覆盖值，按原样使用。对于 gRPC，通常应为不带 `/v1/...` 路径的收集器端点。 |
| `OTEL_EXPORTER_OTLP_HEADERS`（及针对特定信号的变体） | — | 收集器身份验证（`k=v,k2=v2`）。这是外部导出器发送的**唯一**请求头，也是唯一受支持的收集器身份验证机制（配置文件中没有 `headers` 键——访问令牌绝不会保存在磁盘上）。 |
| `OTEL_EXPORTER_OTLP_TIMEOUT` | `10000`（ms） | 导出超时时间。 |
| `OTEL_METRIC_EXPORT_INTERVAL` | `60000`（ms） | 指标导出间隔。 |
| `OTEL_BLRP_SCHEDULE_DELAY`（或别名 `OTEL_LOGS_EXPORT_INTERVAL`） | `5000`（ms） | 日志批处理间隔。 |
| `OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE` | `delta` | `delta` \| `cumulative`。 |
| `OTEL_METRICS_INCLUDE_SESSION_ID` | `1` | 将 `session.id` 附加到指标（可选择退出以降低基数）。 |
| `OTEL_METRICS_INCLUDE_VERSION` | `0` | 将 `app.version` 附加到指标。 |
| `OTEL_LOG_USER_PROMPTS` | `0` | 内容门控：在 `grok_code.user_prompt` 上附加提示文本（上限 60 KB，经过密钥清理）。 |
| `OTEL_LOG_TOOL_DETAILS` | `0` | 内容门控：工具参数（上限 4 KB）、完整文件路径，以及未经修改的 MCP / 技能 / 插件名称。即使启用此门控，v1 中也**绝不会**导出 Bash 命令文本。 |

系统会有意忽略 `OTEL_RESOURCE_ATTRIBUTES`：资源由一组经过审计的固定属性构建。

> **迁移说明：**旧版本可以让 `OTEL_EXPORTER_OTLP_*` 与产品自身的分析管道共享。
> 该行为现已弃用：设置 `GROK_EXTERNAL_OTEL` 后，产品分析会忽略这些变量；
> 如果产品分析已使用这些变量，CLI 会拒绝在该配置中激活外部流——
> 你的收集器只会接收到你明确选择加入的外部流。

## 配置文件

组织默认值位于 `config.toml` 中现有的 `[telemetry]` 表下（环境变量优先）。这些键是其他 `[telemetry]` 设置以 `otel_` 为前缀的对应项：

```toml
[telemetry]
otel_enabled = true
otel_metrics_exporter = "otlp"
otel_logs_exporter = "otlp"
otel_endpoint = "https://collector.corp.example:4318"
otel_protocol = "http/protobuf"  # 或 "grpc"
otel_log_user_prompts = false   # 管理员可通过 requirements 固定这些值
otel_log_tool_details = false
```

配置键是 `[telemetry]` 下的 `otel_*`；为了与生态系统互操作，**环境变量保留其标准 OTEL 名称**（`GROK_EXTERNAL_OTEL`、`OTEL_*`），因此这两个层级有意使用不同的命名空间。`otel_protocol` 配置键映射到 `OTEL_EXPORTER_OTLP_PROTOCOL`。

这里特意没有 `headers` 键：请通过 `OTEL_EXPORTER_OTLP_HEADERS` 提供收集器身份验证信息，确保访问令牌永远不会存储在磁盘上。

托管部署还可以通过 `grok setup` 托管配置 / requirements 固定项分发 `[telemetry]` 的 `otel_*` 键，从而在组织范围内启用遥测；也可以使用相同的本地配置层（`external_otel_disabled`、内容门控锁）在整个设备群中强制将其禁用。

## 资源属性

| 属性 | 值 |
|---|---|
| `service.name` | `grok-cli` |
| `service.version`、`client.version` | 构建 / 客户端版本 |
| `app.entrypoint` | `cli` \| `headless` \| `agent` |
| `terminal.type` | 终端模拟器品牌 |
| `grok_code.schema.version` | `v1` |

身份属性（`user.id`，以及已知时的 `organization.id` / `team.id` / `deployment.id`）会在身份验证完成后附加到每个指标数据点和每个事件。`prompt.id`（每个提示的 UUID）只会出现在事件中，绝不会出现在指标中。

## 指标（meter scope `ai.xai.grok_code`）

| 指标 | 单位 | 属性 |
|---|---|---|
| `grok_code.session.count` | `{session}` | 仅基础属性 |
| `grok_code.token.usage` | `{token}` | `type` = `input` \| `output` \| `reasoning` \| `cache_read`；`model` |
| `grok_code.turn.count` | `{turn}` | `outcome` = `completed` \| `cancelled` \| `error`；`model` |
| `grok_code.tool.decision` | `{decision}` | `tool_name`、`decision` = `allow` \| `deny` \| `cancelled` \| `followup`、`access_kind`、`permission_mode` |
| `grok_code.tool.usage` | `{call}` | `tool_name`、`outcome` |
| `grok_code.error.count` | `{error}` | `error_category`、`model` |

没有 `cost.usage` 指标：请将 `grok_code.token.usage` 与你自己的价格表关联。`lines_of_code.count` 和 `active_time.total` 计划在后续阶段提供。

`tool_name` 的值：内置工具名称按原样传递；除非设置 `OTEL_LOG_TOOL_DETAILS=1`，否则 MCP 工具会折叠为 `mcp_tool`，其他非内置工具会折叠为 `custom_tool`。

## 事件（OTLP 日志记录）

每个事件都携带 `event.sequence`、`session.id`、`turn_number`（回合内）、`prompt.id`，以及身份属性。门控图例：**details** = 需要 `OTEL_LOG_TOOL_DETAILS`，**prompts** = 需要 `OTEL_LOG_USER_PROMPTS`；外部流处于活动状态时，其他所有内容都会始终导出。

| `event.name` | 属性 |
|---|---|
| `grok_code.session_start` | `model`、`permission_mode`、`mcp_server_count`、`plugin_count`、`skill_count`、`hook_count`、`memory_enabled`、`is_git_repo`、`client_identifier` |
| `grok_code.session_end` | `duration_secs`、`turn_count`、`tool_call_count`、`compaction_count`、`model` |
| `grok_code.user_prompt` | `prompt_length`、`model`、`screen_mode?`（`fullscreen` \| `inline` \| `minimal` \| `headless` \| `other`）；`prompt`（**prompts**） |
| `grok_code.turn_completed` | `outcome`、`duration_ms`、`tool_call_count`、`model`、`error_category?`、`cancellation_category?` |
| `grok_code.api_request` | `model`、`duration_ms`、`stop_reason?`、`input_tokens`、`output_tokens`、`reasoning_tokens`、`cache_read_tokens` |
| `grok_code.api_error` | `error_category`、`model`、`status_code?`、`duration_ms?` |
| `grok_code.tool_result` | `tool_name`、`outcome`、`success`、`duration_ms`、`file_extension`；`tool_parameters`、`file_path`（**details**） |
| `grok_code.tool_decision` | `tool_name`、`decision`、`access_kind`、`permission_mode`、`source` |
| `grok_code.mcp_server_connection` | `status`、`transport_type`、`duration_ms`、`tool_count?`、`error_type?`；`mcp_server.name`（**details**；否则折叠为 `mcp_server`） |
| `grok_code.permission_mode_changed` | `to_mode`、`trigger` |
| `grok_code.skill_activated` | `skill_source`；`skill.name`（**details**） |
| `grok_code.plugin_loaded` | `install_kind?`、`success`、`error_category?`；`plugin_name`（**details**） |
| `grok_code.compaction` | `duration_ms`、`tokens_before`、`tokens_after`、`model?` |
| `grok_code.subagent` | `phase` = `launched` \| `completed`、`subagent_type?`、`outcome?`、`duration_ms?` |
| `grok_code.auth` | `auth_method` |
| `grok_code.internal_error` | `error_type`（仅类别——不包含消息，不包含位置） |
| `grok_code.model_switched` | `from_model`、`to_model`、`success`、`error_code?` |

## 隐私模型

三种彼此独立、故障时默认拒绝的机制用于保护传输格式：

1. **类型化模式定义**：属性键是封闭枚举；无法附加枚举之外的任何内容。
2. **发送时脱敏**：每个字符串都会经过密钥形态清理和主目录清理，并进行截断（每个值 512→128 个字符，工具参数上限 4 KB，提示上限 60 KB）。
3. **导出时验证器**：任何携带模式定义之外的键、门控关闭键或未经清理的密钥形态的记录，都会在离开进程前被丢弃；包含模式定义之外属性键的指标导出会被整体丢弃。

绝不会导出：bash 命令文本、错误消息正文、提示文本（未启用门控时）、文件路径（未启用门控时）、`api_key.id`、机器指纹、电子邮件地址、订阅层级。

## 收集器配置示例

```yaml
receivers:
  otlp:
    protocols:
      http:
        endpoint: 0.0.0.0:4318
      grpc:
        endpoint: 0.0.0.0:4317

processors:
  batch:

exporters:
  prometheus:
    endpoint: 0.0.0.0:9464

service:
  pipelines:
    metrics:
      receivers: [otlp]
      processors: [batch]
      exporters: [prometheus]
    logs:
      receivers: [otlp]
      processors: [batch]
      exporters: []   # 指向你的日志后端（loki、elasticsearch 等）
```

查询示例（PromQL，使用上面的 Prometheus 导出器）：

```promql
# 整个组织按模型和类型统计的 token，1 小时速率
sum by (model, type) (rate(grok_code_token_usage_total[1h]))

# 每个团队每天的会话数
sum by (team_id) (increase(grok_code_session_count_total[1d]))

# 工具权限拒绝率
sum(rate(grok_code_tool_decision_total{decision="deny"}[1h]))
  / sum(rate(grok_code_tool_decision_total[1h]))
```

## 调试

设置 `OTEL_LOGS_EXPORTER=console` / `OTEL_METRICS_EXPORTER=console`，可将经过脱敏的记录输出到 **stderr**（在 `agent` / `headless` 入口点中会抑制输出，以保持捕获的日志干净）。导出错误绝不会显示在 TUI 中；请查看调试日志。
