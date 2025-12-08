# 配置

<!-- markdownlint-disable MD012 MD013 MD028 MD033 -->

Every Code 支持多种方式设置配置值：

- 针对配置的命令行参数，如 `--model o3`（最高优先级）。
- 通用 `-c`/`--config`，以 `key=value` 形式传参，例如 `--config model="o3"`。
  - key 可含点号以设置更深层字段，如 `--config model_providers.openai.wire_api="chat"`。
  - 为与 `config.toml` 一致，值使用 TOML 字符串格式，而非 JSON。用 `key='{a = 1, b = 2}'` 而不是 `key='{"a": 1, "b": 2}'`。
    - 必须为值加引号，否则 shell 会按空格拆分，导致 `code` 接收到 `-c key={a` 这类非法参数。
  - 值可为任意 TOML 对象，例如 `--config shell_environment_policy.include_only='["PATH", "HOME", "USER"]'`。
  - 若 value 无法解析为合法 TOML，则按字符串处理，因此 `-c model='"o3"'` 与 `-c model=o3` 等效。
    - 第一种解析为 TOML 字符串 `"o3"`，第二种因 `o3` 不是合法 TOML，最终也视为字符串 `"o3"`。
    - 引号会被 shell 解释，`-c key="true"` 会解析为 TOML 布尔值 `true` 而非字符串；若需要字符串 `"true"`，使用 `-c key='"true"'`。
- `$CODE_HOME/config.toml` 配置文件。`CODE_HOME` 默认 `~/.code`；Every Code 也会读取 `$CODEX_HOME`/`~/.codex`（只写入 `~/.code`）。日志等状态共享该目录。

`--config` 参数与 `config.toml` 支持以下选项：

## model

指定 Code 使用的模型。

```toml
model = "o3"  # 覆盖默认 "gpt-5.1-codex"
```

## model_providers

覆盖或补充默认的模型提供商。该表的键对应 `model_provider` 的取值。

例如要通过 Chat Completions API 使用 OpenAI 4o，可添加：

```toml
# 根键需在表之前声明
model = "gpt-4o"
model_provider = "openai-chat-completions"

[model_providers.openai-chat-completions]
name = "OpenAI using Chat Completions"
# POST /chat/completions 会拼接到此 URL
base_url = "https://api.openai.com/v1"
# env_key 指定必须存在的环境变量；值用于 Bearer TOKEN 头
env_key = "OPENAI_API_KEY"
# wire_api 可为 "chat" 或 "responses"，默认 chat
wire_api = "chat"
# 需要时可添加额外查询参数，见下方 Azure 示例
query_params = {}
```

只要兼容 OpenAI Chat Completions 协议，Code CLI 也可接入其他模型。例如本地 Ollama：

```toml
[model_providers.ollama]
name = "Ollama"
base_url = "http://localhost:11434/v1"
```

或第三方提供商（使用独立的 API Key 环境变量）：

```toml
[model_providers.mistral]
name = "Mistral"
base_url = "https://api.mistral.ai/v1"
env_key = "MISTRAL_API_KEY"
```

也可为请求添加额外 HTTP 头，既可硬编码（`http_headers`）也可从环境变量读取（`env_http_headers`）：

```toml
[model_providers.example]
# name、base_url ...
http_headers = { "X-Example-Header" = "example-value" }
# 若环境变量存在且非空，会添加对应头
env_http_headers = { "X-Example-Features" = "EXAMPLE_FEATURES" }
```

### Azure 提供商示例

Azure 需要将 `api-version` 作为查询参数传递，请在 `query_params` 中设置：

```toml
[model_providers.azure]
name = "Azure"
# 将 YOUR_PROJECT_NAME 替换为你的子域
base_url = "https://YOUR_PROJECT_NAME.openai.azure.com/openai"
env_key = "AZURE_OPENAI_API_KEY"  # 或 "OPENAI_API_KEY"
query_params = { api-version = "2025-04-01-preview" }
wire_api = "responses"
```

在启动 Code 前导出密钥：`export AZURE_OPENAI_API_KEY=…`

### 每个提供商的网络调优

以下可选设置控制 **每个模型提供商** 的重试与流式空闲超时。必须写在对应的 `[model_providers.<id>]` 下（早期顶层键已废弃）：

