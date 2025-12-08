## 非交互模式

使用 Every Code 的非交互模式自动化常见流程。

```shell
code exec "count the total number of lines of code in this project"
```

在非交互模式下，Code 不会请求命令或编辑审批。默认在 `read-only` 模式运行，无法修改文件或执行需要网络的命令。

使用 `code exec --full-auto` 允许文件修改。使用 `code exec --sandbox danger-full-access` 允许修改与联网命令。

### 默认输出模式

默认情况下，Code 将活动流式输出到 stderr，只把智能体的最终消息写到 stdout。这样更易将 `code exec` 管道到其他工具而无需额外过滤。

若要把 `code exec` 的输出写入文件，除了使用重定向 `>`，还可使用专用参数 `-o`/`--output-last-message` 指定输出文件。

### JSON 输出模式

`code exec` 支持 `--json` 模式，在智能体运行时将事件以 JSON Lines（JSONL）流式写到 stdout。

支持的事件类型：

- `thread.started` —— 线程启动或恢复时。
- `turn.started` —— 轮次开始时。一次轮次包含用户消息到助手回复之间的所有事件。
- `turn.completed` —— 轮次完成时；包含 token 用量。
- `turn.failed` —— 轮次失败时；包含错误详情。
- `item.started`/`item.updated`/`item.completed` —— 线程项新增/更新/完成时。

支持的 item 类型：

- `assistant_message` —— 助手消息。
- `reasoning` —— 助手思考摘要。
- `command_execution` —— 助手执行命令。
- `file_change` —— 助手修改文件。
- `mcp_tool_call` —— 助手调用 MCP 工具。
- `web_search` —— 助手执行网络搜索。

通常在轮次末尾会添加一个 `assistant_message`。

示例输出：

```jsonl
{"type":"thread.started","thread_id":"0199a213-81c0-7800-8aa1-bbab2a035a53"}
{"type":"turn.started"}
{"type":"item.completed","item":{"id":"item_0","item_type":"reasoning","text":"**Searching for README files**"}}
{"type":"item.started","item":{"id":"item_1","item_type":"command_execution","command":"bash -lc ls","aggregated_output":"","status":"in_progress"}}
{"type":"item.completed","item":{"id":"item_1","item_type":"command_execution","command":"bash -lc ls","aggregated_output":"AGENTS.md\nCHANGELOG.md\nREADME.md\ncode-rs\ncodex-rs\ncodex-cli\ndocs\nscripts\nsdk\n","exit_code":0,"status":"completed"}}
{"type":"item.completed","item":{"id":"item_2","item_type":"reasoning","text":"**Checking repository root for README**"}}
{"type":"item.completed","item":{"id":"item_3","item_type":"assistant_message","text":"Yep — there’s a `README.md` in the repository root."}}
{"type":"turn.completed","usage":{"input_tokens":24763,"cached_input_tokens":24448,"output_tokens":122}}
```

### 结构化输出

默认情况下，智能体以自然语言回复。使用 `--output-schema` 提供 JSON Schema 来定义期望的 JSON 输出。

JSON Schema 必须遵循[严格 Schema 规则](https://platform.openai.com/docs/guides/structured-outputs)。

示例 Schema：

```json
{
  "type": "object",
  "properties": {
    "project_name": { "type": "string" },
    "programming_languages": { "type": "array", "items": { "type": "string" } }
  },
  "required": ["project_name", "programming_languages"],
  "additionalProperties": false
}
```

```shell
code exec "Extract details of the project" --output-schema ~/schema.json
...

{"project_name":"Every Code CLI","programming_languages":["Rust","TypeScript","Shell"]}
```

将 `--output-schema` 与 `-o` 组合，可只输出最终 JSON。也可以给 `-o` 传文件路径以保存 JSON。

### Git 仓库要求

Code 需要在 Git 仓库中运行以避免破坏性更改。要禁用此检查，使用 `code exec --skip-git-repo-check`。

### 恢复非交互会话

使用 `code exec resume <SESSION_ID>` 或 `code exec resume --last` 恢复之前的非交互会话。会保留对话上下文，便于继续提问或下达新任务。

```shell
code exec "Review the change, look for use-after-free issues"
code exec resume --last "Fix use-after-free issues"
```

仅对话上下文会被保留；你仍需提供参数以自定义 Code 的行为。

```shell
code exec --model gpt-5.1-codex --json "Review the change, look for use-after-free issues"
code exec --model gpt-5.1 --json resume --last "Fix use-after-free issues"
```

## 认证

默认情况下，`code exec` 使用与 TUI 与 VSCode 扩展相同的认证方式。可通过环境变量 `CODEX_API_KEY` 覆盖 API Key。

```shell
CODEX_API_KEY=your-api-key-here code exec "Fix merge conflict"
```

注意：`CODEX_API_KEY` 仅在 `code exec` 中受支持。
