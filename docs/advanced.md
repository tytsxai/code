## 高级

## 非交互 / CI 模式

在流水线中以无头方式运行 Every Code。GitHub Action 示例步骤：

```yaml
- name: Update changelog via Code
  run: |
    npm install -g @just-every/code
    export OPENAI_API_KEY="${{ secrets.OPENAI_KEY }}"
    code exec --full-auto "update CHANGELOG for next release"
```

### 恢复非交互会话

可以恢复之前的无头运行，继续同一对话上下文并附加到相同的 rollout 文件。

交互式 TUI 等价用法：

```shell
code resume             # 选择器
code resume --last      # 最近一次
code resume <SESSION_ID>
```

兼容性：

- 最新源码构建包含 `code exec resume`（见下方示例）。
- 如果 `code exec --help` 没有 `resume`，请升级到最新版本；该参数自 v0.5.0 起提供。

```shell
# 恢复最近会话并用新提示继续
code exec "ship a release draft changelog" resume --last

# 或通过 stdin 传递提示
# 注意：不要加末尾的 '-'，避免被解析为 SESSION_ID
echo "ship a release draft changelog" | code exec resume --last

# 或按 id（UUID）恢复指定会话
code exec resume 7f9f9a2e-1b3c-4c7a-9b0e-123456789abc "continue the task"
```

说明：

- 使用 `--last` 时，Code 选择最新记录的会话；如不存在则等同新建。
- 恢复会将新事件附加到现有会话文件，并保持相同的对话 id。

## 跟踪 / 详细日志

Code 由 Rust 编写，可通过 `RUST_LOG` 环境变量配置日志行为。

TUI 默认使用 `RUST_LOG=code_core=info,code_tui=info,code_browser=info,code_auto_drive_core=info`，日志写入 `~/.code/log/codex-tui.log`（仍会读取旧路径 `~/.codex/log/`）。可在另一个终端持续查看：

```
tail -F ~/.code/log/codex-tui.log
```

同时启用 CLI 的 `--debug` 参数时，请求/响应 JSON 会按照辅助模块划分到 `~/.code/debug_logs/` 下的子目录，例如：

- `auto/coordinator`
- `auto/observer/bootstrap`
- `auto/observer/cadence`
- `auto/observer/cross_check`
- `guided_terminal/agent_install_flow`
- `guided_terminal/upgrade_terminal_flow`
- `tui/rate_limit_refresh`
- `ui/theme_spinner`
- `ui/theme_builder`
- `cli/manual_prompt`

标签会转化为嵌套路径，因此自定义辅助模块会与时间戳文件并列出现。

相比之下，非交互模式（`code exec`）默认 `RUST_LOG=error`，日志直接打印，无需监听文件。

更多配置选项参见 Rust 文档 [`RUST_LOG`](https://docs.rs/env_logger/latest/env_logger/#enabling-logging)。

## Model Context Protocol (MCP)

可通过在 `~/.code/config.toml`（也会读取旧版 `~/.codex/config.toml`）中定义 [`mcp_servers`](./config.md#mcp_servers) 配置，让 Code CLI 使用 MCP 服务器。其设计与 Claude、Cursor 等工具的 `mcpServers` 类似，但采用 TOML 而非 JSON，格式略有差异，例如：

```toml
# 重要：顶层键为 `mcp_servers` 而不是 `mcpServers`。
[mcp_servers.server-name]
command = "npx"
args = ["-y", "mcp-server"]
env = { "API_KEY" = "value" }
```

## 将 Code 作为 MCP 服务器
> [!TIP]
> 虽仍属实验性质，Code CLI 也可通过 `code mcp` 作为 MCP *服务器* 运行。用 MCP 客户端（如 `npx @modelcontextprotocol/inspector code mcp`）启动并发送 `tools/list` 请求，会看到只有一个工具 `code`，它接受多种输入（包含兜底的 `config` map，可覆盖任意内容）。欢迎体验并通过 GitHub issue 提供反馈。