```toml
[model_providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
request_max_retries = 4         # HTTP 请求重试
stream_max_retries = 10         # SSE 流掉线重试
stream_idle_timeout_ms = 300000 # 流式空闲超时（毫秒）
```

- `request_max_retries`：失败请求的重试次数，默认 `4`。
- `stream_max_retries`：流式中断的重连次数，默认 `5`。
- `stream_idle_timeout_ms`：流式无活动判定断开的等待时间，默认 `300000`（5 分钟）。

## model_provider

从 `model_providers` 表选择提供商，默认 `"openai"`。可用环境变量覆盖内置 `openai` 的 `base_url`（`OPENAI_BASE_URL`），并用 `OPENAI_WIRE_API` 强制协议（`"responses"` 或 `"chat"`）。

若覆盖了 `model_provider`，通常也要覆盖 `model`。例如本地运行 Ollama 的 Mistral：

```toml
model_provider = "ollama"
model = "mistral"
```

## approval_policy

决定何时提示用户允许执行命令：

```toml
# Code 内置了一组“可信”命令。
# 设置为 untrusted 时，运行不在列表的命令前会提示。
approval_policy = "untrusted"
```

希望命令失败时再提示，可用 "on-failure"：

```toml
# 命令在沙箱失败时，会请求在沙箱外重试的权限。
approval_policy = "on-failure"
```

由模型决定何时申请权限：

```toml
approval_policy = "on-request"
```

或完全不提示、直接运行：

```toml
# 不会提示；命令失败时 Code 会自行尝试其他方案。`exec` 子命令始终使用此模式。
approval_policy = "never"
```

## agents

使用 `[[agents]]` 注册额外的 CLI 程序，Code 会作为对等智能体启动它们。每个块映射短 `name`（在配置中引用）到可执行命令、默认参数和环境变量。

> **注意：** 内置模型 slug（如 `code-gpt-5.1-codex`、`claude-sonnet-4.5`）会自动注入正确的 `--model`/`-m`。为避免冲突，Code 会在启动智能体前去掉你在 `args`/`args_read_only`/`args_write` 中提供的 `--model`/`-m`。需要新的模型变体时，请在 `code-rs/core/src/agent_defaults.rs` 添加 slug（或设置 CLI 使用的环境变量），不要在这里固定参数。

```toml
[[agents]]
name = "context-collector"
command = "gemini"
enabled = true
read-only = true
description = "Gemini long-context helper that summarizes large repositories"
args = ["-y"]
env = { GEMINI_API_KEY = "..." }
```

当 `enabled = true` 时，智能体会出现在 TUI 选择器及引用它的子智能体命令中。`read-only = true` 则即便主会话允许写入，也会在修改文件前请求审批。

## notice

Code 会在 `[notice]` 表中存储一次性升级提示的确认标记。设为 `true` 后不会再提示对应迁移。

```toml
[notice]
hide_gpt5_1_migration_prompt = true
hide_gpt-5.1-codex-max_migration_prompt = true
```

## profiles

配置集（profile）是一组可以一起应用的配置值，可在 `config.toml` 定义多个，并通过 `--profile` 选择使用的配置。

示例 `config.toml`：

```toml
model = "o3"
approval_policy = "untrusted"
disable_response_storage = false

# 等同命令行 --profile o3，可被 CLI 覆盖
profile = "o3"

[model_providers.openai-chat-completions]
name = "OpenAI using Chat Completions"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "chat"

[profiles.o3]
model = "o3"
model_provider = "openai"
approval_policy = "never"
model_reasoning_effort = "high"
model_reasoning_summary = "detailed"

[profiles.gpt3]
model = "gpt-3.5-turbo"
model_provider = "openai-chat-completions"

[profiles.zdr]
model = "o3"
model_provider = "openai"
approval_policy = "on-failure"
disable_response_storage = true
```

配置优先级从高到低：

1. 自定义命令行参数（如 `--model o3`）
2. 指定的 profile（来自 CLI 或配置文件）
3. `config.toml` 中的条目（如 `model = "o3"`）
4. Code CLI 内置默认值（默认模型 `gpt-5.1-codex`）

## model_reasoning_effort

若所选模型支持推理（如 `o3`、`o4-mini`、`codex-*`、`gpt-5.1`、`gpt-5.1-codex`），在 Responses API 下默认开启。可设置：

- `"minimal"`
- `"low"`
- `"medium"`（默认）
- `"high"`

想尽量减少推理时用 `"minimal"`。

## model_reasoning_summary

模型名以 `"o"`（如 `"o3"`、`"o4-mini"`）或 `"codex"` 开头时，在 Responses API 下默认启用推理摘要。可设置：

- `"auto"`（默认）
- `"concise"`
- `"detailed"`

关闭推理摘要：

```toml
model_reasoning_summary = "none"
```

## model_verbosity

控制使用 Responses API 时 GPT‑5 系列模型的输出长度/详细度。取值：

- `"low"`
- `"medium"`（默认）
- `"high"`

设置后，Code 会在请求载荷中包含 `text` 对象，例如：

```toml
model = "gpt-5.1"
model_verbosity = "low"
```

仅对 Responses API 提供商生效，Chat Completions 不受影响。

## model_supports_reasoning_summaries

默认仅对已知支持推理摘要的 OpenAI 模型设置 `reasoning`。若想强制当前模型启用，可设置：

```toml
model_supports_reasoning_summaries = true
```

## sandbox_mode

Code 会在 OS 级沙箱中执行模型生成的命令。

大多数情况下可直接用单个选项：

```toml
# 等同 --sandbox read-only
sandbox_mode = "read-only"
```

默认策略为 `read-only`：命令可读取任意文件，但写文件或访问网络会被阻止。

更宽松的是 `workspace-write`：Code 任务的当前工作目录可写（macOS 上 `$TMPDIR` 也可写）。CLI 默认将启动目录作为 `cwd`，可用 `--cwd/-C` 覆盖。

历史上 `workspace-write` 允许写入顶层 `.git/`，这种宽松行为现在仍是默认。若想保护 `.git`，可在 `[sandbox_workspace_write]` 设置 `allow_git_writes = false`。

```toml
# 等同 --sandbox workspace-write
sandbox_mode = "workspace-write"

# 仅在 sandbox="workspace-write" 时生效
[sandbox_workspace_write]
exclude_tmpdir_env_var = false
exclude_slash_tmp = false
# 额外可写根目录（除了 $TMPDIR 与 /tmp）
allow_git_writes = true
writable_roots = ["/Users/YOU/.pyenv/shims"]
# 允许沙箱内命令访问网络，默认 false
network_access = false
```

完全关闭沙箱：

```toml
# 等同 --sandbox danger-full-access
sandbox_mode = "danger-full-access"
```

在已有沙箱环境（如 Docker）或本机沙箱不受支持的环境（旧内核、Windows）下，可考虑使用该选项。

## 审批预设

Code 提供三种审批预设：

- Read Only：可读文件与回答问题；写入、运行命令和联网需审批。
- Auto：可在工作区内读写并运行命令；超出工作区或联网时请求审批。
- Full Access：完全磁盘与网络访问，无提示，风险极高。

可结合 `--ask-for-approval` 与 `--sandbox` 在命令行进一步自定义。

## MCP 服务器

可配置 [MCP 服务器](https://modelcontextprotocol.io/about) 以访问外部应用、资源或服务（如 [Playwright](https://github.com/microsoft/playwright-mcp)、[Figma](https://www.figma.com/blog/design-context-everywhere-you-build/)、[documentation](https://context7.com/) 等）。

### 传输配置

每个服务器可设置 `startup_timeout_sec` 调整等待启动与返回工具列表的时长，默认 `10` 秒。`tool_timeout_sec` 限制单次工具调用运行时长（默认 `60` 秒），未设置时回退默认值。

配置方式与 Claude、Cursor 的 `mcpServers` 类似，但使用 TOML。JSON 示例：

```json
{
  "mcpServers": {
    "server-name": {
      "command": "npx",
      "args": ["-y", "mcp-server"],
      "env": { "API_KEY": "value" }
    }
  }
}
```

在 `~/.code/config.toml` 中表示为（也会读取旧版 `~/.codex/config.toml`）：

```toml
# 顶层表名必须为 mcp_servers
[mcp_servers.server-name]
command = "npx"
args = ["-y", "mcp-server"]  # 可选
# 传播给 MCP 服务器的额外环境变量（默认有白名单，见代码注释）
env = { "API_KEY" = "value" }
```

#### 可流式 HTTP

```toml
# 需要实验性的 rmcp 客户端
experimental_use_rmcp_client = true
[mcp_servers.figma]
url = "http://127.0.0.1:3845/mcp"
# 可选 Bearer Token（明文存储，谨慎使用）
bearer_token = "<token>"
```

OAuth 登录细节参见 MCP CLI 命令。

### 其他配置

```toml
startup_timeout_sec = 20  # 覆盖默认 10s
tool_timeout_sec = 30     # 覆盖默认 60s
```

### 从 CLI 管理 MCP（实验性）

```shell
# 添加服务器（env 可重复；`--` 之后是启动命令）
code mcp add docs -- docs-server --port 4000

# 列出服务器
code mcp list
code mcp list --json

# 查看单个服务器
code mcp get docs
code mcp get docs --json

# 删除服务器
code mcp remove docs

# 登录/登出支持 oauth 的可流式 HTTP 服务器
code mcp login SERVER_NAME
code mcp logout SERVER_NAME
```

## subagents

子智能体是可通过斜杠命令触发的编排辅助流程（如 `/plan`、`/solve`、`/code`）。`[[subagents.commands]]` 中的每个条目定义命令名、是否只读、要启动的 `agents`，以及对编排器与子智能体的额外指导。

默认（未配置 `[[agents]]`）时，多智能体运行会宣告以下模型 slug：`code-gpt-5.1`、`claude-sonnet-4.5`、`claude-opus-4.1`、`gemini-3-pro`、`gemini-2.5-pro`、`gemini-2.5-flash`、`qwen-3-coder`、`code-gpt-5.1-codex`、`code-gpt-5.1-codex-mini`。云端版本 `cloud-gpt-5.1-codex` 仅在 `CODE_ENABLE_CLOUD_AGENT_MODEL=1` 时出现。可通过 `[[agents]]` 或在具体 `[[subagents.commands]]` 中设置 `agents = [...]` 覆盖列表。

```toml
[[subagents.commands]]
name = "context"
read-only = true
agents = ["context-collector", "code-gpt-5.1"]
orchestrator-instructions = "Coordinate a context sweep before coding. Ask each agent to emit concise, linked summaries of relevant files and tooling the primary task might need."
agent-instructions = "Summarize the repository areas most relevant to the user's request. List file paths, rationale, and suggested follow-up scripts to run. Keep the reply under 2,000 tokens."
```

在上述示例下，可在 TUI 运行 `/context` 生成摘要单元，供后续 `/code` 引用。`context-collector` 作为普通智能体，如需调用静态分析工具（例如“爆炸半径”工具），请在 `agent-instructions` 描述，以便编排器启动正确流程。你也可以用相同 `name`（`plan`、`solve`、`code`）覆盖内置命令，将 `agents` 指向你的长上下文助手。

## validation

控制应用补丁前的快速验证。只要至少一个验证分组启用，验证就会自动运行。使用 `[validation.groups]` 控制分组开关，`[validation.tools]` 控制具体工具：

```toml
[validation.groups]
functional = true
stylistic = false

[validation.tools]
shellcheck = true
markdownlint = true
hadolint = true
yamllint = true
cargo-check = true
tsc = true
eslint = true
mypy = true
pyright = true
phpstan = true
psalm = true
golangci-lint = true
shfmt = true
prettier = true
```

功能检查默认开启以捕获改动中的回归；风格类 linters 默认关闭，可按需开启。

启用功能检查后，Code 会根据补丁所涉语言自动安排工具：

- Rust：`cargo-check`（限定受影响的 manifest）
- TS/JS：`tsc --noEmit` 与 `eslint --max-warnings=0`
- Python：`mypy` 与 `pyright`
- PHP：`phpstan`/`psalm`（依据配置或 Composer 项）
- Go：`golangci-lint run ./...`，并包含现有 JSON/TOML/YAML 语法检查

在 `[validation.tools]` 中可关闭特定工具，或在禁用分组后单独重新启用。

启用后还可对修改的 workflow 运行 `actionlint`，在 `[github]` 配置：

```toml
[github]
actionlint_on_patch = true
# 可选：指定二进制路径
actionlint_path = "/usr/local/bin/actionlint"
```

## disable_response_storage

若账号启用 Zero Data Retention（ZDR），需将 `disable_response_storage` 设为 `true`，以便使用兼容 ZDR 的替代模式：

```toml
disable_response_storage = true
```

## shell_environment_policy

Code 运行子进程（如助手建议的 `local_shell` 工具调用）时默认继承**完整环境**。可通过 `config.toml` 中的 `shell_environment_policy` 调整：

```toml
[shell_environment_policy]
# inherit 可为 "all"（默认）、"core" 或 "none"
inherit = "core"
# true 表示跳过对包含 KEY/TOKEN 的默认过滤
ignore_default_excludes = false
# 不区分大小写的 glob 排除
exclude = ["AWS_*", "AZURE_*"]
# 强制设置/覆盖的值
set = { CI = "1" }
# 如设置，仅保留匹配任一模式的变量
include_only = ["PATH", "HOME"]
```

| 字段                     | 类型                 | 默认     | 说明 |
| ------------------------ | -------------------- | -------- | ---- |
| `inherit`                | string               | `all`    | 环境继承模板：`all` 继承全部，`core` 仅核心变量（HOME、PATH、USER 等），`none` 表示空环境。 |
| `ignore_default_excludes`| boolean              | `false`  | 为 `false` 时，先移除名称包含 `KEY`、`SECRET` 或 `TOKEN`（不区分大小写）的变量，再应用其他规则。 |
| `exclude`                | array<string>        | `[]`     | 额外排除的 glob 模式，如 `"AWS_*"`、`"AZURE_*"`。 |
| `set`                    | table<string,string> | `{}`     | 显式覆盖/新增的键值，优先级最高。 |
| `include_only`           | array<string>        | `[]`     | 白名单模式列表；非空时仅保留匹配任一模式的变量（通常与 `inherit = "all"` 配合）。 |

模式为 **glob**（非正则），`*` 匹配任意长度，`?` 匹配单字符，`[A-Z]`/`[^0-9]` 等字符类可用，大小写不敏感。实现见 `core/src/config_types.rs` 中的 `EnvironmentVariablePattern`。

若想只保留少数变量可写：

```toml
[shell_environment_policy]
inherit = "none"
set = { PATH = "/usr/bin", MY_FLAG = "1" }
```

当禁用网络时，环境中还会添加 `CODEX_SANDBOX_NETWORK_DISABLED=1`（不可配置）。

## otel

Code 可输出描述每次运行的 [OpenTelemetry](https://opentelemetry.io/) **日志事件**（API 请求、流式响应、用户输入、工具审批等）。默认关闭，需在 `[otel]` 中选择导出方式：

```toml
[otel]
environment = "staging"     # 默认 "dev"
exporter = "none"            # 默认 none；设置 otlp-http 或 otlp-grpc 以发送事件
log_user_prompt = false       # 默认 false；除非显式开启，否则脱敏提示文本
```

所有事件会带上 `service.name = $ORIGINATOR`（默认 `code_cli_rs`）、CLI 版本与 `env` 属性，仅 `code_otel` crate 产生日志会被导出，并保留 `codex.*` 前缀以兼容现有看板。

### 事件目录

所有事件共享公共元数据：`event.timestamp`、`conversation.id`、`app.version`、`auth_mode`（如有）、`user.account_id`（如有）、`terminal.type`、`model`、`slug`。

启用 OTEL 后会产生：

- `codex.conversation_starts`：包含 `provider_name`、`reasoning_effort`（可选）、`reasoning_summary`、`context_window`、`max_output_tokens`、`auto_compact_token_limit`、`approval_policy`、`sandbox_policy`、`mcp_servers`、`active_profile`（可选）
- `codex.api_request`：`attempt`、`duration_ms`、`http.response.status_code`（可选）、`error.message`（失败时）
- `codex.sse_event`：`event.kind`、`duration_ms`、`error.message`（失败时）、`input_token_count`/`output_token_count`/`cached_token_count`（可选）/`reasoning_token_count`（可选）/`tool_token_count`
- `codex.user_prompt`：`prompt_length`、`prompt`（除非 `log_user_prompt=true` 否则脱敏）
- `codex.tool_decision`：`tool_name`、`call_id`、`decision`（`approved`、`approved_for_session`、`denied`、`abort`）、`source`（`config` 或 `user`）
- `codex.tool_result`：`tool_name`、`call_id`（可选）、`arguments`（可选）、`duration_ms`、`success`（"true"/"false"）、`output`

事件格式可能随迭代调整。

### 选择导出器

通过 `otel.exporter` 指定事件去向：

- `none`：保持插桩但不导出（默认）。
- `otlp-http`：以 OTLP/HTTP 发送日志，需指定 endpoint、protocol、headers：

  ```toml
  [otel]
  exporter = { otlp-http = {
    endpoint = "https://otel.example.com/v1/logs",
    protocol = "binary",
    headers = { "x-otlp-api-key" = "${OTLP_TOKEN}" }
  }}
  ```

- `otlp-grpc`：以 gRPC 发送 OTLP 日志，需要 endpoint 与可选 headers：

  ```toml
  [otel]
  exporter = { otlp-grpc = {
    endpoint = "https://otel.example.com:4317",
    headers = { "x-otlp-meta" = "abc123" }
  }}
  ```

两种导出器都可接受可选 `tls` 块以信任自定义 CA 或启用 mTLS。相对路径基于 `~/.code/`（也读取旧版 `~/.codex/`）：

```toml
[otel]
exporter = { otlp-http = {
  endpoint = "https://otel.example.com/v1/logs",
  protocol = "binary",
  headers = { "x-otlp-api-key" = "${OTLP_TOKEN}" },
  tls = {
    ca-certificate = "certs/otel-ca.pem",
    client-certificate = "/etc/code/certs/client.pem",
    client-private-key = "/etc/code/certs/client-key.pem",
  }
}}
```

导出器为 `none` 时不会写出数据；否则需自行运行或指向收集器。所有导出在后台批处理，关闭时会刷新。

源码构建时 OTEL crate 仍在 `otel` feature 后；官方预编译二进制默认启用。禁用时 telemetry 钩子为空操作，CLI 功能不受影响。

## notify

指定一个程序接收 Code 产生的事件通知。程序会收到 JSON 字符串参数，例如：

```json
{
  "type": "agent-turn-complete",
  "turn-id": "12345",
  "input-messages": ["Rename `foo` to `bar` and update the callsites."],
  "last-assistant-message": "Rename complete and verified `cargo build` succeeds."
}
```

目前仅支持 `"agent-turn-complete"` 类型。

示例：在 macOS 上使用 [terminal-notifier](https://github.com/julienXX/terminal-notifier) 发送桌面通知的 Python 脚本（略）。

将脚本用于通知，可在 `~/.code/config.toml`（兼容 `~/.codex/config.toml`）配置：

```toml
notify = ["python3", "/Users/mbolin/.code/notify.py"]
```

> [!NOTE]
> `notify` 适合自动化与集成：Code 每个事件调用外部程序并传递 JSON，与 TUI 独立。若只需 TUI 内的轻量通知，建议使用 `tui.notifications`（基于终端转义，无需外部程序）。两者可同时开启；`tui.notifications` 覆盖 TUI 内提醒（如审批提示），`notify` 适合系统级钩子或自定义提醒。目前 `notify` 仅发出 `agent-turn-complete`，而 `tui.notifications` 还支持 `approval-requested` 并可筛选。

## history

默认情况下，Code CLI 会将发送给模型的消息记录在 `$CODE_HOME/history.jsonl`（兼容读取 `$CODEX_HOME/history.jsonl`）。在 UNIX 下文件权限为 `o600`。

禁用记录：

```toml
[history]
persistence = "none"  # 默认 "save-all"
```

## Context timeline 预览

结构化的环境上下文时间线（baseline + deltas + browser snapshots）受环境变量 `CTX_UI` 控制。启动 Code 前设置 `CTX_UI=1` 体验预览。未开启时仍使用经典的 `== System Status ==` 负载。

## file_opener

指定用于模型输出中引用文件的超链接方案。设置后，模型输出中的文件引用会被重写为对应 URI，便于在终端中 Ctrl/Cmd+Click 打开。

可选值：

- `"vscode"`（默认）
- `"vscode-insiders"`
- `"windsurf"`
- `"cursor"`
- `"none"` 明确禁用

目前默认 `"vscode"`，但不会检查 VS Code 是否安装，未来可能调整默认值。

## hide_agent_reasoning

Code 会间歇输出“reasoning”事件显示模型内部思考。有些场景（如 CI 日志）可能不需要。将其设为 `true` 可在 TUI 与无头 `exec` 中同时隐藏：

```toml
hide_agent_reasoning = true   # 默认 false
```

## show_raw_agent_reasoning

在可用时展示模型的原始 chain-of-thought。

注意：

- 仅当模型/提供商实际返回原始推理内容时生效，许多模型不支持。
- 原始推理可能包含中间想法或敏感上下文，请在可接受的情况下开启。

```toml
show_raw_agent_reasoning = true  # 默认 false
```

## model_context_window

模型的上下文窗口大小（token）。对于不在已知列表的新模型，可用此项告知 Code 剩余上下文计算应使用的值。

## model_max_output_tokens

与 `model_context_window` 类似，但用于模型的最大输出 token 数。

## project_doc_max_bytes

从 `AGENTS.md` 读取的最大字节数，包含在会话首轮指令中，默认 32 KiB。

## project_doc_fallback_filenames

当某级目录缺少 `AGENTS.md` 时按顺序尝试的备用文件名。CLI 总是先查找 `AGENTS.md`，再按提供顺序尝试。便于单仓库逐步迁移到 `AGENTS.md`。

```toml
project_doc_fallback_filenames = ["CLAUDE.md", ".exampleagentrules.md"]
```

建议最终迁移到 AGENTS.md；其他文件名可能降低模型表现。

## tui

TUI 专属选项：

```toml
[tui]
# 当需要审批或轮次完成时发送桌面通知，默认 false
notifications = true
# 可选按类型过滤（"agent-turn-complete"、"approval-requested"）
notifications = [ "agent-turn-complete", "approval-requested" ]
# 仅审批通知
notifications = [ "approval-requested" ]
```

> [!NOTE]
> Code 通过终端转义发送桌面通知，并非所有终端都支持（macOS Terminal.app、VS Code 内置终端不支持；iTerm2、Ghostty、WezTerm 支持）。
>
> `tui.notifications` 仅作用于当前 TUI 会话。若需跨环境或与系统通知集成，请使用顶层 `notify` 运行外部程序。两者相互独立，可同时使用。

### Auto Drive 观察者

为长时间 Auto Drive 运行提供轻量观察线程。用顶层 `auto_drive_observer_cadence`（默认 `5`）配置节奏；每完成 n 个请求就审阅记录、发出遥测并在需要时给出修正提示。设为 `0` 关闭观察者。

```toml
# 每 3 个 Auto Drive 请求运行一次观察者
auto_drive_observer_cadence = 3
```

当观察者报告 `status = "failing"` 时，TUI 横幅会突出干预并更新待发提示（如有），同时记录后续指导。

## Project Hooks

使用 `[projects]` 按工作区路径限定设置。除 `trust_level`、`approval_policy`、`always_allow_commands` 外，还可挂载生命周期钩子，在特定事件自动运行命令。

```toml
[projects."/Users/me/src/my-app"]
trust_level = "trusted"

[[projects."/Users/me/src/my-app".hooks]]
name = "bootstrap"
event = "session.start"
run = ["./scripts/bootstrap.sh"]
timeout_ms = 60000

[[projects."/Users/me/src/my-app".hooks]]
event = "tool.after"
run = "npm run lint -- --changed"
```

支持的事件：

- `session.start`：会话配置完成后（每次启动一次）
- `session.end`：关闭前
- `tool.before`：每个 exec/工具命令运行前
- `tool.after`：每个 exec/工具命令完成后（无论退出码）
- `file.before_write`：应用 `apply_patch` 前
- `file.after_write`：`apply_patch` 完成并输出 diff 后

钩子在与会话相同的沙箱模式下运行，并在 TUI 中显示为独立 exec 单元。失败会作为后台事件提示，但不阻塞主任务。每次调用会收到环境变量，如 `CODE_HOOK_EVENT`、`CODE_HOOK_NAME`、`CODE_HOOK_INDEX`、`CODE_HOOK_CALL_ID`、`CODE_HOOK_PAYLOAD`（JSON，上下文描述）、`CODE_SESSION_CWD`，以及适用时的 `CODE_HOOK_SOURCE_CALL_ID`。钩子也可设置 `cwd`、额外 `env` 与 `timeout_ms`。

示例 `tool.after` 载荷：

```json
{
  "event": "tool.after",
  "call_id": "tool_12",
  "cwd": "/Users/me/src/my-app",
  "command": ["npm", "test"],
  "exit_code": 1,
  "duration_ms": 1832,
  "stdout": "…output truncated…",
  "stderr": "…",
  "timed_out": false
}
```
